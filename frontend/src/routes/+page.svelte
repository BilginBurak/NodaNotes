<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { vaultInfo, openExistingVault, createNewVault, vaultError, loadingVault } from '../lib/stores/vault';
  import { loadSettings } from '../lib/stores/settings';
  import Sidebar from '../lib/components/sidebar/Sidebar.svelte';
  import NoteList from '../lib/components/notelist/NoteList.svelte';
  import Editor from '../lib/components/editor/Editor.svelte';
  import Toolbar from '../lib/components/toolbar/Toolbar.svelte';
  import SyncReportModal from '../lib/components/sync/SyncReportModal.svelte';
  import SettingsModal from '../lib/components/settings/SettingsModal.svelte';

  $: info = $vaultInfo;
  $: error = $vaultError;
  $: loading = $loadingVault;

  let showSettingsModal = false;
  let settingsTab: 'appearance' | 'editor' | 'sync' | 'history' | 'vault' = 'appearance';

  onMount(async () => {
    try {
      await loadSettings();
    } catch (e) {
      console.error('Failed to load settings on startup:', e);
    }
  });

  async function handleOpenVault() {
    vaultError.set(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Vault Folder'
      });
      if (selected && typeof selected === 'string') {
        await openExistingVault(selected);
      }
    } catch (e: any) {
      console.error('Failed to open vault:', e);
      vaultError.set(`Could not open vault: ${e.message || e.toString()}`);
    }
  }

  async function handleCreateVault() {
    vaultError.set(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Choose Folder for New Vault'
      });
      if (selected && typeof selected === 'string') {
        await createNewVault(selected);
      }
    } catch (e: any) {
      console.error('Failed to create vault:', e);
      vaultError.set(`Could not create vault: ${e.message || e.toString()}`);
    }
  }
</script>

