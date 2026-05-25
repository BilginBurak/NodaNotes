import { invoke } from '@tauri-apps/api/core';
import type {
  NoteDto,
  NoteMetadataDto,
  NoteListItemDto,
  VaultInfoDto,
  SyncStatus,
  SyncReport,
  AppError,
  SyncConfig,
  Snapshot,
  TrashEntry,
  SnapshotDiffDto,
  AppConfig,
  AttachmentInfoDto,
} from '../types';

async function call<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err: any) {
    // If err matches AppError shape, re-throw
    if (err && typeof err === 'object' && 'code' in err && 'message' in err) {
      throw err as AppError;
    }
    throw {
      code: 'UNKNOWN_ERROR',
      message: err?.toString() || 'An unexpected error occurred'
    } as AppError;
  }
}

// ── Vault Commands ──────────────────────────────────────────
export const openVault   = (path: string) => call<VaultInfoDto>('open_vault',  { path });
export const createVault = (path: string) => call<VaultInfoDto>('create_vault',{ path });
export const getVaultInfo = ()             => call<VaultInfoDto | null>('get_vault_info');

// ── Note Commands ───────────────────────────────────────────
/**
 * Rust backend `create_note` imzası:
 *   title: String, body: String, parent_id: Option<String>,
 *   color: Option<String>, pinned: bool, tags: Vec<String>
 */
export const createNote = (
  title = 'Untitled',
  body = '',
  parentId: string | null = null,
  color: string | null = null,
  pinned = false,
  tags: string[] = []
) => call<NoteDto>('create_note', {
  title,
  body,
  parentId,
  color,
  pinned,
  tags,
});

export const getNote  = (id: string)  => call<NoteDto>('get_note', { id });
export const getNoteMetadata = (id: string) => call<NoteMetadataDto>('get_note_metadata', { id });
export const listNotes = ()           => call<NoteListItemDto[]>('list_notes');

/**
 * Rust backend `update_note` imzası:
 *   id, title, body, parent_id, color, pinned, tags
 */
export const updateNote = (
  id: string,
  title: string,
  body: string,
  parentId: string | null = null,
  color: string | null = null,
  pinned = false,
  tags: string[] = []
) => call<NoteDto>('update_note', {
  id,
  title,
  body,
  parentId,
  color,
  pinned,
  tags,
});

export const renameNote  = (id: string, newTitle: string) => call<NoteDto>('rename_note',  { id, newTitle });
export const deleteNote  = (id: string)                   => call<void>('delete_note',     { id });

// ── Search Commands ─────────────────────────────────────────
export interface SearchResult {
  id: string;
  title: string;
  snippet: string;
}
export const searchNotes = (query: string) => call<SearchResult[]>('search_notes', { query });

// ── Sync Commands ───────────────────────────────────────────
export const startSync  = ()                  => call<void>('start_sync');
export const stopSync   = ()                  => call<void>('stop_sync');
/**
 * syncNow artık SyncReport döndürüyor — backend zaten döndürüyordu ama
 * frontend void olarak işaretliyordu.
 */
export const syncNow    = ()                  => call<SyncReport>('sync_now');
export const getSyncStatus = ()               => call<SyncStatus>('get_sync_status');
export const updateSyncConfig = (config: SyncConfig) => call<void>('update_sync_config', { config });
export const validateSyncConfig = (config: SyncConfig) => call<void>('validate_sync_config', { config });
export const getSyncConfig = ()               => call<SyncConfig>('get_sync_config');

// ── History Commands ────────────────────────────────────────
export const listSnapshots    = (noteId: string)                    => call<Snapshot[]>('list_snapshots',    { noteId });
export const restoreSnapshot  = (noteId: string, timestamp: string) => call<NoteDto>('restore_snapshot',    { noteId, timestamp });
export const compareSnapshot  = (noteId: string, timestamp: string) => call<SnapshotDiffDto>('compare_snapshot',    { noteId, timestamp });
export const deleteSnapshot   = (noteId: string, timestamp: string) => call<void>('delete_snapshot',     { noteId, timestamp });

