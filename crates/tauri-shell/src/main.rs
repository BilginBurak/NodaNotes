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

use state::AppState;
use tauri::Manager;

fn main() {
    let app_state = AppState::default();

    protocols::setup_protocols(tauri::Builder::default())
        .manage(app_state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::vault_commands::open_vault,
            commands::vault_commands::create_vault,
            commands::vault_commands::get_vault_info,
            commands::note_commands::create_note,
            commands::note_commands::get_note,
            commands::note_commands::get_note_metadata,
            commands::note_commands::update_note,
            commands::note_commands::rename_note,
            commands::note_commands::delete_note,
            commands::note_commands::list_notes,
            commands::note_commands::import_note,
            commands::note_commands::import_note_from_content,
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
            commands::folder_commands::list_folders,
            commands::folder_commands::create_folder,
            commands::folder_commands::delete_folder,
            commands::folder_commands::move_note,
            commands::folder_commands::move_folder,
            commands::folder_commands::rename_folder,
        ])
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

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let app_state = app_handle.state::<AppState>();
                let app_state_clone = app_state.inner().clone();
                tauri::async_runtime::block_on(async move {
                    app_state_clone.shutdown().await;
                });
            }
        });
}
