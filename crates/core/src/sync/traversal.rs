//! Traversal module to scan WebDAV remote tree

use crate::errors::NodaError;
use crate::sync::client::{WebDavClient, RemoteEntry};
use std::collections::VecDeque;

/// Helper to decode percent-encoded UTF-8 characters (e.g. `%20` -> space)
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut decoded = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(dec) = hex_to_byte(bytes[i + 1], bytes[i + 2]) {
                decoded.push(dec);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_to_byte(h: u8, l: u8) -> Option<u8> {
    let h_val = (h as char).to_digit(16)?;
    let l_val = (l as char).to_digit(16)?;
    Some((h_val << 4 | l_val) as u8)
}

/// Normalizes any WebDAV href or URL to a clean, absolute-style path component (e.g., `/dav/vault`)
pub fn normalize_path(path: &str) -> String {
    let decoded = percent_decode(path);
    
    // Extract path component from absolute URL if necessary
    let path_part = if decoded.starts_with("http://") || decoded.starts_with("https://") {
        if let Some(start_idx) = decoded.find("://") {
            if let Some(path_idx) = decoded[start_idx + 3..].find('/') {
                &decoded[start_idx + 3 + path_idx..]
            } else {
                "/"
            }
        } else {
            &decoded
        }
    } else {
        &decoded
    };

    // Clean duplicate/trailing slashes and ensure a single leading slash
    let mut cleaned = String::new();
    for segment in path_part.split('/') {
        if !segment.is_empty() {
            cleaned.push('/');
            cleaned.push_str(segment);
        }
    }

    if cleaned.is_empty() {
        "/".to_string()
    } else {
        cleaned
    }
}

/// Checks if two paths refer to the same WebDAV resource
fn is_same_path(requested_abs: &str, returned_href: &str) -> bool {
    let req = normalize_path(requested_abs);
    let ret = normalize_path(returned_href);
    req == ret
}

/// Recursively lists a remote WebDAV directory using recursive `Depth: 1` walking.
/// Compatibility layer for InfiniCLOUD and similar restrictive WebDAV servers.
pub async fn list_remote_tree(
    client: &WebDavClient,
    root_path: &str,
) -> Result<Vec<RemoteEntry>, NodaError> {
    let mut all_entries = Vec::new();
    let mut queue = VecDeque::new();
    
    let base_url_path = normalize_path(&client.base_url);
    
    // Ensure root path starts with /
    let mut root = root_path.to_string();
    if !root.starts_with('/') {
        root = format!("/{}", root);
    }
    
    queue.push_back(root);

    while let Some(current_path) = queue.pop_front() {
        let entries = client.propfind(&current_path, 1).await?;
        let requested_absolute_url = client.build_url(&current_path);

        for mut entry in entries.into_iter() {
            let is_self = is_same_path(&requested_absolute_url, &entry.href);
            if is_self {
                continue;
            }

            // Standardize entry's href to be a clean path relative to the WebDAV base URL
            let normalized_href = normalize_path(&entry.href);
            let relative_path = if normalized_href.starts_with(&base_url_path) {
                let remainder = &normalized_href[base_url_path.len()..];
                if remainder.starts_with('/') {
                    remainder.to_string()
                } else {
                    format!("/{}", remainder)
                }
            } else {
                normalized_href.clone()
            };

            entry.href = relative_path.clone();

            // Ignore anything inside the .noda system folder EXCEPT attachments and history
            let normalized_relative = relative_path.trim_start_matches('/');
            
            // Exclude hidden files or folders, EXCEPT allowing .noda itself
            let mut parts = normalized_relative.split('/');
            let has_hidden = parts.any(|part| part.starts_with('.') && part != ".noda");
            if has_hidden {
                continue;
            }

            if normalized_relative.split('/').any(|s| s == ".noda") {
                let is_allowed = normalized_relative == ".noda"
                    || normalized_relative.starts_with(".noda/attachments")
                    || normalized_relative.starts_with(".noda/history");
                if !is_allowed {
                    continue;
                }
            }

            if entry.is_collection {
                queue.push_back(relative_path);
            }
            
            all_entries.push(entry);
        }
    }

    Ok(all_entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::client::WebDavClient;
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("hello%2Fworld"), "hello/world");
        assert_eq!(percent_decode("no_percent"), "no_percent");
        assert_eq!(percent_decode("%invalid"), "%invalid");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("https://example.com/dav/vault/"), "/dav/vault");
        assert_eq!(normalize_path("/dav/vault/"), "/dav/vault");
        assert_eq!(normalize_path("///dav//vault//"), "/dav/vault");
        assert_eq!(normalize_path("https://example.com/dav/vault/My%20Note.md"), "/dav/vault/My Note.md");
        assert_eq!(normalize_path("https://example.com/"), "/");
        assert_eq!(normalize_path(""), "/");
    }

    #[tokio::test]
    async fn test_list_remote_tree_recursive() {
        // Start a local mock WebDAV HTTP server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server_url = format!("http://{}", addr);

        // Spawn a background task to handle mock WebDAV requests
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0; 4096];
                    if let Ok(n) = socket.read(&mut buf).await {
                        if n == 0 { return; }
                        let req_str = String::from_utf8_lossy(&buf[..n]);
                        
                        if req_str.starts_with("PROPFIND") {
                            if req_str.contains("/dav/vault/folder1") {
                                // Return response 2
                                let body = concat!(
                                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                                    "<d:multistatus xmlns:d=\"DAV:\">\n",
                                    "  <d:response>\n",
                                    "    <d:href>/dav/vault/folder1/</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop>\n",
                                    "        <d:resourcetype><d:collection/></d:resourcetype>\n",
                                    "      </d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "  <d:response>\n",
                                    "    <d:href>/dav/vault/folder1/note2.md</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop>\n",
                                    "        <d:resourcetype/>\n",
                                    "        <d:getcontentlength>456</d:getcontentlength>\n",
                                    "        <d:getlastmodified>Wed, 20 May 2026 03:05:00 GMT</d:getlastmodified>\n",
                                    "        <d:getetag>\"def\"</d:getetag>\n",
                                    "      </d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "</d:multistatus>"
                                );
                                let response = format!(
                                    "HTTP/1.1 207 Multi-Status\r\nContent-Type: text/xml; charset=\"utf-8\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                            } else if req_str.contains("/dav/vault") {
                                // Return response 1
                                let body = concat!(
                                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                                    "<d:multistatus xmlns:d=\"DAV:\">\n",
                                    "  <d:response>\n",
                                    "    <d:href>/dav/vault/</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop>\n",
                                    "        <d:resourcetype><d:collection/></d:resourcetype>\n",
                                    "      </d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "  <d:response>\n",
                                    "    <d:href>/dav/vault/folder1/</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop>\n",
                                    "        <d:resourcetype><d:collection/></d:resourcetype>\n",
                                    "      </d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "  <d:response>\n",
                                    "    <d:href>/dav/vault/note1.md</d:href>\n",
                                    "    <d:propstat>\n",
                                    "      <d:prop>\n",
                                    "        <d:resourcetype/>\n",
                                    "        <d:getcontentlength>123</d:getcontentlength>\n",
                                    "        <d:getlastmodified>Wed, 20 May 2026 03:00:00 GMT</d:getlastmodified>\n",
                                    "        <d:getetag>\"abc\"</d:getetag>\n",
                                    "      </d:prop>\n",
                                    "      <d:status>HTTP/1.1 200 OK</d:status>\n",
                                    "    </d:propstat>\n",
                                    "  </d:response>\n",
                                    "</d:multistatus>"
                                );
                                let response = format!(
                                    "HTTP/1.1 207 Multi-Status\r\nContent-Type: text/xml; charset=\"utf-8\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    body.len(),
                                    body
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                            }
                        }
                    }
                });
            }
        });

        // Instantiate client pointing to our mock server
        // base_url path has suffix "/dav"
        let base_url = format!("{}/dav", server_url);
        let client = WebDavClient::new(&base_url, "user", "pass").unwrap();

        let entries = list_remote_tree(&client, "/vault").await.unwrap();

        // There should be 3 entries in total:
        // 1. /vault/folder1/ (collection)
        // 2. /vault/note1.md (file)
        // 3. /vault/folder1/note2.md (file)
        assert_eq!(entries.len(), 3);

        let folder1 = entries.iter().find(|e| e.is_collection).unwrap();
        assert_eq!(folder1.href, "/vault/folder1");

        let files: Vec<&RemoteEntry> = entries.iter().filter(|e| !e.is_collection).collect();
        assert_eq!(files.len(), 2);
        
        let note1 = files.iter().find(|e| e.href.ends_with("note1.md")).unwrap();
        assert_eq!(note1.href, "/vault/note1.md");
        assert_eq!(note1.size, Some(123));

        let note2 = files.iter().find(|e| e.href.ends_with("note2.md")).unwrap();
        assert_eq!(note2.href, "/vault/folder1/note2.md");
        assert_eq!(note2.size, Some(456));
    }
}