// ── Trash Commands ──────────────────────────────────────────
export const listTrash        = ()            => call<TrashEntry[]>('list_trash');
export const trashNote        = (id: string)  => call<void>('trash_note',          { id });
export const restoreFromTrash = (id: string)  => call<void>('restore_from_trash',  { noteId: id });
export const permanentDelete  = (id: string)  => call<void>('permanent_delete',    { noteId: id });
export const getTrashNote     = (id: string)  => call<import('../types').NoteDto>('get_trash_note', { id });

// ── Conflict Commands ────────────────────────────────────────
export const listConflicts            = ()                                          => call<import('../types').ConflictEntry[]>('list_conflicts');
export const getConflictNote          = (archivedPath: string)                      => call<import('../types').NoteDto>('get_conflict_note', { archivedPath });
export const resolveConflictKeepLocal = (archivedPath: string)                      => call<void>('resolve_conflict_keep_local', { archivedPath });
export const resolveConflictKeepRemote = (noteId: string, archivedPath: string)     => call<void>('resolve_conflict_keep_remote', { noteId, archivedPath });

// ── Attachment Commands ─────────────────────────────────────
export const addAttachment             = (sourcePath: string) => call<string>('add_attachment',    { sourcePath });
export const listAttachments           = ()                   => call<string[]>('list_attachments');
export const deleteAttachment          = (name: string)       => call<void>('delete_attachment',   { name });
export const listAttachmentsWithMetadata = ()                   => call<AttachmentInfoDto[]>('list_attachments_with_metadata');

// ── Settings Commands ───────────────────────────────────────
export const getSettings      = ()                   => call<AppConfig>('get_settings');
export const saveSettings     = (config: AppConfig)  => call<void>('save_settings',       { config });

// ── Maintenance Commands ────────────────────────────────────
export interface OrphanedAttachment {
  filename: string;
  size_bytes: number;
}
export const rebuildDatabaseCache      = () => call<void>('rebuild_database_cache');
export const vacuumDatabaseCache       = () => call<void>('vacuum_database_cache');
export const getOrphanedAttachments    = () => call<OrphanedAttachment[]>('get_orphaned_attachments');
export const deleteOrphanedAttachments  = (filenames: string[]) => call<void>('delete_orphaned_attachments', { filenames });
export const clearSyncQueue            = () => call<void>('clear_sync_queue');
export const clearSyncCache            = () => call<void>('clear_sync_cache');

export interface DuplicateFileEntry {
  relative_path: string;
  last_modified: string;
  size_bytes: number;
}
export interface DuplicateNoteGroup {
  note_id: string;
  title: string;
  files: DuplicateFileEntry[];
}
export const getDuplicateNotes          = () => call<DuplicateNoteGroup[]>('get_duplicate_notes');
export const deleteDuplicateNoteFile    = (relativePath: string) => call<void>('delete_duplicate_note_file', { relativePath });

// ── Folder Commands ─────────────────────────────────────────
export const listFolders  = () => call<string[]>('list_folders');
export const createFolder = (relPath: string) => call<void>('create_folder', { relPath });
export const deleteFolder = (relPath: string) => call<void>('delete_folder', { relPath });
export const moveNote     = (id: string, targetDir: string) => call<void>('move_note', { id, targetDir });
export const moveFolder   = (srcDir: string, targetDir: string) => call<void>('move_folder', { srcDir, targetDir });
export const renameFolder = (srcDir: string, newName: string) => call<void>('rename_folder', { srcDir, newName });

// ── Added Rich Editor & Import Commands ─────────────────────
export const importNote = (sourcePath: string, targetDir: string | null) =>
  call<NoteDto>('import_note', { sourcePath, targetDir });

export const importNoteFromContent = (title: string, content: string, targetDir: string | null) =>
  call<NoteDto>('import_note_from_content', { title, content, targetDir });

export const addAttachmentBytes = (fileName: string, bytes: number[]) =>
  call<string>('add_attachment_bytes', { fileName, bytes });



