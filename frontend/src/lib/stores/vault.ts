import { writable } from 'svelte/store';
import type { VaultInfoDto } from '../types';
import * as ipc from '../services/ipc';
import { loadSyncConfig, startBackgroundSync } from './sync';

export const vaultInfo = writable<VaultInfoDto | null>(null);
export const loadingVault = writable<boolean>(false);
export const vaultError = writable<string | null>(null);
export const vaultUnlocked = writable<boolean>(false);

export async function checkActiveVault() {
  loadingVault.set(true);
  vaultError.set(null);
  try {
    const info = await ipc.getVaultInfo();
    vaultInfo.set(info);
    if (info) {
      await loadSyncConfig();
      await startBackgroundSync();
      const unlocked = await ipc.isVaultSessionUnlocked();
      vaultUnlocked.set(unlocked);
    } else {
      vaultUnlocked.set(false);
    }
    return info;
  } catch (e: any) {
    vaultError.set(e.message || 'Failed to retrieve vault info');
    vaultInfo.set(null);
    vaultUnlocked.set(false);
    return null;
  } finally {
    loadingVault.set(false);
  }
}

export async function openExistingVault(path: string) {
  loadingVault.set(true);
  vaultError.set(null);
  try {
    const info = await ipc.openVault(path);
    vaultInfo.set(info);
    vaultUnlocked.set(false);
    await loadSyncConfig();
    await startBackgroundSync();
    return info;
  } catch (e: any) {
    vaultError.set(e.message || 'Failed to open vault');
    throw e;
  } finally {
    loadingVault.set(false);
  }
}

export async function createNewVault(path: string) {
  loadingVault.set(true);
  vaultError.set(null);
  try {
    const info = await ipc.createVault(path);
    vaultInfo.set(info);
    vaultUnlocked.set(false);
    await loadSyncConfig();
    await startBackgroundSync();
    return info;
  } catch (e: any) {
    vaultError.set(e.message || 'Failed to create vault');
    throw e;
  } finally {
    loadingVault.set(false);
  }
}