{#if !info}
  <!-- ─── Launcher ─────────────────────────────────── -->
  <div class="launcher">
    <!-- macOS traffic lights boşluğu -->
    <div class="launcher-traffic-lights" data-tauri-drag-region></div>

    <!-- Drag region — üst alan sürüklenebilir -->
    <div class="launcher-drag" data-tauri-drag-region></div>

    <!-- İçerik kartı -->
    <div class="launcher-card">
      <!-- Marka -->
      <div class="launcher-brand">
        <div class="brand-logo" aria-hidden="true">
          <svg viewBox="0 0 48 48" fill="none">
            <rect width="48" height="48" rx="12" fill="#0a84ff"/>
            <path d="M13 36V12l11 18 11-18v24" stroke="white" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <div class="brand-text">
          <h1>Noda</h1>
          <p>Local-first Markdown vault</p>
        </div>
      </div>

      <!-- Hata mesajı -->
      {#if error}
        <div class="error-box" role="alert">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
            <circle cx="8" cy="8" r="6.5"/>
            <line x1="8" y1="5" x2="8" y2="8.5"/>
            <line x1="8" y1="11" x2="8" y2="11" stroke-width="2.4"/>
          </svg>
          <div class="error-content">
            <strong>Failed to open vault</strong>
            <span>{error}</span>
          </div>
        </div>
      {/if}

      <!-- Eylem kartları -->
      <div class="action-list">
        <button
          class="action-card"
          onclick={handleOpenVault}
          disabled={loading}
        >
          <div class="action-icon open-icon" aria-hidden="true">
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 7a2 2 0 0 1 2-2h3.586a1 1 0 0 1 .707.293L10.5 6.5H15a2 2 0 0 1 2 2V14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"/>
            </svg>
          </div>
          <div class="action-text">
            <strong>Open Existing Vault</strong>
            <span>Select a folder with a .noda index</span>
          </div>
          <svg class="chevron" viewBox="0 0 10 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="2,2 8,8 2,14"/>
          </svg>
        </button>

        <button
          class="action-card"
          onclick={handleCreateVault}
          disabled={loading}
        >
          <div class="action-icon create-icon" aria-hidden="true">
            <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="10" cy="10" r="7.5"/>
              <line x1="10" y1="6.5" x2="10" y2="13.5"/>
              <line x1="6.5" y1="10" x2="13.5" y2="10"/>
            </svg>
          </div>
          <div class="action-text">
            <strong>Create New Vault</strong>
            <span>Initialize an empty directory as a vault</span>
          </div>
          <svg class="chevron" viewBox="0 0 10 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="2,2 8,8 2,14"/>
          </svg>
        </button>
      </div>

      <!-- Yükleniyor durumu -->
      {#if loading}
        <div class="loading-row" aria-live="polite">
          <div class="loading-spinner"></div>
          <span>Opening vault…</span>
        </div>
      {/if}

      <!-- Versiyon bilgisi -->
      <div class="launcher-footer">
        <span>Noda v2.0 · Tauri + Rust + SvelteKit</span>
      </div>
    </div>
  </div>

{:else}
  <!-- ─── Uygulama Workspace ───────────────────────── -->
  <div class="app-container">
    <Toolbar />
    <div class="app-workspace">
      <Sidebar onOpenSettings={(tab) => { showSettingsModal = true; settingsTab = tab; }} />
      <NoteList />
      <div class="main-content">
        <Editor />
      </div>
    </div>
    <!-- Global sync rapor bildirimi + detay modal -->
    <SyncReportModal />

    {#if showSettingsModal}
      <SettingsModal isOpen={showSettingsModal} bind:activeTab={settingsTab} on:close={() => showSettingsModal = false} />
    {/if}
  </div>
{/if}

<style>
  /* ── Launcher ── */
  .launcher {
    position: fixed;
    inset: 0;
    background-color: var(--bg-window);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    font-family: var(--font-sans);
    /* Hafif doku hissi */
    background-image: radial-gradient(
      ellipse at 50% 0%,
      rgba(10, 132, 255, 0.04) 0%,
      transparent 60%
    );
  }

  /* Traffic lights için sabit üst boşluk */
  .launcher-traffic-lights {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 48px;
  }

  .launcher-drag {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 48px;
  }

  /* ── Kart ── */
  .launcher-card {
    width: 400px;
    max-width: calc(100vw - 48px);
    display: flex;
    flex-direction: column;
    gap: 20px;
    animation: slideDown 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* ── Marka ── */
  .launcher-brand {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 4px;
  }

  .brand-logo {
    width: 48px;
    height: 48px;
    flex-shrink: 0;
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 8px 24px rgba(10, 132, 255, 0.25), 0 2px 6px rgba(0,0,0,0.5);
  }

  .brand-logo svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  .brand-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .brand-text h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.5px;
    line-height: 1;
  }

  .brand-text p {
    margin: 0;
    font-size: 13px;
    color: var(--text-tertiary);
  }

  /* ── Hata kutusu ── */
  .error-box {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background-color: var(--color-red-muted);
    border: 1px solid rgba(255, 69, 58, 0.25);
    border-radius: var(--radius-md);
    padding: 12px 14px;
    color: var(--color-red);
    font-size: 12px;
  }

  .error-box svg {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    margin-top: 1px;
  }

  .error-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .error-content strong {
    font-weight: 600;
    font-size: 12px;
  }

  .error-content span {
    color: rgba(255, 69, 58, 0.75);
    font-size: 11px;
    line-height: 1.4;
  }

  /* ── Eylem listesi ── */
  .action-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  /* Eylem kartı — macOS Settings tarzı büyük liste öğesi */
  .action-card {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    padding: 14px 16px;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
    transition: all 0.15s ease;
    box-sizing: border-box;
  }

  .action-card:hover:not(:disabled) {
    background-color: var(--bg-elevated-2);
    border-color: var(--border-normal);
  }

  .action-card:active:not(:disabled) {
    transform: scale(0.99);
  }

  .action-card:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  /* Eylem ikonları */
  .action-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: all 0.12s ease;
  }

  .action-icon svg {
    width: 18px;
    height: 18px;
    display: block;
  }

  .open-icon {
    background-color: var(--accent-muted);
    color: var(--accent);
  }

  .action-card:hover:not(:disabled) .open-icon {
    background-color: var(--accent);
    color: white;
  }

  .create-icon {
    background-color: var(--color-green-muted);
    color: var(--color-green);
  }

  .action-card:hover:not(:disabled) .create-icon {
    background-color: var(--color-green);
    color: white;
  }

  /* Eylem metni */
  .action-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    overflow: hidden;
  }

  .action-text strong {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: -0.1px;
  }

  .action-text span {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  /* Chevron ok */
  .chevron {
    width: 8px;
    height: 14px;
    color: var(--text-disabled);
    flex-shrink: 0;
    display: block;
    transition: color 0.12s ease;
  }

  .action-card:hover:not(:disabled) .chevron {
    color: var(--text-tertiary);
  }

  /* ── Yükleniyor ── */
  .loading-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .loading-spinner {
    width: 14px;
    height: 14px;
    border: 1.5px solid var(--border-normal);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  /* ── Footer ── */
  .launcher-footer {
    text-align: center;
    font-size: 11px;
    color: var(--text-disabled);
    padding-top: 4px;
  }

  /* ── App workspace — global class kullanımı ── */
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background-color: var(--bg-window);
  }

  .app-workspace {
    display: flex;
    flex: 1;
    height: calc(100vh - 48px);
    overflow: hidden;
  }

  .main-content {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
    position: relative;
  }
</style>
