//! Database schema definitions

pub const INIT_SCHEMA: &str = r#"
-- Main notes table (cache, not source of truth)
CREATE TABLE IF NOT EXISTS notes (
    rowid     INTEGER PRIMARY KEY,
    id        TEXT UNIQUE NOT NULL,          -- UUID
    parent_id TEXT,                          -- UUID optional
    title     TEXT NOT NULL,
    body      TEXT NOT NULL,
    color     TEXT,                          -- optional
    pinned    BOOLEAN NOT NULL DEFAULT 0,
    status    TEXT NOT NULL DEFAULT 'active',
    created   TEXT NOT NULL,                 -- ISO 8601
    updated   TEXT NOT NULL,                 -- ISO 8601
    file_path TEXT NOT NULL                  -- Relative path from vault root
);

-- Full-text search index (external content table)
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    title,
    body,
    content=notes,
    content_rowid=rowid,
    tokenize='unicode61 remove_diacritics 2'
);

-- Triggers to keep FTS5 in sync with notes table
CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, title, body)
    VALUES (new.rowid, new.title, new.body);
END;

CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body)
    VALUES ('delete', old.rowid, old.title, old.body);
END;

CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body)
    VALUES ('delete', old.rowid, old.title, old.body);
    INSERT INTO notes_fts(rowid, title, body)
    VALUES (new.rowid, new.title, new.body);
END;

-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_notes_title ON notes(title);
CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated);
CREATE INDEX IF NOT EXISTS idx_notes_status ON notes(status);

-- 1. Tüm benzersiz taglerin tutulduğu ana sözlük tablosu
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL
);

-- 2. Notlar ile Tagleri bağlayan Çoktan-Çoğa (Many-to-Many) ilişki tablosu
CREATE TABLE IF NOT EXISTS note_tags (
    note_id TEXT NOT NULL,
    tag_id INTEGER NOT NULL,
    source TEXT NOT NULL, -- 'yaml' veya 'inline' (Nereden geldiğini bilmek için kritik!)
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY(note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

--// Sync file states (relational tracking)
CREATE TABLE IF NOT EXISTS sync_file_states (
    path TEXT PRIMARY KEY,
    etag TEXT,
    last_modified TEXT,
    size INTEGER NOT NULL,
    local_updated_at TEXT,
    hash TEXT NOT NULL,
    is_dirty INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER DEFAULT 0,
    sync_error TEXT
);
CREATE INDEX IF NOT EXISTS idx_sync_file_states_hash ON sync_file_states(hash);

-- Peer file states (Git-like snapshot manifests)
CREATE TABLE IF NOT EXISTS peer_file_states (
    device_name TEXT,
    path TEXT,
    hash TEXT,
    PRIMARY KEY (device_name, path)
);
CREATE INDEX IF NOT EXISTS idx_peer_file_states_device ON peer_file_states(device_name);

-- Sync device states
CREATE TABLE IF NOT EXISTS sync_device_states (
    device_name TEXT PRIMARY KEY,
    last_known_etag TEXT,
    last_known_modified TEXT
);

-- Trusted devices for remote authorization
CREATE TABLE IF NOT EXISTS trusted_devices (
    id TEXT PRIMARY KEY,
    device_name TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    status TEXT NOT NULL,
    device_token TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_trusted_devices_token ON trusted_devices(device_token);
"#;

