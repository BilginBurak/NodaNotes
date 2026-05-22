import { writable } from 'svelte/store';
import type { SyncStatus, SyncReport, ConflictEntry, SyncConfig } from '../types';
import * as ipc from '../services/ipc';

export const syncStatus    = writable<SyncStatus>({ status: 'Idle' });
export const syncConfig    = writable<SyncConfig>({
  webdav_url: '',
  webdav_username: '',
  interval_secs: 300,
});
export const syncConflicts  = writable<ConflictEntry[]>([]);
export const loadingSync    = writable<boolean>(false);
export const syncError      = writable<string | null>(null);

/** Tamamlanan son sync raporunu tutar — bildirim ve detay modal için */
export const lastSyncReport = writable<SyncReport | null>(null);
/** Sync özet bildiriminin gösterilip gösterilmeyeceği */
export const showSyncReport = writable<boolean>(false);

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

export async function saveSyncConfig(config: SyncConfig) {
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
    const res: any = await ipc.getSyncStatus();
    syncStatus.set({
      status: res.status,
      last_sync_time: res.last_sync_time || undefined,
      error_message: res.error_message || undefined
    });
  } catch (e: any) {
    console.error('Failed to load sync status:', e);
  }
}

/**
 * Manuel ya da otomatik sync tetikler.
 * Tamamlanınca SyncReport'u store'a yazar ve bildirimi gösterir.
 */
export async function triggerSyncNow(): Promise<SyncReport | null> {
  syncError.set(null);
  syncStatus.set({ status: 'Syncing' });
  try {
    const report = await ipc.syncNow();
    // Toplam işlem sayısını ekle
    const enriched: SyncReport = {
      ...report,
      total: report.uploads + report.downloads + report.deletes_local + report.deletes_remote + report.conflicts,
      completed_at: new Date().toISOString(),
    };
    lastSyncReport.set(enriched);
    showSyncReport.set(true);
    syncStatus.set({ status: 'Idle', last_sync_time: enriched.completed_at });
    return enriched;
  } catch (e: any) {
    syncError.set(e.message || 'Failed to trigger sync');
    syncStatus.set({ status: 'Error', error_message: e.message });
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

/** Bildirim kapatıldığında çağrılır */
export function dismissSyncReport() {
  showSyncReport.set(false);
}
