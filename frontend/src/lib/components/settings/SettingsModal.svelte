<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { fade, scale, slide } from 'svelte/transition';
  import { appConfig, saveSettings, loadSettings, loadingSettings, settingsError } from '../../stores/settings';
  import {
    validateSyncConfig,
    rebuildDatabaseCache,
    vacuumDatabaseCache,
    getOrphanedAttachments,
    deleteOrphanedAttachments,
    clearSyncQueue,
    clearSyncCache
  } from '../../services/ipc';
  import type { OrphanedAttachment } from '../../services/ipc';
  import type { AppConfig } from '../../types';

  const dispatch = createEventDispatcher();

  export let isOpen = false;

  export let activeTab: 'appearance' | 'editor' | 'sync' | 'history' | 'vault' | 'maintenance' = 'appearance';

  // Config copy for editing
  let localConfig: AppConfig = {
    appearance: { theme: 'dark', accent_color: 'blue' },
    editor: { font_size: 14, typography: 'sans', show_word_count: true, auto_save_delay_ms: 1500 },
    sync: { webdav_url: '', webdav_username: '', webdav_password: '', interval_secs: 300 },
    history: { retention_days: 30, max_snapshots_per_note: 50, empty_trash_after_days: 30 }
  };

  // Sync state
  let webdavPassword = '';
  let validationStatus: 'idle' | 'testing' | 'success' | 'error' = 'idle';
  let validationErrorMessage = '';

  // Maintenance state
  let loadingAction: 'none' | 'rebuild_db' | 'vacuum_db' | 'scan_attachments' | 'delete_attachments' | 'reset_queue' | 'clear_cache' = 'none';
  let orphanedAttachments: OrphanedAttachment[] = [];
  let selectedAttachments: string[] = [];
  let scannedAttachments = false;
  let maintenanceSuccessMsg = '';
  let maintenanceErrorMsg = '';

  function clearMaintenanceMessages() {
    maintenanceSuccessMsg = '';
    maintenanceErrorMsg = '';
  }

  async function handleRebuildDatabase() {
    if (!confirm('Tüm veritabanı indeksleri sıfırdan yeniden oluşturulacaktır. Notlarınız silinmez, sadece arama ve indeksleme verileri taranır. Devam etmek istiyor musunuz?')) return;
    clearMaintenanceMessages();
    loadingAction = 'rebuild_db';
    try {
      await rebuildDatabaseCache();
      maintenanceSuccessMsg = 'Veritabanı indeksi başarıyla yeniden oluşturuldu!';
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Veritabanı derleme başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleVacuumDatabase() {
    clearMaintenanceMessages();
    loadingAction = 'vacuum_db';
    try {
      await vacuumDatabaseCache();
      maintenanceSuccessMsg = 'Veritabanı başarıyla sıkıştırıldı ve optimize edildi!';
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Sıkıştırma işlemi başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleScanAttachments() {
    clearMaintenanceMessages();
    loadingAction = 'scan_attachments';
    try {
      orphanedAttachments = await getOrphanedAttachments();
      selectedAttachments = orphanedAttachments.map(a => a.filename);
      scannedAttachments = true;
      if (orphanedAttachments.length === 0) {
        maintenanceSuccessMsg = 'Harika! Başıboş veya yetim kalmış hiçbir ek dosya bulunamadı.';
      }
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Ek taraması başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleDeleteSelectedAttachments() {
    if (selectedAttachments.length === 0) return;
    if (!confirm(`Seçilen ${selectedAttachments.length} adet bağımsız ek dosyayı kalıcı olarak silmek istediğinizden emin misiniz? Bu işlem geri alınamaz.`)) return;
    
    clearMaintenanceMessages();
    loadingAction = 'delete_attachments';
    try {
      await deleteOrphanedAttachments(selectedAttachments);
      maintenanceSuccessMsg = `${selectedAttachments.length} adet ek dosya başarıyla silindi ve depolama alanı geri kazanıldı!`;
      orphanedAttachments = orphanedAttachments.filter(a => !selectedAttachments.includes(a.filename));
      selectedAttachments = [];
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Silme işlemi başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleResetSyncQueue() {
    if (!confirm('Senkronizasyon kuyruğu tamamen temizlenecektir. Bekleyen dosya transferleri iptal edilir ve sıfırlanır. Devam etmek istiyor musunuz?')) return;
    clearMaintenanceMessages();
    loadingAction = 'reset_queue';
    try {
      await clearSyncQueue();
      maintenanceSuccessMsg = 'Senkronizasyon kuyruğu başarıyla sıfırlandı!';
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Kuyruk sıfırlama başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleClearSyncCache() {
    if (!confirm('Remote tracking cache sıfırlanacaktır. Bir sonraki eşitlemede tam karşılaştırma (Full Reconciliation) yapılacaktır. Devam etmek istiyor musunuz?')) return;
    clearMaintenanceMessages();
    loadingAction = 'clear_cache';
    try {
      await clearSyncCache();
      maintenanceSuccessMsg = 'Senkronizasyon cache verisi başarıyla temizlendi!';
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Eşitleme önbelleği temizleme başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  // Mock Vaults list (visually premium dummy)
  const mockVaults = [
    { name: 'NodaNotes Dev', path: '/Users/burakbilgin/Documents/Kodlar/Rust/NodaNotes', active: true, size: '24 MB', notes: 42 },
    { name: 'Personal Wiki', path: '/Users/burakbilgin/NodaVault/Personal', active: false, size: '154 MB', notes: 312 },
    { name: 'Work Notes', path: '/Users/burakbilgin/NodaVault/Work', active: false, size: '89 MB', notes: 145 },
    { name: 'Research Vault', path: '/Users/burakbilgin/NodaVault/Research', active: false, size: '412 MB', notes: 589 },
    { name: 'Recipe Box', path: '/Users/burakbilgin/NodaVault/Recipes', active: false, size: '12 MB', notes: 28 }
  ];

  onMount(async () => {
    await fetchSettings();
  });

  async function fetchSettings() {
    const config = await loadSettings();
    if (config) {
      // Create a deep copy
      localConfig = JSON.parse(JSON.stringify(config));
      webdavPassword = localConfig.sync.webdav_password || '';
    }
  }

  async function handleVerifySync() {
    validationStatus = 'testing';
    validationErrorMessage = '';
    try {
      await validateSyncConfig({
        ...localConfig.sync,
        webdav_password: webdavPassword || undefined
      });
      validationStatus = 'success';
      setTimeout(() => { validationStatus = 'idle'; }, 3000);
    } catch (e: any) {
      validationStatus = 'error';
      validationErrorMessage = e.message || 'Verification failed';
    }
  }

  async function handleSave() {
    try {
      localConfig.sync.webdav_password = webdavPassword || undefined;
      await saveSettings(localConfig);
      
      // Apply theme changes to document attribute if needed
      document.documentElement.setAttribute('data-theme', localConfig.appearance.theme);
      
      // Dispatch close
      dispatch('close');
    } catch (e) {
      console.error('Failed to save settings:', e);
    }
  }

  function handleClose() {
    dispatch('close');
  }
</script>

{#if isOpen}
  <div class="settings-backdrop" transition:fade={{ duration: 150 }} on:click={handleClose} role="presentation"></div>
  
  <div class="settings-modal" transition:scale={{ duration: 180, start: 0.96 }} role="dialog" aria-modal="true" aria-label="Settings">
    <!-- Left Navigation Sidebar -->
    <div class="settings-sidebar">
      <div class="sidebar-header">
        <svg class="settings-icon" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="10" cy="10" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33-1.82l-.06-.06a2 2 0 0 1-.7-.7 2 2 0 0 1-.3-1.54l.1-.38A1.65 1.65 0 0 0 17 8.5h-.08a2 2 0 0 1-1.26-.64 2 2 0 0 1-.54-1.18l-.1-.47a1.65 1.65 0 0 0-1.3-1.2h-.08a2 2 0 0 1-1.18-.54 2 2 0 0 1-.64-1.26l-.08-.08a1.65 1.65 0 0 0-1.82-.33l-.06.06a2 2 0 0 1-.7.7 2 2 0 0 1-1.54.3l-.38-.1a1.65 1.65 0 0 0-2 1.34V5a2 2 0 0 1-.64 1.26 2 2 0 0 1-1.18.54l-.47.1a1.65 1.65 0 0 0-1.2 1.3v.08a2 2 0 0 1-.54 1.18 2 2 0 0 1-1.26.64l-.08.08a1.65 1.65 0 0 0-.33 1.82l.06.06a2 2 0 0 1 .7.7 2 2 0 0 1 .3 1.54l-.1.38A1.65 1.65 0 0 0 3 11.5h.08a2 2 0 0 1 1.26.64 2 2 0 0 1 .54 1.18l.1.47a1.65 1.65 0 0 0 1.3 1.2h.08a2 2 0 0 1 1.18.54 2 2 0 0 1 .64 1.26l.08.08a1.65 1.65 0 0 0 1.82.33l.06-.06a2 2 0 0 1 .7-.7 2 2 0 0 1 1.54-.3l.38.1a1.65 1.65 0 0 0 2-1.34v-.08a2 2 0 0 1 .64-1.26 2 2 0 0 1 1.18-.54l.47-.1a1.65 1.65 0 0 0 1.2-1.3v-.08a2 2 0 0 1 .54-1.18 2 2 0 0 1 1.26-.64l.08-.08z"/>
        </svg>
        <span>Settings</span>
      </div>

      <nav class="sidebar-nav">
        <button class="nav-tab" class:active={activeTab === 'appearance'} on:click={() => activeTab = 'appearance'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2H4a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z"/>
            <circle cx="6" cy="6" r="1.5"/>
            <path d="m14 10-2.5-2.5a1 1 0 0 0-1.4 0L6 11.6"/>
          </svg>
          Appearance
        </button>

        <button class="nav-tab" class:active={activeTab === 'editor'} on:click={() => activeTab = 'editor'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11.5 2.5 13.5 4.5 5 13H3v-2L11.5 2.5z"/>
            <line x1="8" y1="12" x2="13" y2="12"/>
          </svg>
          Editor
        </button>

        <button class="nav-tab" class:active={activeTab === 'sync'} on:click={() => activeTab = 'sync'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M13 10a4 4 0 0 0-4-4H6a4 4 0 0 0 0 8h3"/>
            <polyline points="10,7 13,10 10,13"/>
          </svg>
          Sync & Cloud
        </button>

        <button class="nav-tab" class:active={activeTab === 'history'} on:click={() => activeTab = 'history'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="8" cy="8" r="6.5"/>
            <polyline points="8,4.5 8,8 10.5,10"/>
          </svg>
          History & Backup
        </button>

        <button class="nav-tab" class:active={activeTab === 'vault'} on:click={() => activeTab = 'vault'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2z"/>
            <line x1="5" y1="4" x2="11" y2="4"/>
          </svg>
          Vaults
        </button>

        <button class="nav-tab nav-tab-maintenance" class:active={activeTab === 'maintenance'} on:click={() => { activeTab = 'maintenance'; clearMaintenanceMessages(); }}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14.7 6.3a1 1 0 0 0 0-1.4l-1.4-1.4a1 1 0 0 0-1.4 0L3.7 10.7a1 1 0 0 0 0 1.4l1.4 1.4a1 1 0 0 0 1.4 0l7.2-7.2z"/>
            <path d="M14.7 6.3 10.2 10.8m0 0a2 2 0 1 0 2.8 2.8m-2.8-2.8a2 2 0 1 1 2.8 2.8"/>
          </svg>
          Maintenance
        </button>
      </nav>

      <div class="sidebar-footer">
        <span class="version-text">Noda v0.1.0</span>
      </div>
    </div>

    <!-- Right Content Area -->
    <div class="settings-content scrollbar-thin">
      {#if activeTab === 'appearance'}
        <div class="content-header">
          <h2>Appearance</h2>
          <p>Customize the look and feel of Noda.</p>
        </div>

        <div class="settings-section">
          <div class="form-group">
            <span class="group-label">Theme</span>
            <div class="theme-options">
              <label class="theme-card" class:selected={localConfig.appearance.theme === 'dark'}>
                <input type="radio" bind:group={localConfig.appearance.theme} value="dark" />
                <div class="theme-preview dark-preview">
                  <div class="preview-sidebar"></div>
                  <div class="preview-body"></div>
                </div>
                <span>Dark</span>
              </label>

              <label class="theme-card" class:selected={localConfig.appearance.theme === 'light'}>
                <input type="radio" bind:group={localConfig.appearance.theme} value="light" />
                <div class="theme-preview light-preview">
                  <div class="preview-sidebar"></div>
                  <div class="preview-body"></div>
                </div>
                <span>Light (Dummy)</span>
              </label>

              <label class="theme-card" class:selected={localConfig.appearance.theme === 'auto'}>
                <input type="radio" bind:group={localConfig.appearance.theme} value="auto" />
                <div class="theme-preview auto-preview">
                  <div class="preview-sidebar"></div>
                  <div class="preview-body"></div>
                </div>
                <span>System (Dummy)</span>
              </label>
            </div>
          </div>

          <div class="form-group">
            <label for="accent-color">Accent Color</label>
            <div class="accent-options">
              {#each ['blue', 'purple', 'green', 'orange', 'red'] as color}
                <label class="accent-dot-wrapper" class:selected={localConfig.appearance.accent_color === color}>
                  <input type="radio" bind:group={localConfig.appearance.accent_color} value={color} />
                  <span class="accent-dot accent-{color}" style="background-color: var(--color-{color === 'blue' ? 'blue' : color});"></span>
                </label>
              {/each}
            </div>
            <span class="input-desc">Select the primary tint color for active states (Dummy).</span>
          </div>
        </div>

      {:else if activeTab === 'editor'}
        <div class="content-header">
          <h2>Editor Settings</h2>
          <p>Configure typing dynamics, auto-saving, and font configurations.</p>
        </div>

        <div class="settings-section">
          <!-- Real Auto-Save Setting -->
          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="auto-save-delay">Auto-Save Delay</label>
                <span class="input-desc">Interval of inactivity after typing before saving note to disk.</span>
              </div>
              <div class="field-control">
                <select id="auto-save-delay" bind:value={localConfig.editor.auto_save_delay_ms}>
                  <option value={500}>500 ms (Fast)</option>
                  <option value={1000}>1 second</option>
                  <option value={1500}>1.5 seconds (Default)</option>
                  <option value={3000}>3 seconds</option>
                  <option value={5000}>5 seconds</option>
                  <option value={10000}>10 seconds</option>
                  <option value={30000}>30 seconds</option>
                  <option value={60000}>1 minute</option>
                  <option value={300000}>5 minutes</option>
                </select>
              </div>
            </div>
          </div>

          <!-- Dummy Settings -->
          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="editor-font-size">Font Size</label>
                <span class="input-desc">Set the font size for the Markdown text editor (Dummy).</span>
              </div>
              <div class="field-control">
                <input id="editor-font-size" type="number" min="10" max="24" bind:value={localConfig.editor.font_size} />
              </div>
            </div>
          </div>

          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="editor-typography">Typography</label>
                <span class="input-desc">Select font family used in editor workspace (Dummy).</span>
              </div>
              <div class="field-control">
                <select id="editor-typography" bind:value={localConfig.editor.typography}>
                  <option value="sans">System Sans-Serif</option>
                  <option value="serif">New York Serif</option>
                  <option value="mono">SF Mono Code</option>
                </select>
              </div>
            </div>
          </div>

          <div class="form-group">
            <div class="toggle-row">
              <div class="field-label-desc">
                <span class="toggle-label">Show Word Count</span>
                <span class="input-desc">Display active note character and word statistics in footer (Dummy).</span>
              </div>
              <label class="switch-control">
                <input type="checkbox" bind:checked={localConfig.editor.show_word_count} />
                <span class="switch-slider"></span>
              </label>
            </div>
          </div>
        </div>

      {:else if activeTab === 'sync'}
        <div class="content-header">
          <h2>Cloud Sync & Backup</h2>
          <p>Keep your notes safely backed up and synced via WebDAV servers.</p>
        </div>

        <div class="settings-section">
          <div class="form-group">
            <label for="webdav-url">WebDAV Server URL</label>
            <input id="webdav-url" type="url" placeholder="https://example.com/dav/" bind:value={localConfig.sync.webdav_url} />
            <span class="input-desc">Root folder WebDAV link from cloud provider (e.g. InfiniCLOUD, Nextcloud).</span>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="webdav-user">Username</label>
              <input id="webdav-user" type="text" placeholder="username" bind:value={localConfig.sync.webdav_username} />
            </div>
            <div class="form-group">
              <label for="webdav-pass">App Password</label>
              <input id="webdav-pass" type="password" placeholder="••••••••" bind:value={webdavPassword} />
            </div>
          </div>

          <div class="form-group">
            <label for="sync-interval">Sync Frequency</label>
            <select id="sync-interval" bind:value={localConfig.sync.interval_secs}>
              <option value={60}>Every 1 minute</option>
              <option value={300}>Every 5 minutes</option>
              <option value={900}>Every 15 minutes</option>
              <option value={3600}>Every hour</option>
            </select>
            <span class="input-desc">How often the engine should wake up and run background sync checks.</span>
          </div>

          <div class="verify-wrapper">
            <button
              class="btn btn-ghost verify-btn"
              on:click={handleVerifySync}
              disabled={validationStatus === 'testing'}
            >
              {#if validationStatus === 'testing'}
                <div class="spinner-sm"></div>Testing Server...
              {:else}
                Verify Connection
              {/if}
            </button>

            {#if validationStatus === 'success'}
              <div class="alert alert-success" transition:fade>
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="2,8 6,12 14,4"/>
                </svg>
                <span>Connection established and verified!</span>
              </div>
            {:else if validationStatus === 'error'}
              <div class="alert alert-error" transition:fade>
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                  <circle cx="8" cy="8" r="6"/>
                  <line x1="8" y1="5" x2="8" y2="8"/>
                  <line x1="8" y1="11" x2="8" y2="11"/>
                </svg>
                <span>{validationErrorMessage}</span>
              </div>
            {/if}
          </div>
        </div>

      {:else if activeTab === 'history'}
        <div class="content-header">
          <h2>History & Auto-Backup</h2>
          <p>Manage retention limits and trash policies for historical snapshots.</p>
        </div>

        <div class="settings-section">
          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="max-snapshots">Max Snapshots per Note</label>
                <span class="input-desc">Maximum snapshots saved per note. Excess old versions are pruned automatically.</span>
              </div>
              <div class="field-control">
                <input id="max-snapshots" type="number" min="5" max="500" bind:value={localConfig.history.max_snapshots_per_note} />
              </div>
            </div>
          </div>

          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="retention-days">Snapshot Retention (Days)</label>
                <span class="input-desc">Keep snapshots on disk for this number of days before automatic cleanup.</span>
              </div>
              <div class="field-control">
                <input id="retention-days" type="number" min="1" max="365" bind:value={localConfig.history.retention_days} />
              </div>
            </div>
          </div>

          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="empty-trash-days">Auto-Empty Trash</label>
                <span class="input-desc">Days a deleted note is kept in Trash before permanent physical deletion.</span>
              </div>
              <div class="field-control">
                <input id="empty-trash-days" type="number" min="1" max="180" bind:value={localConfig.history.empty_trash_after_days} />
              </div>
            </div>
          </div>
        </div>

      {:else if activeTab === 'vault'}
        <div class="content-header">
          <h2>Vaults Directory</h2>
          <p>List and switch between your local markdown vault folders.</p>
        </div>

        <div class="settings-section">
          <div class="vaults-list">
            {#each mockVaults as vault}
              <div class="vault-card" class:active={vault.active}>
                <div class="vault-info">
                  <div class="vault-title-row">
                    <span class="vault-name">{vault.name}</span>
                    {#if vault.active}
                      <span class="active-badge">Active</span>
                    {:else}
                      <span class="dummy-badge">Local Vault</span>
                    {/if}
                  </div>
                  <span class="vault-path">{vault.path}</span>
                  <div class="vault-stats">
                    <span>{vault.notes} Notes</span>
                    <span class="dot-divider">•</span>
                    <span>{vault.size}</span>
                  </div>
                </div>
                
                <div class="vault-actions">
                  {#if vault.active}
                    <button class="btn btn-secondary btn-sm" disabled>Current</button>
                  {:else}
                    <button class="btn btn-ghost btn-sm">Switch (Dummy)</button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>

      {:else if activeTab === 'maintenance'}
        <div class="content-header">
          <h2>Maintenance & Self-Healing</h2>
          <p>Troubleshoot, optimize, and reclaim disk space in your note vault.</p>
        </div>

        {#if maintenanceSuccessMsg}
          <div class="maintenance-alert alert-success" transition:fade>
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8">
              <path d="M13.25 4.75L6.75 11.25L2.75 7.25" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>{maintenanceSuccessMsg}</span>
            <button class="close-alert" on:click={() => maintenanceSuccessMsg = ''}>&times;</button>
          </div>
        {/if}

        {#if maintenanceErrorMsg}
          <div class="maintenance-alert alert-danger" transition:fade>
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8">
              <circle cx="8" cy="8" r="6.25"/>
              <line x1="8" y1="5" x2="8" y2="9" stroke-linecap="round"/>
              <circle cx="8" cy="11" r="0.75" fill="currentColor"/>
            </svg>
            <span>{maintenanceErrorMsg}</span>
            <button class="close-alert" on:click={() => maintenanceErrorMsg = ''}>&times;</button>
          </div>
        {/if}

        <div class="settings-section">
          <h3>Database Administration</h3>
          <p class="section-desc">Maintain search performance and clean index metadata.</p>
          
          <div class="maintenance-card">
            <div class="card-info">
              <span class="action-title">Rebuild Database Cache</span>
              <span class="action-desc">Fully re-scans vault note files and rebuilds the SQLite search and tag index from scratch. Useful if some notes are missing from list or search.</span>
            </div>
            <button class="btn btn-warning" on:click={handleRebuildDatabase} disabled={loadingAction !== 'none'}>
              {#if loadingAction === 'rebuild_db'}
                <div class="spinner-sm"></div>Processing...
              {:else}
                Rebuild Cache
              {/if}
            </button>
          </div>

          <div class="maintenance-card">
            <div class="card-info">
              <span class="action-title">Vacuum Database</span>
              <span class="action-desc">Defragments the database file, cleans unused cache spaces, and optimizes internal query performance. Safe to run anytime.</span>
            </div>
            <button class="btn btn-secondary" on:click={handleVacuumDatabase} disabled={loadingAction !== 'none'}>
              {#if loadingAction === 'vacuum_db'}
                <div class="spinner-sm"></div>Processing...
              {:else}
                Vacuum DB
              {/if}
            </button>
          </div>
        </div>

        <div class="settings-section">
          <h3>Attachments Diagnostics</h3>
          <p class="section-desc">Reclaim local storage space by purging unreferenced media files.</p>
          
          <div class="maintenance-card">
            <div class="card-info">
              <span class="action-title">Scan Orphaned Attachments</span>
              <span class="action-desc">Scans `.noda/attachments/` to identify images and files that are no longer linked or used inside any active note.</span>
            </div>
            <button class="btn btn-primary" on:click={handleScanAttachments} disabled={loadingAction !== 'none'}>
              {#if loadingAction === 'scan_attachments'}
                <div class="spinner-sm"></div>Scanning...
              {:else}
                Scan Files
              {/if}
            </button>
          </div>

          {#if scannedAttachments}
            <div class="orphaned-box" transition:slide>
              <div class="box-header">
                <span class="box-title">Found {orphanedAttachments.length} Orphaned Files</span>
                {#if orphanedAttachments.length > 0}
                  <span class="box-total-size">Total: {formatBytes(orphanedAttachments.reduce((sum, a) => sum + a.size_bytes, 0))}</span>
                {/if}
              </div>

              {#if orphanedAttachments.length === 0}
                <div class="empty-orphaned">
                  <span class="success-indicator">🎉</span>
                  <span class="empty-text">Your vault is fully optimized! No orphaned attachments found.</span>
                </div>
              {:else}
                <div class="orphaned-list scrollbar-thin">
                  {#each orphanedAttachments as att}
                    <label class="orphaned-item">
                      <input type="checkbox" bind:group={selectedAttachments} value={att.filename} />
                      <div class="item-details">
                        <span class="item-name">{att.filename}</span>
                        <span class="item-size">{formatBytes(att.size_bytes)}</span>
                      </div>
                    </label>
                  {/each}
                </div>
                
                <div class="orphaned-actions">
                  <span class="selected-count">{selectedAttachments.length} files selected</span>
                  <button class="btn btn-danger btn-sm" on:click={handleDeleteSelectedAttachments} disabled={selectedAttachments.length === 0 || loadingAction !== 'none'}>
                    {#if loadingAction === 'delete_attachments'}
                      <div class="spinner-sm"></div>Deleting...
                    {:else}
                      Permanently Delete Selected
                    {/if}
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <div class="settings-section">
          <h3>Synchronization Self-Healing</h3>
          <p class="section-desc">Troubleshoot sync conflicts, queue blocks, or stale connections.</p>
          
          <div class="maintenance-card">
            <div class="card-info">
              <span class="action-title">Reset Sync Queue</span>
              <span class="action-desc">Purges the persistent transaction sync queue. Safe fallback if you have a failing "poison-pill" action blocking synchronization loops.</span>
            </div>
            <button class="btn btn-danger" on:click={handleResetSyncQueue} disabled={loadingAction !== 'none'}>
              {#if loadingAction === 'reset_queue'}
                <div class="spinner-sm"></div>Resetting...
              {:else}
                Reset Queue
              {/if}
            </button>
          </div>

          <div class="maintenance-card">
            <div class="card-info">
              <span class="action-title">Clear Remote Tracking Cache</span>
              <span class="action-desc">Purges `remote_state.json`. Clears out-of-sync local metadata state caches. On the next sync cycle, a complete comparative comparison with WebDAV is run.</span>
            </div>
            <button class="btn btn-warning" on:click={handleClearSyncCache} disabled={loadingAction !== 'none'}>
              {#if loadingAction === 'clear_cache'}
                <div class="spinner-sm"></div>Clearing...
              {:else}
                Clear Cache
              {/if}
            </button>
          </div>
        </div>
      {/if}
    </div>

    <!-- Bottom Actions Footer -->
    <div class="settings-footer">
      {#if $settingsError}
        <span class="footer-error">{$settingsError}</span>
      {/if}
      <button class="btn btn-secondary" on:click={handleClose} disabled={$loadingSettings}>Cancel</button>
      <button class="btn btn-primary" on:click={handleSave} disabled={$loadingSettings}>
        {#if $loadingSettings}
          <div class="spinner-sm"></div>Saving...
        {:else}
          Apply & Close
        {/if}
      </button>
    </div>
  </div>
{/if}

<style>
  .settings-backdrop {
    position: fixed;
    inset: 0;
    background-color: var(--overlay-bg);
    backdrop-filter: blur(8px);
    z-index: 15000;
  }

  .settings-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 780px;
    height: 520px;
    max-width: 95vw;
    max-height: 90vh;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-xl);
    z-index: 15001;
    display: flex;
    box-shadow: var(--shadow-xl);
    overflow: hidden;
  }

  /* Left Sidebar Navigation */
  .settings-sidebar {
    width: 200px;
    background-color: var(--bg-sidebar);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    padding: 18px 10px;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px 16px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 12px;
  }

  .settings-icon {
    width: 18px;
    height: 18px;
    color: var(--accent);
  }

  .sidebar-header span {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .nav-tab {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    transition: all 0.12s ease;
    font-family: var(--font-sans);
  }

  .nav-tab svg {
    width: 14.5px;
    height: 14.5px;
    flex-shrink: 0;
  }

  .nav-tab:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .nav-tab.active {
    color: var(--accent);
    background-color: var(--accent-muted);
  }

  .sidebar-footer {
    padding: 8px 10px 0;
    border-top: 1px solid var(--border-subtle);
  }

  .version-text {
    font-size: 10.5px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }

  /* Right Settings Content Area */
  .settings-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 24px;
    overflow-y: auto;
    background-color: var(--bg-editor);
  }

  .content-header {
    margin-bottom: 20px;
  }

  .content-header h2 {
    margin: 0 0 4px;
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.3px;
  }

  .content-header p {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 20px;
    flex: 1;
  }

  /* Form Elements */
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-row {
    display: flex;
    gap: 14px;
  }

  .form-row .form-group {
    flex: 1;
  }

  .form-group label,
  .group-label {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .form-group input[type="text"],
  .form-group input[type="password"],
  .form-group input[type="url"],
  .form-group input[type="number"],
  .form-group select {
    background-color: var(--bg-control);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    padding: 8px 12px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-sans);
    outline: none;
    transition: all 0.12s ease;
  }

  .form-group input:focus,
  .form-group select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }

  .input-desc {
    font-size: 11px;
    color: var(--text-tertiary);
    line-height: 1.4;
  }

  /* Theme Radio Cards */
  .theme-options {
    display: flex;
    gap: 12px;
    margin-top: 4px;
  }

  .theme-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    user-select: none;
  }

  .theme-card input {
    display: none;
  }

  .theme-preview {
    width: 100%;
    height: 72px;
    border-radius: var(--radius-md);
    border: 2px solid var(--border-normal);
    display: flex;
    overflow: hidden;
    transition: all 0.15s ease;
    background-color: #000;
  }

  .theme-card:hover .theme-preview {
    border-color: var(--border-strong);
  }

  .theme-card.selected .theme-preview {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }

  .theme-card span {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .theme-card.selected span {
    color: var(--accent);
    font-weight: 600;
  }

  .dark-preview .preview-sidebar { width: 25%; background-color: #161618; border-right: 1px solid #222; }
  .dark-preview .preview-body { flex: 1; background-color: #1c1c1e; }

  .light-preview .preview-sidebar { width: 25%; background-color: #e5e5e7; border-right: 1px solid #d1d1d6; }
  .light-preview .preview-body { flex: 1; background-color: #f2f2f7; }

  .auto-preview .preview-sidebar { width: 25%; background-color: #161618; background: linear-gradient(135deg, #161618 50%, #e5e5e7 50%); }
  .auto-preview .preview-body { flex: 1; background: linear-gradient(135deg, #1c1c1e 50%, #f2f2f7 50%); }

  /* Accent Options */
  .accent-options {
    display: flex;
    gap: 10px;
    margin: 4px 0;
  }

  .accent-dot-wrapper {
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid transparent;
    transition: all 0.12s ease;
  }

  .accent-dot-wrapper input {
    display: none;
  }

  .accent-dot-wrapper:hover {
    border-color: var(--border-strong);
  }

  .accent-dot-wrapper.selected {
    border-color: var(--accent);
  }

  .accent-dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
  }

  /* Field Rows / Toggles */
  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .field-label-desc {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
  }

  .field-label-desc label,
  .toggle-label {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .field-control input[type="number"],
  .field-control select {
    width: 180px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  /* Switch Slider Toggle Custom styling */
  .switch-control {
    position: relative;
    display: inline-block;
    width: 38px;
    height: 22px;
    flex-shrink: 0;
  }

  .switch-control input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .switch-slider {
    position: absolute;
    cursor: pointer;
    inset: 0;
    background-color: var(--border-strong);
    border-radius: 22px;
    transition: .2s ease;
  }

  .switch-slider:before {
    position: absolute;
    content: "";
    height: 18px;
    width: 18px;
    left: 2px;
    bottom: 2px;
    background-color: white;
    border-radius: 50%;
    transition: .2s ease;
    box-shadow: var(--shadow-sm);
  }

  .switch-control input:checked + .switch-slider {
    background-color: var(--accent);
  }

  .switch-control input:checked + .switch-slider:before {
    transform: translateX(16px);
  }

  /* Sync Verification & Alerts */
  .verify-wrapper {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 6px;
  }

  .verify-btn {
    flex-shrink: 0;
  }

  .alert {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-md);
    font-size: 12px;
    font-weight: 500;
    flex: 1;
  }

  .alert svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .alert-success {
    background-color: var(--color-green-muted);
    border: 1px solid rgba(48, 209, 88, 0.25);
    color: var(--color-green);
  }

  .alert-error {
    background-color: var(--color-red-muted);
    border: 1px solid rgba(255, 69, 58, 0.25);
    color: var(--color-red);
  }

  /* Vault Cards styling */
  .vaults-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .vault-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-lg);
    transition: all 0.12s ease;
  }

  .vault-card:hover {
    border-color: var(--border-strong);
    background-color: var(--bg-control-hover);
  }

  .vault-card.active {
    border-color: var(--accent-border);
    background-color: var(--accent-muted);
  }

  .vault-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
  }

  .vault-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .vault-name {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .active-badge {
    background-color: var(--accent);
    color: white;
    font-size: 9.5px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .dummy-badge {
    background-color: var(--border-strong);
    color: var(--text-secondary);
    font-size: 9.5px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-pill);
  }

  .vault-path {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .vault-stats {
    font-size: 11px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dot-divider {
    color: var(--text-disabled);
  }

  /* Bottom Actions Footer */
  .settings-footer {
    position: absolute;
    bottom: 0;
    right: 0;
    left: 200px; /* Aligned with settings sidebar */
    height: 52px;
    padding: 0 24px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    background-color: var(--bg-editor);
  }

  .footer-error {
    margin-right: auto;
    color: var(--color-red);
    font-size: 12px;
    font-weight: 500;
  }

  /* General Button styling */
  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: var(--radius-md);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    font-family: var(--font-sans);
    transition: all 0.12s ease;
    border: 1px solid transparent;
    white-space: nowrap;
    user-select: none;
  }

  .btn-sm {
    padding: 4px 10px;
    font-size: 11.5px;
    border-radius: var(--radius-sm);
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .btn-primary {
    background-color: var(--accent);
    color: #ffffff;
    border-color: var(--accent);
  }

  .btn-primary:hover:not(:disabled) {
    background-color: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .btn-secondary {
    background-color: var(--bg-control);
    color: var(--text-secondary);
    border-color: var(--border-normal);
  }

  .btn-secondary:hover:not(:disabled) {
    background-color: var(--bg-control-hover);
    color: var(--text-primary);
  }

  .btn-ghost {
    background-color: transparent;
    color: var(--accent);
    border-color: var(--accent-border);
  }

  .btn-ghost:hover:not(:disabled) {
    background-color: var(--accent-muted);
  }

  .spinner-sm {
    width: 12px;
    height: 12px;
    border: 1.5px solid rgba(255, 255, 255, 0.25);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  /* Maintenance Page Custom Styles */
  .nav-tab-maintenance svg {
    color: #fbbf24;
  }
  
  .maintenance-card {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    background-color: var(--bg-surface-elevated, #1c1c1e);
    border: 1px solid var(--border-normal, #2c2c2e);
    border-radius: var(--radius-md, 8px);
    padding: 14px 16px;
    margin-bottom: 12px;
    transition: border-color 0.2s, background-color 0.2s;
  }
  
  .maintenance-card:hover {
    border-color: var(--border-focus, #3a3a3c);
    background-color: var(--bg-surface-hover, #242426);
  }
  
  .card-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  
  .action-title {
    font-weight: 500;
    font-size: 13.5px;
    color: var(--text-primary);
  }
  
  .action-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  
  .section-desc {
    font-size: 12.5px;
    color: var(--text-tertiary, #8e8e93);
    margin-top: -6px;
    margin-bottom: 16px;
  }
  
  .btn-warning {
    background-color: rgba(245, 158, 11, 0.12) !important;
    color: #fbbf24 !important;
    border: 1.2px solid rgba(245, 158, 11, 0.35) !important;
  }
  
  .btn-warning:hover:not(:disabled) {
    background-color: rgba(245, 158, 11, 0.22) !important;
    border-color: rgba(245, 158, 11, 0.55) !important;
  }
  
  .btn-danger {
    background-color: rgba(239, 68, 68, 0.12) !important;
    color: #f87171 !important;
    border: 1.2px solid rgba(239, 68, 68, 0.35) !important;
  }
  
  .btn-danger:hover:not(:disabled) {
    background-color: rgba(239, 68, 68, 0.22) !important;
    border-color: rgba(239, 68, 68, 0.55) !important;
  }
  
  .maintenance-alert {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: var(--radius-md, 8px);
    margin-bottom: 20px;
    font-size: 12.5px;
    position: relative;
  }
  
  .maintenance-alert svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }
  
  .alert-success {
    background-color: rgba(16, 185, 129, 0.12);
    border: 1px solid rgba(16, 185, 129, 0.25);
    color: #34d399;
  }
  
  .alert-danger {
    background-color: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.25);
    color: #f87171;
  }
  
  .close-alert {
    background: none;
    border: none;
    color: currentColor;
    font-size: 18px;
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.2s;
  }
  
  .close-alert:hover {
    opacity: 1;
  }
  
  .orphaned-box {
    background-color: var(--bg-surface-elevated, #1c1c1e);
    border: 1px solid var(--border-normal, #2c2c2e);
    border-radius: var(--radius-md, 8px);
    margin-top: -4px;
    margin-bottom: 20px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  
  .box-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-normal, #2c2c2e);
    padding-bottom: 10px;
  }
  
  .box-title {
    font-weight: 500;
    font-size: 13px;
    color: var(--text-primary);
  }
  
  .box-total-size {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .empty-orphaned {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
  }
  
  .success-indicator {
    font-size: 28px;
  }
  
  .empty-text {
    font-size: 12.5px;
    color: var(--text-secondary);
  }
  
  .orphaned-list {
    max-height: 180px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-right: 4px;
  }
  
  .orphaned-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background-color: var(--bg-main, #141415);
    border: 1.2px solid var(--border-normal, #2c2c2e);
    border-radius: var(--radius-sm, 6px);
    cursor: pointer;
    transition: background-color 0.2s, border-color 0.2s;
  }
  
  .orphaned-item:hover {
    background-color: var(--bg-control-hover, #1e1e1f);
    border-color: var(--border-focus, #3a3a3c);
  }
  
  .orphaned-item input[type="checkbox"] {
    accent-color: var(--accent);
    cursor: pointer;
  }
  
  .item-details {
    display: flex;
    justify-content: space-between;
    flex: 1;
    font-size: 12px;
  }
  
  .item-name {
    color: var(--text-primary);
    word-break: break-all;
    padding-right: 12px;
  }
  
  .item-size {
    color: var(--text-secondary);
    white-space: nowrap;
  }
  
  .orphaned-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--border-normal, #2c2c2e);
    padding-top: 12px;
    margin-top: 4px;
  }
  
  .selected-count {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .spinner-sm {
    display: inline-block;
    vertical-align: middle;
    margin-right: 6px;
  }
</style>
