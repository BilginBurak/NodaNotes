import { listen } from '@tauri-apps/api/event';
import type { SyncStatus, ConflictEntry, SyncReport, SyncProgress } from '../types';

export type VaultUpdatedPayload = Record<string, 'Created' | 'Modified' | 'Deleted' | 'Renamed'>;

const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

/**
 * Listens to Tauri system events and forwards them to Svelte stores/callbacks.
 * Returns a dummy unlisten function in standard browser context to prevent crashes.
 */
export async function listenToVaultUpdated(callback: (payload: VaultUpdatedPayload) => void) {
  if (!isTauri) return () => {};
  return await listen<VaultUpdatedPayload>('vault_updated', (event) => {
    callback(event.payload);
  });
}

export async function listenToSyncStatus(callback: (status: SyncStatus) => void) {
  if (!isTauri) return () => {};
  return await listen<SyncStatus>('sync_status_changed', (event) => {
    callback(event.payload);
  });
}

export async function listenToSyncConflict(callback: (conflict: ConflictEntry) => void) {
  if (!isTauri) return () => {};
  return await listen<ConflictEntry>('sync_conflict', (event) => {
    callback(event.payload);
  });
}

export async function listenToSyncFinished(callback: (report: SyncReport) => void) {
  if (!isTauri) return () => {};
  return await listen<SyncReport>('sync_finished', (event) => {
    callback(event.payload);
  });
}

export async function listenToSyncProgress(callback: (progress: SyncProgress) => void) {
  if (!isTauri) return () => {};
  return await listen<SyncProgress>('sync_progress', (event) => {
    callback(event.payload);
  });
}
