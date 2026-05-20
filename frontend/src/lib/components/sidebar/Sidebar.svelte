<script lang="ts">
  import { onMount } from 'svelte';
  import { vaultInfo, checkActiveVault } from '../../stores/vault';
  import { trashList, loadTrash, recoverFromTrash, emptyTrashPermanently } from '../../stores/editor';
  import { syncConfig, loadSyncConfig, saveSyncConfig, syncStatus, triggerSyncNow } from '../../stores/sync';
  import { validateSyncConfig } from '../../services/ipc';

  $: info = $vaultInfo;
  $: trash = $trashList;
  $: config = $syncConfig;
  $: status = $syncStatus;

  let activeTab: 'navigation' | 'trash' = 'navigation';
  let showSyncConfigModal = false;

  // Sync Form State
  let webdavUrl = '';
  let webdavUsername = '';
  let webdavPassword = '';
  let syncIntervalSecs = 300;
  let saveStatus: 'idle' | 'saving' | 'success' | 'error' = 'idle';
  let saveErrorMessage = '';

  // Validation State
  let validationStatus: 'idle' | 'testing' | 'success' | 'error' = 'idle';
  let validationErrorMessage = '';

  async function handleVerifySync() {
    validationStatus = 'testing';
    validationErrorMessage = '';
    try {
      await validateSyncConfig({
        webdav_url: webdavUrl,
        webdav_username: webdavUsername,
        webdav_password: webdavPassword ? webdavPassword : undefined,
        interval_secs: syncIntervalSecs
      });
      validationStatus = 'success';
      setTimeout(() => {
        validationStatus = 'idle';
      }, 3000);
    } catch (e: any) {
      validationStatus = 'error';
      validationErrorMessage = e.message || 'Verification failed';
    }
  }

  function openSyncConfig() {
    webdavUrl = config.webdav_url;
    webdavUsername = config.webdav_username;
    webdavPassword = ''; // Keep password hidden/empty by default for safety
    syncIntervalSecs = config.interval_secs;
    showSyncConfigModal = true;
    saveStatus = 'idle';
    validationStatus = 'idle';
    validationErrorMessage = '';
  }

  function closeSyncConfig() {
    showSyncConfigModal = false;
  }

  async function handleSaveSync() {
    saveStatus = 'saving';
    saveErrorMessage = '';
    try {
      await saveSyncConfig({
        webdav_url: webdavUrl,
        webdav_username: webdavUsername,
        webdav_password: webdavPassword ? webdavPassword : undefined,
        interval_secs: syncIntervalSecs
      });
      saveStatus = 'success';
      setTimeout(() => {
        showSyncConfigModal = false;
        saveStatus = 'idle';
      }, 800);
    } catch (e: any) {
      saveStatus = 'error';
      saveErrorMessage = e.message || 'Failed to save configuration';
    }
  }

  function closeVault() {
    // Resetting vaultInfo takes us back to launcher
    vaultInfo.set(null);
  }

  async function handleRestoreTrash(id: string) {
    try {
      await recoverFromTrash(id);
    } catch (e) {
      console.error('Failed to restore note:', e);
    }
  }

  async function handlePermanentDelete(id: string) {
    if (confirm('Are you sure you want to permanently delete this note? This action is absolutely irrecoverable.')) {
      try {
        await emptyTrashPermanently(id);
      } catch (e) {
        console.error('Failed to delete permanently:', e);
      }
    }
  }

  onMount(() => {
    loadTrash();
    loadSyncConfig();
  });
</script>

