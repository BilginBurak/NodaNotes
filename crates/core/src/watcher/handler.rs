//! Handler to convert notify events to standard Vault events

use notify::Event;
use notify::EventKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum VaultEvent {
    Create(PathBuf),
    Modify(PathBuf),
    Delete(PathBuf),
    Rename(PathBuf, PathBuf),
}

pub fn is_valid_file(path: &Path, vault_root: &Path) -> bool {
    // Ignore anything inside .noda
    if let Ok(rel_path) = path.strip_prefix(vault_root) {
        if rel_path.components().any(|c| c.as_os_str() == ".noda") {
            return false;
        }
    } else {
        return false;
    }

    // Only .md files
    if path.extension().map_or(true, |ext| ext != "md") {
        return false;
    }

    true
}

pub fn process_event(event: Event, vault_root: &Path) -> Vec<VaultEvent> {
    let mut vault_events = Vec::new();
    
    match event.kind {
        EventKind::Create(_) => {
            for path in event.paths {
                if is_valid_file(&path, vault_root) {
                    vault_events.push(VaultEvent::Create(path));
                }
            }
        }
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
            // Rename is tricky because notify sometimes gives two paths, sometimes one path per event.
            if event.paths.len() == 2 {
                let from = &event.paths[0];
                let to = &event.paths[1];
                let from_valid = is_valid_file(from, vault_root);
                let to_valid = is_valid_file(to, vault_root);
                
                if from_valid && to_valid {
                    vault_events.push(VaultEvent::Rename(from.clone(), to.clone()));
                } else if from_valid && !to_valid {
                    vault_events.push(VaultEvent::Delete(from.clone()));
                } else if !from_valid && to_valid {
                    vault_events.push(VaultEvent::Create(to.clone()));
                }
            } else if event.paths.len() == 1 {
                let path = &event.paths[0];
                if is_valid_file(path, vault_root) {
                    vault_events.push(VaultEvent::Modify(path.clone()));
                }
            }
        }
        EventKind::Modify(_) => {
            for path in event.paths {
                if is_valid_file(&path, vault_root) {
                    vault_events.push(VaultEvent::Modify(path));
                }
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                if is_valid_file(&path, vault_root) {
                    vault_events.push(VaultEvent::Delete(path));
                }
            }
        }
        _ => {}
    }
    
    vault_events
}
