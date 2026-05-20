use tauri::Manager;
use tauri::http::{Response, header::CONTENT_TYPE, header::ACCESS_CONTROL_ALLOW_ORIGIN};
use noda_core::protocol::serve_file;
use crate::state::AppState;

pub fn setup_protocols(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_asynchronous_uri_scheme_protocol("noda", move |app, request, responder| {
        let app_handle = app.app_handle().clone();
        let uri = request.uri().to_string();
        
        tauri::async_runtime::spawn(async move {
            let vault_path = {
                let state = app_handle.state::<AppState>();
                let locked = state.vault_path.read();
                if let Some(path) = &*locked {
                    path.clone()
                } else {
                    let res = Response::builder()
                        .status(500)
                        .body("Vault path not initialized".to_string().into_bytes())
                        .unwrap();
                    responder.respond(res);
                    return;
                }
            };
            
            match serve_file(vault_path, &uri).await {
                Ok((bytes, mime_type)) => {
                    let res = Response::builder()
                        .status(200)
                        .header(CONTENT_TYPE, mime_type)
                        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(bytes)
                        .unwrap();
                    responder.respond(res);
                }
                Err(e) => {
                    let status = match e {
                        noda_core::errors::NodaError::PathTraversal(_) => 403,
                        noda_core::errors::NodaError::NotFound(_) => 404,
                        _ => 500,
                    };
                    let res = Response::builder()
                        .status(status)
                        .body(e.to_string().into_bytes())
                        .unwrap();
                    responder.respond(res);
                }
            }
        });
    })
}