<div class="sidebar border-right">
  <!-- Top: App Logo & Section Toggles -->
  <div class="sidebar-header" data-tauri-drag-region>
    <div class="logo-area" data-tauri-drag-region>
      <div class="logo-ring">
        <div class="logo-dot"></div>
      </div>
      <span class="app-title">Noda</span>
    </div>

    <div class="tab-switcher">
      <button
        class="tab-btn"
        class:active={activeTab === 'navigation'}
        onclick={() => activeTab = 'navigation'}
        title="Navigator"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A9 9 0 0 1 12 3v0a9 9 0 0 1 9 9v.75m-18 0a2.25 2.25 0 0 0 2.25 2.25h13.5A2.25 2.25 0 0 0 21 12.75m-18 0V12a9 9 0 0 1 9-9v0a9 9 0 0 1 9 9v.75" />
        </svg>
      </button>
      <button
        class="tab-btn"
        class:active={activeTab === 'trash'}
        onclick={() => { activeTab = 'trash'; loadTrash(); }}
        title="Trash Bin"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.34 9m-4.78 0L9 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
        </svg>
        {#if trash.length > 0}
          <span class="badge">{trash.length}</span>
        {/if}
      </button>
    </div>
  </div>

  <!-- Middle Pane Content -->
  <div class="sidebar-body scrollbar-thin">
    {#if activeTab === 'navigation'}
      <div class="nav-section">
        <h3 class="section-title">Workspace</h3>
        <div class="nav-items">
          <div class="nav-item active">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4 icon">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 21a9.004 9.004 0 0 0 8.716-6.747M12 21a9.004 9.004 0 0 1-8.716-6.747M12 21c2.485 0 4.5-4.03 4.5-9S14.485 3 12 3m0 18c-2.485 0-4.5-4.03-4.5-9S9.515 3 12 3m0 0a8.997 8.997 0 0 1 7.843 4.582M12 3a8.997 8.997 0 0 0-7.843 4.582m15.686 0A11.953 11.953 0 0 1 12 10.5c-2.998 0-5.74-1.1-7.843-2.918m15.686 0A8.959 8.959 0 0 1 21 12c0 .778-.099 1.533-.284 2.253m0 0A17.919 17.919 0 0 1 12 16.5c-3.162 0-6.133-.815-8.716-2.247m0 0A9.015 9.015 0 0 1 3 12c0-.778.099-1.533.284-2.253" />
            </svg>
            <span>All Documents</span>
          </div>
        </div>
      </div>

      <div class="nav-section">
        <h3 class="section-title">Cloud Synchronization</h3>
        <div class="nav-items">
          <button class="nav-item action-btn" onclick={openSyncConfig}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4 icon">
              <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12a7.5 7.5 0 0 0 15 0m-15 0a7.5 7.5 0 1 1 15 0m-15 0H3m16.5 0H21m-1.5 0H12m-8.457 3.077 1.41-.513m14.095-5.13 1.41-.513M5.106 17.785l1.15-.827m11.488-8.232 1.15-.827M6.89 20.08l.75-1.143m10.72-8.15.75-1.142m-12.784 10.5.22-1.343m13.064-9.357.22-1.343M12 21v-1.5m0-13.5V3" />
            </svg>
            <span>WebDAV Settings</span>
          </button>
        </div>
      </div>
    {:else}
      <!-- Trash tab -->
      <div class="trash-section">
        <div class="trash-header-row">
          <h3 class="section-title">Trash Bin</h3>
        </div>

        {#if trash.length === 0}
          <div class="trash-empty">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-8 h-8">
              <path stroke-linecap="round" stroke-linejoin="round" d="m20.25 7.5-.625 10.632a2.25 2.25 0 0 1-2.247 2.118H6.622a2.25 2.25 0 0 1-2.247-2.118L3.75 7.5m6 4.125 2.25 2.25m0 0 2.25 2.25m-2.25-2.25 2.25-2.25m-2.25 2.25-2.25 2.25M3.75 7.5h16.5M9 3.75h6m-9 3h12" />
            </svg>
            <p>Trash is empty</p>
          </div>
        {:else}
          <div class="trash-items-list">
            {#each trash as note (note.id)}
              <div class="trash-item-card">
                <div class="trash-item-info">
                  <span class="trash-item-title">{note.title || 'Untitled'}</span>
                  <span class="trash-item-path">{note.original_path.split('/').pop()}</span>
                </div>
                <div class="trash-item-actions">
                  <button class="action-btn restore" onclick={() => handleRestoreTrash(note.id)} title="Restore note">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M9 15 3 9m0 0 6-6M3 9h12a6 6 0 0 1 0 12h-3" />
                    </svg>
                  </button>
                  <button class="action-btn delete" onclick={() => handlePermanentDelete(note.id)} title="Delete permanently">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-3.5 h-3.5">
                      <path stroke-linecap="round" stroke-linejoin="round" d="m14.74 9-.34 9m-4.78 0L9 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0" />
                    </svg>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Bottom: Vault Settings Panel -->
  <div class="sidebar-footer border-top">
    {#if info}
      <div class="vault-detail">
        <div class="vault-detail-meta">
          <span class="vault-label">Active Vault:</span>
          <span class="vault-path-text" title={info.path}>{info.path}</span>
        </div>
        <button class="close-vault-btn" onclick={closeVault} title="Close Vault">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15M12 9l-3 3m0 0 3 3m-3-3h12.75" />
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <!-- Sync Configuration Settings Modal -->
  {#if showSyncConfigModal}
    <div class="modal-backdrop" onclick={closeSyncConfig}></div>
    <div class="modal border-glow">
      <div class="modal-header">
        <h3>Cloud Sync Settings</h3>
        <button class="modal-close" onclick={closeSyncConfig}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="modal-body">
        <p class="description">
          Integrate with standard WebDAV servers (e.g. InfiniCLOUD, Nextcloud) to sync all markdown files, history snapshots, and attachments asynchronously in the background.
        </p>

        <div class="form-group">
          <label for="webdav-url">WebDAV Server Endpoint URL</label>
          <input
            id="webdav-url"
            type="url"
            placeholder="https://example.com/dav/"
            bind:value={webdavUrl}
          />
        </div>

        <div class="form-row">
          <div class="form-group flex-1">
            <label for="webdav-user">Username</label>
            <input
              id="webdav-user"
              type="text"
              placeholder="username"
              bind:value={webdavUsername}
            />
          </div>
          <div class="form-group flex-1">
            <label for="webdav-pass">App Password</label>
            <input
              id="webdav-pass"
              type="password"
              placeholder="••••••••••••"
              bind:value={webdavPassword}
            />
          </div>
        </div>

        <div class="form-group">
          <label for="sync-interval">Sync Interval (seconds)</label>
          <select id="sync-interval" bind:value={syncIntervalSecs}>
            <option value={60}>Every 1 minute</option>
            <option value={300}>Every 5 minutes</option>
            <option value={900}>Every 15 minutes</option>
            <option value={3600}>Every 1 hour</option>
          </select>
        </div>

        {#if saveStatus === 'success'}
          <div class="status-alert success">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
            </svg>
            <span>Sync configuration updated successfully!</span>
          </div>
        {:else}
          {#if saveStatus === 'error'}
            <div class="status-alert error">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
              </svg>
              <span>{saveErrorMessage}</span>
            </div>
          {/if}
        {/if}

        {#if validationStatus === 'success'}
          <div class="status-alert success">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
            </svg>
            <span>Connection verified successfully!</span>
          </div>
        {:else}
          {#if validationStatus === 'error'}
            <div class="status-alert error">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
              </svg>
              <span>{validationErrorMessage}</span>
            </div>
          {/if}
        {/if}
      </div>

      <div class="modal-footer">
        <button
          class="btn btn-secondary verify-btn"
          style="margin-right: auto;"
          onclick={handleVerifySync}
          disabled={saveStatus === 'saving' || validationStatus === 'testing'}
        >
          {#if validationStatus === 'testing'}
            <div class="spinner-sm"></div>
            <span>Testing...</span>
          {:else}
            <span>Verify Connection</span>
          {/if}
        </button>

        <button class="btn btn-secondary" onclick={closeSyncConfig} disabled={saveStatus === 'saving'}>Cancel</button>
        <button class="btn btn-primary" onclick={handleSaveSync} disabled={saveStatus === 'saving'}>
          {#if saveStatus === 'saving'}
            <div class="spinner-sm"></div>
            <span>Saving...</span>
          {:else}
            <span>Save Configuration</span>
          {/if}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .sidebar {
    width: 220px;
    height: 100%;
    background-color: #06080c;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
  }

  .border-right {
    border-right: 1px solid rgba(255, 255, 255, 0.05);
  }

  .sidebar-header {
    height: 48px;
    padding: 0 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
    flex-shrink: 0;
  }

  :global(.platform-darwin) .sidebar-header {
    /* Padding for macOS traffic lights gap */
    padding-left: 20px;
  }

  .logo-area {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .logo-ring {
    width: 14px;
    height: 14px;
    border: 2px solid #6366f1;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .logo-dot {
    width: 4px;
    height: 4px;
    background-color: #818cf8;
    border-radius: 50%;
  }

  .app-title {
    font-size: 0.9rem;
    font-weight: 700;
    color: #f8fafc;
    letter-spacing: 0.5px;
  }

  .tab-switcher {
    display: flex;
    gap: 2px;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.04);
    border-radius: 6px;
    padding: 2px;
  }

  .tab-btn {
    background: transparent;
    border: none;
    color: #475569;
    padding: 4px;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    position: relative;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    color: #94a3b8;
    background-color: rgba(255, 255, 255, 0.03);
  }

  .tab-btn.active {
    color: #818cf8;
    background-color: rgba(99, 102, 241, 0.1);
  }

  .tab-btn .badge {
    position: absolute;
    top: -4px;
    right: -4px;
    background-color: #6366f1;
    color: #ffffff;
    font-size: 0.6rem;
    font-weight: 600;
    min-width: 12px;
    height: 12px;
    border-radius: 9999px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1px;
    box-shadow: 0 0 6px rgba(99, 102, 241, 0.6);
  }

  .sidebar-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 8px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .nav-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-title {
    margin: 0 8px;
    font-size: 0.68rem;
    font-weight: 700;
    color: #3f3f46;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .nav-items {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 6px;
    color: #71717a;
    font-size: 0.8rem;
    font-weight: 500;
    user-select: none;
    transition: all 0.2s ease;
  }

  .nav-item.active {
    color: #818cf8;
    background-color: rgba(99, 102, 241, 0.05);
  }

  .nav-item.action-btn {
    background: transparent;
    border: none;
    width: 100%;
    cursor: pointer;
    text-align: left;
  }

  .nav-item.action-btn:hover {
    color: #cbd5e1;
    background-color: rgba(255, 255, 255, 0.02);
  }

  .nav-item .icon {
    flex-shrink: 0;
  }

  /* Trash styles */
  .trash-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
  }

  .trash-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #27272a;
    padding: 40px 0;
    text-align: center;
  }

  .trash-empty svg {
    margin-bottom: 8px;
  }

  .trash-empty p {
    font-size: 0.75rem;
    margin: 0;
  }

  .trash-items-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .trash-item-card {
    background-color: rgba(255, 255, 255, 0.01);
    border: 1px solid rgba(255, 255, 255, 0.02);
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    transition: all 0.15s ease;
  }

  .trash-item-card:hover {
    border-color: rgba(255, 255, 255, 0.05);
    background-color: rgba(255, 255, 255, 0.02);
  }

  .trash-item-info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }

  .trash-item-title {
    font-size: 0.78rem;
    font-weight: 500;
    color: #cbd5e1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .trash-item-path {
    font-size: 0.65rem;
    color: #475569;
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .trash-item-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .trash-item-actions .action-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .trash-item-actions .action-btn.restore {
    color: #818cf8;
  }

  .trash-item-actions .action-btn.restore:hover {
    background-color: rgba(99, 102, 241, 0.15);
    color: #a5b4fc;
  }

  .trash-item-actions .action-btn.delete {
    color: #f43f5e;
  }

  .trash-item-actions .action-btn.delete:hover {
    background-color: rgba(244, 63, 94, 0.15);
    color: #fda4af;
  }

  /* Footer styling */
  .sidebar-footer {
    padding: 12px 16px;
    background-color: rgba(0, 0, 0, 0.1);
    flex-shrink: 0;
  }

  .border-top {
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .vault-detail {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .vault-detail-meta {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
  }

  .vault-label {
    font-size: 0.65rem;
    color: #475569;
    font-weight: 500;
  }

  .vault-path-text {
    font-size: 0.72rem;
    color: #94a3b8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: monospace;
  }

  .close-vault-btn {
    background: transparent;
    border: none;
    color: #475569;
    cursor: pointer;
    padding: 6px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .close-vault-btn:hover {
    color: #f43f5e;
    background-color: rgba(244, 63, 94, 0.1);
  }

  /* Modal styling */
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(3, 7, 18, 0.6);
    backdrop-filter: blur(4px);
    z-index: 10002;
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 480px;
    max-width: 90vw;
    background-color: #0d111a;
    border-radius: 12px;
    z-index: 10003;
    display: flex;
    flex-direction: column;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.8);
    animation: zoomIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes zoomIn {
    from {
      opacity: 0;
      transform: translate(-50%, -48%) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
    }
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .modal-header h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: #f1f5f9;
  }

  .modal-close {
    background: transparent;
    border: none;
    color: #475569;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-close:hover {
    color: #ffffff;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-body .description {
    font-size: 0.76rem;
    color: #94a3b8;
    line-height: 1.4;
    margin: 0 0 4px 0;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-row {
    display: flex;
    gap: 12px;
  }

  .flex-1 {
    flex: 1;
  }

  .form-group label {
    font-size: 0.7rem;
    font-weight: 600;
    color: #475569;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .form-group input, .form-group select {
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    padding: 8px 12px;
    color: #f1f5f9;
    font-size: 0.82rem;
    font-family: inherit;
    outline: none;
    transition: all 0.15s ease;
  }

  .form-group input:focus, .form-group select:focus {
    border-color: rgba(99, 102, 241, 0.5);
    background-color: rgba(255, 255, 255, 0.03);
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.1);
  }

  .status-alert {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-radius: 6px;
    font-size: 0.76rem;
    font-weight: 500;
  }

  .status-alert.success {
    background-color: rgba(16, 185, 129, 0.06);
    border: 1px solid rgba(16, 185, 129, 0.2);
    color: #10b981;
  }

  .status-alert.error {
    background-color: rgba(244, 63, 94, 0.06);
    border: 1px solid rgba(244, 63, 94, 0.2);
    color: #f43f5e;
  }

  .modal-footer {
    padding: 14px 20px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    background-color: rgba(255, 255, 255, 0.01);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #cbd5e1;
  }

  .btn-secondary:hover:not(:disabled) {
    background-color: rgba(255, 255, 255, 0.03);
    color: #ffffff;
  }

  .btn-primary {
    background-color: #6366f1;
    border: none;
    color: #ffffff;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: #4f46e5;
    box-shadow: 0 0 12px rgba(99, 102, 241, 0.4);
  }

  .spinner-sm {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.2);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .verify-btn {
    border-color: rgba(99, 102, 241, 0.3);
    color: #a5b4fc;
  }

  .verify-btn:hover:not(:disabled) {
    background-color: rgba(99, 102, 241, 0.1);
    color: #c7d2fe;
    border-color: rgba(99, 102, 241, 0.5);
  }

  /* Sleek Scrollbar */
  .scrollbar-thin::-webkit-scrollbar {
    width: 4px;
  }

  .scrollbar-thin::-webkit-scrollbar-track {
    background: transparent;
  }

  .scrollbar-thin::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 9999px;
  }

  .scrollbar-thin::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.15);
  }
</style>
