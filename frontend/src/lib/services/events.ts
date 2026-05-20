import { listen } from '@tauri-apps/api/event';
import type { SyncStatus, ConflictEntry } from '../types';

export type VaultUpdatedPayload = Record<string, 'Created' | 'Modified' | 'Deleted' | 'Renamed'>;

/**
 * Listens to Tauri system events and forwards them to Svelte stores/callbacks
 */
export async function listenToVaultUpdated(callback: (payload: VaultUpdatedPayload) => void) {
  return await listen<VaultUpdatedPayload>('vault_updated', (event) => {
    callback(event.payload);
  });
}

export async function listenToSyncStatus(callback: (status: SyncStatus) => void) {
  return await listen<SyncStatus>('sync_status_changed', (event) => {
    callback(event.payload);
  });
}

export async function listenToSyncConflict(callback: (conflict: ConflictEntry) => void) {
  return await listen<ConflictEntry>('sync_conflict', (event) => {
    callback(event.payload);
  });
}
