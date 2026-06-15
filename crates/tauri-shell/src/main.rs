// Main entry point for the Noda Tauri shell application.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod commands;
pub mod events;
pub mod protocols;
pub mod state;
pub mod window;
pub mod server;

use state::AppState;
use tauri::Manager;

use std::sync::atomic::{AtomicBool, Ordering};

static CLOSE_REQUESTED_BY_USER: AtomicBool = AtomicBool::new(false);

fn main() {
    let app_state = AppState::default();

    protocols::setup_protocols(tauri::Builder::default())
        .manage(app_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::settings_commands::get_daemon_token,
            commands::vault_commands::open_vault,
            commands::vault_commands::create_vault,
            commands::vault_commands::get_vault_info,
            commands::vault_commands::reveal_in_file_manager,
            commands::note_commands::create_note,
            commands::note_commands::get_note,
            commands::note_commands::get_note_metadata,
            commands::note_commands::update_note,
            commands::note_commands::rename_note,
            commands::note_commands::delete_note,
            commands::note_commands::list_notes,
            commands::note_commands::import_note,
            commands::note_commands::import_note_from_content,
            commands::note_commands::list_tags_with_counts,
            commands::note_commands::trigger_daily_note,
            commands::note_commands::toggle_task_status,
            commands::search_commands::search_notes,
            commands::sync_commands::start_sync,
            commands::sync_commands::stop_sync,
            commands::sync_commands::sync_now,
            commands::sync_commands::get_sync_status,
            commands::sync_commands::update_sync_config,
            commands::sync_commands::validate_sync_config,
            commands::sync_commands::get_sync_config,
            commands::history_commands::list_snapshots,
            commands::history_commands::restore_snapshot,
            commands::history_commands::compare_snapshot,
            commands::history_commands::delete_snapshot,
            commands::trash_commands::list_trash,
            commands::trash_commands::trash_note,
            commands::trash_commands::restore_from_trash,
            commands::trash_commands::permanent_delete,
            commands::trash_commands::get_trash_note,
            commands::sync_commands::list_conflicts,
            commands::sync_commands::get_conflict_note,
            commands::sync_commands::resolve_conflict_keep_local,
            commands::sync_commands::resolve_conflict_keep_remote,
            commands::attachment_commands::add_attachment,
            commands::attachment_commands::add_attachment_bytes,
            commands::attachment_commands::list_attachments,
            commands::attachment_commands::delete_attachment,
            commands::attachment_commands::list_attachments_with_metadata,
            commands::settings_commands::get_settings,
            commands::settings_commands::save_settings,
            commands::maintenance_commands::rebuild_database_cache,
            commands::maintenance_commands::vacuum_database_cache,
            commands::maintenance_commands::get_orphaned_attachments,
            commands::maintenance_commands::delete_orphaned_attachments,
            commands::maintenance_commands::clear_sync_queue,
            commands::maintenance_commands::clear_sync_cache,
            commands::maintenance_commands::get_duplicate_notes,
            commands::maintenance_commands::delete_duplicate_note_file,
            commands::maintenance_commands::get_orphaned_remnants,
            commands::maintenance_commands::delete_orphaned_remnants,
            commands::maintenance_commands::delete_orphaned_file,
            commands::folder_commands::list_folders,
            commands::folder_commands::create_folder,
            commands::folder_commands::delete_folder,
            commands::folder_commands::move_note,
            commands::folder_commands::move_folder,
            commands::folder_commands::rename_folder,
            commands::device_commands::get_trusted_devices,
            commands::device_commands::approve_device,
            commands::device_commands::revoke_device,
            commands::device_commands::regenerate_daemon_token,
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                #[cfg(target_os = "macos")]
                {
                    CLOSE_REQUESTED_BY_USER.store(true, Ordering::Relaxed);
                }
            }
        })
        .setup(|app| {
            // Configure main window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window::setup_window(&window);
            }

            let app_handle = app.handle().clone();
            let app_state = app.state::<AppState>().inner().clone();

            // Load last opened vault synchronously during setup to avoid race condition with Svelte onMount
            tauri::async_runtime::block_on(async move {
                if let Ok(Some(path)) = noda_core::vault::persistence::load_last_vault_path().await {
                    if let Err(e) = app_state.init_vault(&path, app_handle).await {
                        tracing::error!("Failed to auto-open last vault: {}", e);
                    }
                }
            });

            // Start Unified Axum Localhost Integrated Server
            let app_state_server = app.state::<AppState>().inner().clone();
            let app_handle_clone = app.handle().clone();
            let token = app_state_server.daemon_token.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::server::run_server(app_handle_clone, app_state_server, token).await {
                    tracing::error!("Failed to start unified daemon server: {}", e);
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            #[cfg(target_os = "macos")]
            match &event {
                tauri::RunEvent::ExitRequested { api, .. } => {
                    if CLOSE_REQUESTED_BY_USER.swap(false, Ordering::Relaxed) {
                        api.prevent_exit();
                    }
                }
                tauri::RunEvent::Reopen { .. } => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    } else {
                        // Recreate the window
                        let builder = tauri::WebviewWindowBuilder::new(
                            app_handle,
                            "main",
                            tauri::WebviewUrl::default(),
                        )
                        .title("Noda")
                        .inner_size(1200.0, 800.0)
                        .min_inner_size(800.0, 600.0)
                        .transparent(true)
                        .hidden_title(true)
                        .disable_drag_drop_handler()
                        .title_bar_style(tauri::TitleBarStyle::Overlay);

                        if let Ok(window) = builder.build() {
                            let _ = window::setup_window(&window);
                        }
                    }
                }
                _ => {}
            }

            if let tauri::RunEvent::Exit = event {
                let app_state = app_handle.state::<AppState>();
                let app_state_clone = app_state.inner().clone();
                tauri::async_runtime::block_on(async move {
                    app_state_clone.shutdown().await;
                });
            }
        });
}
