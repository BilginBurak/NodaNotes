export interface Frontmatter {
  id: string;
  title: string;
  created: string;
  updated: string;
  tags: string[];
  status?: string;
  [key: string]: any;
}

export interface NoteDto {
  id: string;
  parent_id?: string | null;
  title: string;
  body: string;
  color?: string | null;
  pinned: boolean;
  tags: string[];
  inline_tags: string[];
  created_at: string;
  updated_at: string;
  file_path: string;
  frontmatter?: Frontmatter; // Keep optional for backwards compatibility
}

export interface NoteMetadataDto {
  id: string;
  title: string;
  file_name: string;
  relative_path: string;
  absolute_path: string;
  created_at: string;
  updated_at: string;
  tags: string[];
  history_count: number;
  last_upload_time?: string | null;
  file_size_bytes: number;
  word_count: number;
  char_count: number;
}

export interface NoteListItemDto {
  id: string;
  parent_id?: string | null;
  title: string;
  color?: string | null;
  pinned: boolean;
  tags: string[];
  inline_tags: string[];
  updated_at: string;
  file_path: string;
}

export interface VaultInfoDto {
  path: string;
  name: string;
  note_count: number;
}

export interface Snapshot {
  note_id: string;
  timestamp: string;
  absolute_path: string;
  reason: string;
}

export interface DiffChunk {
  tag: 'Equal' | 'Insert' | 'Delete' | 'Separator';
  text: string;
}

export interface SnapshotDiffDto {
  note_id: string;
  timestamp: string;
  body_chunks: DiffChunk[];
}

export interface TrashEntry {
  id: string;
  title: string;
  original_path: string;
  deleted_at: string;
}

export interface AppError {
  code: string;
  message: string;
}

export type SyncStatusType = 'Idle' | 'Syncing' | 'Error';

export interface SyncStatus {
  status: SyncStatusType;
  last_sync_time?: string;
  error_message?: string;
}

export interface ConflictEntry {
  id: string;
  title: string;
  file_path: string;
  archived_path: string;
  detected_at: string;
}

export type SpecialFolder = '__trash__' | '__conflicts__';

/**
 * SyncReport — Rust backend'deki SyncReport struct'ının frontend karşılığı
 * Alanlar: crates/core/src/sync/engine.rs — SyncReport
 */
export interface SyncReport {
  uploads: number;
  downloads: number;
  deletes_local: number;
  deletes_remote: number;
  conflicts: number;
  uploaded_files: string[];
  downloaded_files: string[];
  deleted_local_files: string[];
  deleted_remote_files: string[];
  conflict_files: string[];
  /** Toplam işlem sayısı — hesaplanmış */
  total?: number;
  /** Sync tamamlanma zamanı — frontend tarafından eklenir */
  completed_at?: string;
}

export interface SyncConfig {
  webdav_url: string;
  webdav_username: string;
  webdav_password?: string;
  interval_secs: number;
}

export interface AppearanceSettings {
  theme: string; // "light", "dark", "auto"
  accent_color: string;
}

export interface EditorSettings {
  font_size: number;
  typography: string; // "sans", "serif", "mono"
  show_word_count: boolean;
  auto_save_delay_ms: number;
  default_daily_template?: string | null;
}

export interface TagWithCountDto {
  name: string;
  count: number;
}

export interface HistorySettings {
  retention_days: number;
  max_snapshots_per_note: number;
  empty_trash_after_days: number;
  snapshot_interval_mins: number;
}

export interface AppConfig {
  appearance: AppearanceSettings;
  editor: EditorSettings;
  sync: SyncConfig;
  history: HistorySettings;
}

export interface TreeNode {
  name: string;
  type: 'folder' | 'note';
  relPath: string; // relative path from root, e.g. "work/ideas"
  id?: string; // note ID if type is 'note'
  children: TreeNode[];
  isOpen?: boolean;
}

export interface AttachmentInfoDto {
  name: string;
  modified_at: number; // Epoch millis
  size: number; // Bytes
}


