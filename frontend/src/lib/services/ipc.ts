import { invoke } from '@tauri-apps/api/core';
import type {
  NoteDto,
  NoteListItemDto,
  VaultInfoDto,
  Snapshot,
  TrashEntry,
  SyncStatus,
  AppError
} from '../types';

async function call<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err: any) {
    // If err matches AppError shape, throw it, else make a generic one
    if (err && typeof err === 'object' && 'code' in err && 'message' in err) {
      throw err as AppError;
    }
    throw {
      code: 'UNKNOWN_ERROR',
      message: err?.toString() || 'An unexpected error occurred'
    } as AppError;
  }
}

// Vault Commands
export const openVault = (path: string) => call<VaultInfoDto>('open_vault', { path });
export const createVault = (path: string) => call<VaultInfoDto>('create_vault', { path });
export const getVaultInfo = () => call<VaultInfoDto | null>('get_vault_info');

// Note Commands
export const createNote = () => call<NoteDto>('create_note');
export const getNote = (id: string) => call<NoteDto>('get_note', { id });
export const updateNote = (id: string, title: string, body: string, frontmatter: Record<string, any>) =>
  call<NoteDto>('update_note', { id, title, body, frontmatter });
export const renameNote = (id: string, newTitle: string) => call<NoteDto>('rename_note', { id, newTitle });
export const deleteNote = (id: string) => call<void>('delete_note', { id });
export const listNotes = () => call<NoteListItemDto[]>('list_notes');

// Search Commands
export interface SearchResult {
  id: string;
  title: string;
  snippet: string;
}
export const searchNotes = (query: string) => call<SearchResult[]>('search_notes', { query });

// Sync Commands
export interface SyncConfig {
  webdav_url: string;
  webdav_username: string;
  webdav_password?: string;
  interval_secs: number;
}

export const startSync = () => call<void>('start_sync');
export const stopSync = () => call<void>('stop_sync');
export const syncNow = () => call<void>('sync_now');
export const getSyncStatus = () => call<SyncStatus>('get_sync_status');
export const updateSyncConfig = (config: SyncConfig) => call<void>('update_sync_config', { config });
export const validateSyncConfig = (config: SyncConfig) => call<void>('validate_sync_config', { config });
export const getSyncConfig = () => call<SyncConfig>('get_sync_config');

// History Commands
export const listSnapshots = (noteId: string) => call<Snapshot[]>('list_snapshots', { noteId });
export const restoreSnapshot = (noteId: string, timestamp: string) => call<NoteDto>('restore_snapshot', { noteId, timestamp });

// Trash Commands
export const listTrash = () => call<TrashEntry[]>('list_trash');
export const trashNote = (id: string) => call<void>('trash_note', { id });
export const restoreFromTrash = (id: string) => call<void>('restore_from_trash', { id });
export const permanentDelete = (id: string) => call<void>('permanent_delete', { id });

// Attachment Commands
export const addAttachment = (sourcePath: string) => call<string>('add_attachment', { sourcePath });
export const listAttachments = () => call<string[]>('list_attachments');
export const deleteAttachment = (name: string) => call<void>('delete_attachment', { name });
