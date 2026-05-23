//! WebDAV client for syncing

use crate::errors::NodaError;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoteEntry {
    pub href: String,
    pub is_collection: bool,
    pub last_modified: Option<String>,
    pub size: Option<u64>,
    pub etag: Option<String>,
}

pub struct WebDavClient {
    client: Client,
    pub(crate) base_url: String,
    username: String,
    password: Option<String>,
}

impl WebDavClient {
    pub fn new(url: &str, username: &str, password: &str) -> Result<Self, NodaError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        Ok(Self {
            client,
            base_url: url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: Some(password.to_string()),
        })
    }

    pub(crate) fn build_url(&self, path: &str) -> String {
        let trimmed_path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, trimmed_path)
    }

    pub async fn propfind(&self, path: &str, depth: u8) -> Result<Vec<RemoteEntry>, NodaError> {
        let url = self.build_url(path);
        
        let response = self.client.request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, self.password.as_ref())
            .header("Depth", depth.to_string())
            .send()
            .await
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NodaError::Sync(format!("PROPFIND failed: {}", response.status())));
        }

        let xml = response.text().await.map_err(|e| NodaError::Sync(e.to_string()))?;
        self.parse_propfind(&xml)
    }

    fn parse_propfind(&self, xml: &str) -> Result<Vec<RemoteEntry>, NodaError> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut entries = Vec::new();

        let mut current_href = String::new();
        let mut is_col = false;
        let mut size = None;
        let mut etag = None;
        let mut last_mod = None;
        
        let mut in_href = false;
        let mut in_getcontentlength = false;
        let mut in_getetag = false;
        let mut in_getlastmodified = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.name().into_inner();
                    let local_name = Self::strip_prefix(name);
                    
                    match local_name {
                        b"response" => {
                            current_href.clear();
                            is_col = false;
                            size = None;
                            etag = None;
                            last_mod = None;
                        }
                        b"href" => in_href = true,
                        b"collection" => is_col = true,
                        b"getcontentlength" => in_getcontentlength = true,
                        b"getetag" => in_getetag = true,
                        b"getlastmodified" => in_getlastmodified = true,
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let name = e.name().into_inner();
                    let local_name = Self::strip_prefix(name);
                    if local_name == b"collection" {
                        is_col = true;
                    }
                }
                Ok(Event::Text(e)) => {
                    let txt = e.unescape().unwrap_or_default().trim().to_string();
                    if in_href {
                        current_href = txt;
                    } else if in_getcontentlength {
                        size = txt.parse::<u64>().ok();
                    } else if in_getetag {
                        etag = Some(txt);
                    } else if in_getlastmodified {
                        last_mod = Some(txt);
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name().into_inner();
                    let local_name = Self::strip_prefix(name);

                    match local_name {
                        b"response" => {
                            entries.push(RemoteEntry {
                                href: current_href.clone(),
                                is_collection: is_col,
                                last_modified: last_mod.take(),
                                size: size.take(),
                                etag: etag.take(),
                            });
                        }
                        b"href" => in_href = false,
                        b"getcontentlength" => in_getcontentlength = false,
                        b"getetag" => in_getetag = false,
                        b"getlastmodified" => in_getlastmodified = false,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(NodaError::Sync(format!("XML parse error: {}", e))),
                _ => {}
            }
            buf.clear();
        }

        Ok(entries)
    }

    fn strip_prefix(name: &[u8]) -> &[u8] {
        if let Some(idx) = name.iter().position(|&b| b == b':') {
            &name[idx + 1..]
        } else {
            name
        }
    }

    pub async fn get(&self, path: &str) -> Result<Vec<u8>, NodaError> {
        let url = self.build_url(path);
        let response = self.client.get(&url)
            .basic_auth(&self.username, self.password.as_ref())
            .send()
            .await
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(NodaError::NotFound(format!("File not found on remote: {}", path)));
        }

        if !response.status().is_success() {
            return Err(NodaError::Sync(format!("GET failed: {}", response.status())));
        }

        let bytes = response.bytes().await.map_err(|e| NodaError::Sync(e.to_string()))?;
        Ok(bytes.to_vec())
    }

    pub async fn put(&self, path: &str, data: Vec<u8>) -> Result<(), NodaError> {
        let url = self.build_url(path);
        let response = self.client.put(&url)
            .basic_auth(&self.username, self.password.as_ref())
            .body(data)
            .send()
            .await
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::CREATED {
            return Err(NodaError::Sync(format!("PUT failed: {}", response.status())));
        }

        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<(), NodaError> {
        let url = self.build_url(path);
        let response = self.client.delete(&url)
            .basic_auth(&self.username, self.password.as_ref())
            .send()
            .await
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(NodaError::Sync(format!("DELETE failed: {}", response.status())));
        }

        Ok(())
    }

    pub async fn move_file(&self, from: &str, to: &str) -> Result<(), NodaError> {
        let url = self.build_url(from);
        let destination = self.build_url(to);
        
        let response = self.client.request(Method::from_bytes(b"MOVE").unwrap(), &url)
            .basic_auth(&self.username, self.password.as_ref())
            .header("Destination", destination)
            .header("Overwrite", "T")
            .send()
            .await
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NodaError::Sync(format!("MOVE failed: {}", response.status())));
        }

        Ok(())
    }

    pub async fn mkcol(&self, path: &str) -> Result<(), NodaError> {
        let url = self.build_url(path);
        let response = self.client.request(Method::from_bytes(b"MKCOL").unwrap(), &url)
            .basic_auth(&self.username, self.password.as_ref())
            .send()
            .await
            .map_err(|e| NodaError::Sync(e.to_string()))?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(NodaError::Sync(format!("MKCOL failed: {}", response.status())));
        }

        Ok(())
    }
}
