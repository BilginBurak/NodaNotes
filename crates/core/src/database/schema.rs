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
    tags      TEXT NOT NULL DEFAULT '[]',    -- JSON array
    status    TEXT NOT NULL DEFAULT 'active',
    created   TEXT NOT NULL,                 -- ISO 8601
    updated   TEXT NOT NULL,                 -- ISO 8601
    file_path TEXT NOT NULL,                 -- Relative path from vault root
    file_hash TEXT                           -- For change detection
);

-- Full-text search index (external content table)
CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    title,
    body,
    tags,
    content=notes,
    content_rowid=rowid,
    tokenize='unicode61 remove_diacritics 2'
);

-- Triggers to keep FTS5 in sync with notes table
CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
    INSERT INTO notes_fts(rowid, title, body, tags)
    VALUES (new.rowid, new.title, new.body, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body, tags)
    VALUES ('delete', old.rowid, old.title, old.body, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
    INSERT INTO notes_fts(notes_fts, rowid, title, body, tags)
    VALUES ('delete', old.rowid, old.title, old.body, old.tags);
    INSERT INTO notes_fts(rowid, title, body, tags)
    VALUES (new.rowid, new.title, new.body, new.tags);
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

-- 3. Geçmiş (snapshot) kayıtlarının tutulduğu tablo
CREATE TABLE IF NOT EXISTS history_snapshots (
    note_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    reason TEXT NOT NULL,
    file_path TEXT NOT NULL,
    PRIMARY KEY (note_id, timestamp)
);
"#;
