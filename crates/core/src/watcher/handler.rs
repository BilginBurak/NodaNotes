//! Event classification for raw `notify` events.

use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::Event;
use std::path::PathBuf;

/// Classified watcher event.
#[derive(Debug)]
pub enum ClassifiedEvent {
    Modified(PathBuf),
    Created(PathBuf),
    Deleted(PathBuf),
    RenamedFrom(PathBuf),
    RenamedTo(PathBuf),
    Ignored,
}

/// Classify a raw `notify::Event` into a `ClassifiedEvent`.
///
/// Only `.md` files are considered; all others are `Ignored`.
pub fn classify(event: &Event) -> Vec<ClassifiedEvent> {
    let mut results = Vec::new();

    for path in &event.paths {
        // Only track markdown files.
        let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
        if !is_md {
            continue;
        }

        let classified = match &event.kind {
            EventKind::Create(_) => ClassifiedEvent::Created(path.clone()),
            EventKind::Modify(ModifyKind::Data(_)) => ClassifiedEvent::Modified(path.clone()),
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                ClassifiedEvent::RenamedFrom(path.clone())
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                ClassifiedEvent::RenamedTo(path.clone())
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                // Some backends emit both paths in a single event.
                if event.paths.len() == 2 {
                    ClassifiedEvent::RenamedFrom(path.clone())
                } else {
                    ClassifiedEvent::Modified(path.clone())
                }
            }
            EventKind::Remove(_) => ClassifiedEvent::Deleted(path.clone()),
            _ => ClassifiedEvent::Ignored,
        };

        results.push(classified);
    }

    results
}
