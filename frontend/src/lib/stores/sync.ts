import { writable } from 'svelte/store';
import type { SyncStatus, ConflictEntry } from '../types';
import * as ipc from '../services/ipc';

export const syncStatus = writable<SyncStatus>({ status: 'Idle' });
export const syncConfig = writable<ipc.SyncConfig>({
  webdav_url: '',
  webdav_username: '',
  interval_secs: 300
});
export const syncConflicts = writable<ConflictEntry[]>([]);
export const loadingSync = writable<boolean>(false);
export const syncError = writable<string | null>(null);

export async function loadSyncConfig() {
  loadingSync.set(true);
  syncError.set(null);
  try {
    const config = await ipc.getSyncConfig();
    syncConfig.set(config);
  } catch (e: any) {
    syncError.set(e.message || 'Failed to load sync configuration');
  } finally {
    loadingSync.set(false);
  }
}

export async function saveSyncConfig(config: ipc.SyncConfig) {
  loadingSync.set(true);
  syncError.set(null);
  try {
    await ipc.updateSyncConfig(config);
    syncConfig.set(config);
  } catch (e: any) {
    syncError.set(e.message || 'Failed to save sync configuration');
    throw e;
  } finally {
    loadingSync.set(false);
  }
}

export async function loadSyncStatus() {
  try {
    const status = await ipc.getSyncStatus();
    syncStatus.set(status);
  } catch (e: any) {
    console.error('Failed to load sync status:', e);
  }
}

export async function triggerSyncNow() {
  syncError.set(null);
  try {
    await ipc.syncNow();
  } catch (e: any) {
    syncError.set(e.message || 'Failed to trigger sync');
    throw e;
  }
}

export async function startBackgroundSync() {
  try {
    await ipc.startSync();
  } catch (e: any) {
    console.error('Failed to start sync:', e);
  }
}

export async function stopBackgroundSync() {
  try {
    await ipc.stopSync();
  } catch (e: any) {
    console.error('Failed to stop sync:', e);
  }
}
