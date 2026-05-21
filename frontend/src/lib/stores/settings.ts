import { writable } from 'svelte/store';
import type { AppConfig } from '../types';
import * as ipc from '../services/ipc';

export const appConfig = writable<AppConfig | null>(null);
export const loadingSettings = writable<boolean>(false);
export const settingsError = writable<string | null>(null);

export async function loadSettings() {
  loadingSettings.set(true);
  settingsError.set(null);
  try {
    const config = await ipc.getSettings();
    appConfig.set(config);
    return config;
  } catch (e: any) {
    console.error('Failed to load settings:', e);
    settingsError.set(e.message || 'Failed to load settings');
    return null;
  } finally {
    loadingSettings.set(false);
  }
}

export async function saveSettings(config: AppConfig) {
  loadingSettings.set(true);
  settingsError.set(null);
  try {
    await ipc.saveSettings(config);
    appConfig.set(config);
  } catch (e: any) {
    console.error('Failed to save settings:', e);
    settingsError.set(e.message || 'Failed to save settings');
    throw e;
  } finally {
    loadingSettings.set(false);
  }
}
