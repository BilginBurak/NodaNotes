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
  title: string;
  body: string;
  frontmatter: Frontmatter;
  file_path: string;
}

export interface NoteListItemDto {
  id: string;
  title: string;
  file_path: string;
  tags: string[];
  created: string;
  updated: string;
}

export interface VaultInfoDto {
  path: string;
  name: string;
  note_count: number;
}

export interface Snapshot {
  id: string;
  note_id: string;
  timestamp: string;
  file_path: string;
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
  filename: string;
  local_path: string;
  remote_path: string;
}
