<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, scale, slide } from 'svelte/transition';
  import { appConfig, saveSettings, loadSettings, loadingSettings, settingsError } from '../../stores/settings';
  import { notesList } from '../../stores/notes';
  import {
    validateSyncConfig,
    rebuildDatabaseCache,
    vacuumDatabaseCache,
    getOrphanedAttachments,
    deleteOrphanedAttachments,
    clearSyncQueue,
    clearSyncCache,
    getDuplicateNotes,
    deleteDuplicateNoteFile,
    getConflictNote,
    getOrphanedRemnants,
    deleteOrphanedRemnants,
    deleteOrphanedFile
  } from '../../services/ipc';
  import type { OrphanedAttachment, DuplicateNoteGroup, OrphanedRemnants } from '../../services/ipc';
  import type { AppConfig } from '../../types';

  let {
    isOpen = $bindable(false),
    activeTab = $bindable('appearance'),
    onclose
  } = $props<{
    isOpen?: boolean;
    activeTab?: 'appearance' | 'editor' | 'sync' | 'history' | 'vault' | 'maintenance' | 'templates';
    onclose?: () => void;
  }>();

  // Reactive state fields for config
  let appearanceTheme = $state('dark');
  let appearanceAccentColor = $state('blue');
  let editorFontSize = $state(14);
  let editorTypography = $state('sans');
  let editorShowWordCount = $state(true);
  let editorAutoSaveDelayMs = $state(1500);
  let editorDefaultDailyTemplate = $state<string>('');
  let syncWebdavUrl = $state('');
  let syncWebdavUsername = $state('');
  let syncIntervalSecs = $state(300);
  let historyRetentionDays = $state(30);
  let historyMaxSnapshots = $state(50);
  let historyEmptyTrashDays = $state(30);
  let historySnapshotIntervalMins = $state(5);

  const templateNotes = $derived(
    $notesList.filter(n => n.file_path.startsWith('.templates/'))
  );

  // Sync state
  let webdavPassword = $state('');
  let validationStatus = $state<'idle' | 'testing' | 'success' | 'error'>('idle');
  let validationErrorMessage = $state('');

  // Maintenance state
  let loadingAction = $state<'none' | 'rebuild_db' | 'vacuum_db' | 'scan_attachments' | 'delete_attachments' | 'reset_queue' | 'clear_cache' | 'scan_duplicates' | 'delete_duplicate' | 'scan_remnants' | 'delete_remnants'>('none');
  let orphanedAttachments = $state<OrphanedAttachment[]>([]);
  let selectedAttachments = $state<string[]>([]);
  let scannedAttachments = $state(false);
  let scannedRemnants = $state(false);
  let orphanedRemnants = $state<OrphanedRemnants | null>(null);

  let duplicateNotes = $state<DuplicateNoteGroup[]>([]);
  let scannedDuplicates = $state(false);
  let previewNoteContent = $state<string | null>(null);
  let previewingFile = $state<string | null>(null);
  let previewingTitle = $state<string | null>(null);
  let loadingPreview = $state(false);

  let maintenanceSuccessMsg = $state('');
  let maintenanceErrorMsg = $state('');

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

  async function handleScanDuplicates() {
    clearMaintenanceMessages();
    loadingAction = 'scan_duplicates';
    try {
      duplicateNotes = await getDuplicateNotes();
      scannedDuplicates = true;
      if (duplicateNotes.length === 0) {
        maintenanceSuccessMsg = 'Harika! Vault klasöründe hiçbir mükerrer not bulunamadı.';
      }
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Mükerrer not taraması başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleDeleteDuplicate(relativePath: string) {
    if (!confirm('Bu not kopyasını fiziksel olarak diskten kalıcı olarak silmek istediğinizden emin misiniz? Bu işlem geri alınamaz.')) return;
    
    clearMaintenanceMessages();
    loadingAction = 'delete_duplicate';
    try {
      await deleteDuplicateNoteFile(relativePath);
      maintenanceSuccessMsg = 'Mükerrer not kopyası başarıyla silindi!';
      
      // Update local duplicateNotes state list
      duplicateNotes = duplicateNotes.map(group => {
        return {
          ...group,
          files: group.files.filter(f => f.relative_path !== relativePath)
        };
      }).filter(group => group.files.length > 1);
      
      // Close preview if the previewed file is deleted
      if (previewingFile === relativePath) {
        closePreview();
      }
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Dosya silme işlemi başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleScanRemnants() {
    clearMaintenanceMessages();
    loadingAction = 'scan_remnants';
    try {
      orphanedRemnants = await getOrphanedRemnants();
      scannedRemnants = true;
      const count = orphanedRemnants?.files?.length || 0;
      if (count === 0) {
        maintenanceSuccessMsg = 'Harika! Vault klasöründe hiçbir sahipsiz geçmiş veya çakışma kalıntısı bulunamadı.';
      }
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Kalıntı taraması başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleDeleteSelectedRemnants() {
    if (!orphanedRemnants) return;
    const count = orphanedRemnants.files?.length || 0;
    if (count === 0) return;
    if (!confirm(`Seçilen ${count} adet sahipsiz kalıntıyı (geçmiş ve çakışma dosyaları) kalıcı olarak silmek istediğinizden emin misiniz? Bu işlem geri alınamaz.`)) return;

    clearMaintenanceMessages();
    loadingAction = 'delete_remnants';
    try {
      await deleteOrphanedRemnants(orphanedRemnants);
      maintenanceSuccessMsg = `Tüm sahipsiz geçmiş sürümleri ve çakışma dosyaları başarıyla temizlendi, depolama alanı geri kazanıldı!`;
      orphanedRemnants = { files: [], total_recovered_bytes: 0 };
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Kalıntı temizleme başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handleDeleteOrphanedFile(relativePath: string) {
    if (!confirm('Bu kalıntı dosyasını diskten kalıcı olarak silmek istediğinizden emin misiniz? Bu işlem geri alınamaz.')) return;
    
    clearMaintenanceMessages();
    loadingAction = 'delete_remnants';
    try {
      await deleteOrphanedFile(relativePath);
      maintenanceSuccessMsg = 'Kalıntı dosya başarıyla silindi!';
      
      if (orphanedRemnants) {
        const fileToDelete = orphanedRemnants.files.find(f => f.relative_path === relativePath);
        const size = fileToDelete ? fileToDelete.size_bytes : 0;
        
        orphanedRemnants = {
          files: orphanedRemnants.files.filter(f => f.relative_path !== relativePath),
          total_recovered_bytes: Math.max(0, orphanedRemnants.total_recovered_bytes - size)
        };
      }
      
      if (previewingFile === relativePath) {
        closePreview();
      }
    } catch (e: any) {
      maintenanceErrorMsg = e.message || 'Kalıntı silme başarısız';
    } finally {
      loadingAction = 'none';
    }
  }

  async function handlePreviewNote(relativePath: string, title: string) {
    loadingPreview = true;
    previewingFile = relativePath;
    previewingTitle = title;
    previewNoteContent = null;
    try {
      // Re-use existing getConflictNote FFI command to read note details safely
      const note = await getConflictNote(relativePath);
      previewNoteContent = note.body;
    } catch (e: any) {
      previewNoteContent = 'İçerik yüklenemedi: ' + (e.message || e.toString());
    } finally {
      loadingPreview = false;
    }
  }

  function closePreview() {
    previewingFile = null;
    previewingTitle = null;
    previewNoteContent = null;
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
      appearanceTheme = config.appearance.theme;
      appearanceAccentColor = config.appearance.accent_color;
      editorFontSize = config.editor.font_size;
      editorTypography = config.editor.typography;
      editorShowWordCount = config.editor.show_word_count;
      editorAutoSaveDelayMs = config.editor.auto_save_delay_ms;
      editorDefaultDailyTemplate = config.editor.default_daily_template || '';
      syncWebdavUrl = config.sync.webdav_url;
      syncWebdavUsername = config.sync.webdav_username;
      webdavPassword = config.sync.webdav_password || '';
      syncIntervalSecs = config.sync.interval_secs;
      historyRetentionDays = config.history.retention_days;
      historyMaxSnapshots = config.history.max_snapshots_per_note;
      historyEmptyTrashDays = config.history.empty_trash_after_days;
      historySnapshotIntervalMins = config.history.snapshot_interval_mins ?? 5;
    }
  }

  async function handleVerifySync() {
    validationStatus = 'testing';
    validationErrorMessage = '';
    try {
      await validateSyncConfig({
        webdav_url: syncWebdavUrl,
        webdav_username: syncWebdavUsername,
        webdav_password: webdavPassword || undefined,
        interval_secs: syncIntervalSecs
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
      const config: AppConfig = {
        appearance: { theme: appearanceTheme, accent_color: appearanceAccentColor },
        editor: {
          font_size: editorFontSize,
          typography: editorTypography,
          show_word_count: editorShowWordCount,
          auto_save_delay_ms: editorAutoSaveDelayMs,
          default_daily_template: editorDefaultDailyTemplate || null
        },
        sync: {
          webdav_url: syncWebdavUrl,
          webdav_username: syncWebdavUsername,
          webdav_password: webdavPassword || undefined,
          interval_secs: syncIntervalSecs
        },
        history: {
          retention_days: historyRetentionDays,
          max_snapshots_per_note: historyMaxSnapshots,
          empty_trash_after_days: historyEmptyTrashDays,
          snapshot_interval_mins: historySnapshotIntervalMins
        }
      };
      await saveSettings(config);
      
      // Apply theme changes to document attribute if needed
      document.documentElement.setAttribute('data-theme', appearanceTheme);
      
      // Close callback
      onclose?.();
    } catch (e) {
      console.error('Failed to save settings:', e);
    }
  }

  function handleClose() {
    onclose?.();
  }
</script>

{#if isOpen}
  <div class="settings-backdrop" transition:fade={{ duration: 150 }} onclick={handleClose} role="presentation"></div>
  
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
        <button class="nav-tab" class:active={activeTab === 'appearance'} onclick={() => activeTab = 'appearance'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2H4a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4a2 2 0 0 0-2-2z"/>
            <circle cx="6" cy="6" r="1.5"/>
            <path d="m14 10-2.5-2.5a1 1 0 0 0-1.4 0L6 11.6"/>
          </svg>
          Appearance
        </button>

        <button class="nav-tab" class:active={activeTab === 'editor'} onclick={() => activeTab = 'editor'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11.5 2.5 13.5 4.5 5 13H3v-2L11.5 2.5z"/>
            <line x1="8" y1="12" x2="13" y2="12"/>
          </svg>
          Editor
        </button>

        <button class="nav-tab" class:active={activeTab === 'sync'} onclick={() => activeTab = 'sync'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M13 10a4 4 0 0 0-4-4H6a4 4 0 0 0 0 8h3"/>
            <polyline points="10,7 13,10 10,13"/>
          </svg>
          Sync & Cloud
        </button>

        <button class="nav-tab" class:active={activeTab === 'history'} onclick={() => activeTab = 'history'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="8" cy="8" r="6.5"/>
            <polyline points="8,4.5 8,8 10.5,10"/>
          </svg>
          History & Backup
        </button>

        <button class="nav-tab" class:active={activeTab === 'vault'} onclick={() => activeTab = 'vault'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d="M2 2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2z"/>
            <line x1="5" y1="4" x2="11" y2="4"/>
          </svg>
          Vaults
        </button>

        <button class="nav-tab" class:active={activeTab === 'templates'} onclick={() => activeTab = 'templates'}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="2" width="12" height="12" rx="2"/>
            <line x1="6" y1="6" x2="10" y2="6"/>
            <line x1="6" y1="10" x2="10" y2="10"/>
          </svg>
          Templates
        </button>

        <button class="nav-tab nav-tab-maintenance" class:active={activeTab === 'maintenance'} onclick={() => { activeTab = 'maintenance'; clearMaintenanceMessages(); }}>
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
              <label class="theme-card" class:selected={appearanceTheme === 'dark'}>
                <input type="radio" bind:group={appearanceTheme} value="dark" />
                <div class="theme-preview dark-preview">
                  <div class="preview-sidebar"></div>
                  <div class="preview-body"></div>
                </div>
                <span>Dark</span>
              </label>

              <label class="theme-card" class:selected={appearanceTheme === 'light'}>
                <input type="radio" bind:group={appearanceTheme} value="light" />
                <div class="theme-preview light-preview">
                  <div class="preview-sidebar"></div>
                  <div class="preview-body"></div>
                </div>
                <span>Light (Dummy)</span>
              </label>

              <label class="theme-card" class:selected={appearanceTheme === 'auto'}>
                <input type="radio" bind:group={appearanceTheme} value="auto" />
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
                <label class="accent-dot-wrapper" class:selected={appearanceAccentColor === color}>
                  <input type="radio" bind:group={appearanceAccentColor} value={color} />
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
                <select id="auto-save-delay" bind:value={editorAutoSaveDelayMs}>
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
                <input id="editor-font-size" type="number" min="10" max="24" bind:value={editorFontSize} />
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
                <select id="editor-typography" bind:value={editorTypography}>
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
                <input type="checkbox" bind:checked={editorShowWordCount} />
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
            <input id="webdav-url" type="url" placeholder="https://example.com/dav/" bind:value={syncWebdavUrl} />
            <span class="input-desc">Root folder WebDAV link from cloud provider (e.g. InfiniCLOUD, Nextcloud).</span>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="webdav-user">Username</label>
              <input id="webdav-user" type="text" placeholder="username" bind:value={syncWebdavUsername} />
            </div>
            <div class="form-group">
              <label for="webdav-pass">App Password</label>
              <input id="webdav-pass" type="password" placeholder="••••••••" bind:value={webdavPassword} />
            </div>
          </div>

          <div class="form-group">
            <label for="sync-interval">Sync Frequency</label>
            <select id="sync-interval" bind:value={syncIntervalSecs}>
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
              onclick={handleVerifySync}
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
                <input id="max-snapshots" type="number" min="5" max="500" bind:value={historyMaxSnapshots} />
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
                <input id="retention-days" type="number" min="1" max="365" bind:value={historyRetentionDays} />
              </div>
            </div>
          </div>

          <div class="form-group">
            <div class="field-row">
              <div class="field-label-desc">
                <label for="snapshot-interval">Snapshot Interval (Minutes)</label>
                <span class="input-desc">Minutes of typing before generating a new history snapshot version.</span>
              </div>
              <div class="field-control">
                <input id="snapshot-interval" type="number" min="1" max="60" bind:value={historySnapshotIntervalMins} />
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
                <input id="empty-trash-days" type="number" min="1" max="180" bind:value={historyEmptyTrashDays} />
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
          <div class="alert alert-success" transition:fade style="margin-bottom: 20px;">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width: 16px; height: 16px; flex-shrink: 0;">
              <polyline points="2,8 6,12 14,4"/>
            </svg>
            <span>{maintenanceSuccessMsg}</span>
            <button class="close-alert" onclick={() => maintenanceSuccessMsg = ''}>&times;</button>
          </div>
        {/if}

        {#if maintenanceErrorMsg}
          <div class="alert alert-error" transition:fade style="margin-bottom: 20px;">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" style="width: 16px; height: 16px; flex-shrink: 0;">
              <circle cx="8" cy="8" r="6"/>
              <line x1="8" y1="5" x2="8" y2="8"/>
              <line x1="8" y1="11" x2="8" y2="11"/>
            </svg>
            <span>{maintenanceErrorMsg}</span>
            <button class="close-alert" onclick={() => maintenanceErrorMsg = ''}>&times;</button>
          </div>
        {/if}

        <div class="settings-section">
          <!-- Section 1: Database Health -->
          <div class="maintenance-group">
            <div class="group-title-row">
              <span class="group-title">Database Administration</span>
              <span class="group-subtitle">Maintain search performance and clean index metadata</span>
            </div>
            
            <div class="maintenance-card">
              <div class="card-icon-container">
                <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                  <path d="M3 5V19A9 3 0 0 0 21 19V5"></path>
                  <path d="M3 12A9 3 0 0 0 21 12"></path>
                </svg>
              </div>
              <div class="card-info">
                <span class="action-title">Rebuild Database Cache</span>
                <span class="action-desc">Fully re-scans vault note files and rebuilds the SQLite search and tag index from scratch. Useful if some notes are missing from list or search.</span>
              </div>
              <button class="btn btn-warning" onclick={handleRebuildDatabase} disabled={loadingAction !== 'none'}>
                {#if loadingAction === 'rebuild_db'}
                  <div class="spinner-sm"></div>Processing...
                {:else}
                  Rebuild Cache
                {/if}
              </button>
            </div>

            <div class="maintenance-card">
              <div class="card-icon-container">
                <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="m21 16-4 4-4-4"></path>
                  <path d="M17 20V4"></path>
                  <path d="m3 8 4-4 4 4"></path>
                  <path d="M7 4v16"></path>
                </svg>
              </div>
              <div class="card-info">
                <span class="action-title">Vacuum Database</span>
                <span class="action-desc">Defragments the database file, cleans unused cache spaces, and optimizes internal query performance. Safe to run anytime.</span>
              </div>
              <button class="btn btn-secondary" onclick={handleVacuumDatabase} disabled={loadingAction !== 'none'}>
                {#if loadingAction === 'vacuum_db'}
                  <div class="spinner-sm"></div>Processing...
                {:else}
                  Vacuum DB
                {/if}
              </button>
            </div>
          </div>

          <!-- Section 2: Vault Cleanup -->
          <div class="maintenance-group">
            <div class="group-title-row">
              <span class="group-title">Vault Diagnostics & Storage Cleanup</span>
              <span class="group-subtitle">Reclaim local storage space by purging unreferenced media or duplicate files</span>
            </div>

            <!-- Attachments Card -->
            <div class="maintenance-card-group">
              <div class="maintenance-card">
                <div class="card-icon-container">
                  <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48"></path>
                  </svg>
                </div>
                <div class="card-info">
                  <span class="action-title">Scan Orphaned Attachments</span>
                  <span class="action-desc">Scans `.noda/attachments/` to identify images and files that are no longer linked or used inside any active note.</span>
                </div>
                <button class="btn btn-primary" onclick={handleScanAttachments} disabled={loadingAction !== 'none'}>
                  {#if loadingAction === 'scan_attachments'}
                    <div class="spinner-sm"></div>Scanning...
                  {:else}
                    Scan Files
                  {/if}
                </button>
              </div>

              {#if scannedAttachments}
                <div class="orphaned-box {orphanedAttachments.length === 0 ? 'box-healthy' : 'box-action'}" transition:slide>
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
                      <button class="btn btn-danger btn-sm" onclick={handleDeleteSelectedAttachments} disabled={selectedAttachments.length === 0 || loadingAction !== 'none'}>
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

            <!-- Duplicate Notes Card -->
            <div class="maintenance-card-group">
              <div class="maintenance-card">
                <div class="card-icon-container">
                  <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                  </svg>
                </div>
                <div class="card-info">
                  <span class="action-title">Scan Duplicate Notes</span>
                  <span class="action-desc">Scans the entire vault recursively to locate any files sharing identical internal note IDs.</span>
                </div>
                <button class="btn btn-primary" onclick={handleScanDuplicates} disabled={loadingAction !== 'none'}>
                  {#if loadingAction === 'scan_duplicates'}
                    <div class="spinner-sm"></div>Scanning...
                  {:else}
                    Scan Duplicates
                  {/if}
                </button>
              </div>

              {#if scannedDuplicates}
                <div class="orphaned-box {duplicateNotes.length === 0 ? 'box-healthy' : 'box-action'}" transition:slide>
                  {#if duplicateNotes.length === 0}
                    <div class="empty-orphaned">
                      <span class="success-indicator">🎉</span>
                      <span class="empty-text">Harika! Vault klasöründe hiçbir mükerrer not bulunamadı.</span>
                    </div>
                  {:else}
                    <div class="box-header">
                      <span class="box-title">Found {duplicateNotes.length} Duplicate Note Groups</span>
                    </div>

                    <div class="duplicate-groups-list scrollbar-thin">
                      {#each duplicateNotes as group}
                        <div class="duplicate-group-card">
                          <div class="group-header">
                            <span class="group-note-title">📝 {group.title || 'Untitled'}</span>
                            <span class="group-note-id">ID: {group.note_id}</span>
                          </div>
                          <div class="group-files-list">
                            {#each group.files as file}
                              <div class="duplicate-file-item" class:previewing={previewingFile === file.relative_path}>
                                <div class="file-info-col" onclick={() => handlePreviewNote(file.relative_path, group.title)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && handlePreviewNote(file.relative_path, group.title)}>
                                  <span class="file-path">{file.relative_path}</span>
                                  <span class="file-meta">
                                    Size: {formatBytes(file.size_bytes)} • Modified: {new Date(file.last_modified).toLocaleString()}
                                  </span>
                                </div>
                                <button class="btn btn-danger btn-xs" onclick={() => handleDeleteDuplicate(file.relative_path)} disabled={loadingAction !== 'none'}>
                                  Delete
                                </button>
                              </div>
                            {/each}
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>

            <!-- Remnants Card -->
            <div class="maintenance-card-group">
              <div class="maintenance-card">
                <div class="card-icon-container">
                  <svg class="card-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"></path>
                    <path d="M3 3v5h5"></path>
                    <path d="M12 7v5l4 2"></path>
                  </svg>
                </div>
                <div class="card-info">
                  <span class="action-title">Scan Orphaned Remnants</span>
                  <span class="action-desc">Scans `.noda/history/` and `.noda/conflicts/` directories to identify metadata that no longer belongs to any active or trashed note.</span>
                </div>
                <button class="btn btn-primary" onclick={handleScanRemnants} disabled={loadingAction !== 'none'}>
                  {#if loadingAction === 'scan_remnants'}
                    <div class="spinner-sm"></div>Scanning...
                  {:else}
                    Scan Remnants
                  {/if}
                </button>
              </div>

              {#if scannedRemnants && orphanedRemnants}
                <div class="orphaned-box {orphanedRemnants.files.length === 0 ? 'box-healthy' : 'box-action'}" transition:slide>
                  {#if orphanedRemnants.files.length === 0}
                    <div class="empty-orphaned">
                      <span class="success-indicator">🎉</span>
                      <span class="empty-text">Harika! Vault klasöründe hiçbir sahipsiz geçmiş veya çakışma kalıntısı bulunamadı.</span>
                    </div>
                  {:else}
                    <div class="box-header">
                      <span class="box-title">Bulunan Sahipsiz Kalıntılar ({orphanedRemnants.files.length} Dosya)</span>
                      <span class="box-total-size">Kazanılacak Alan: {formatBytes(orphanedRemnants.total_recovered_bytes)}</span>
                    </div>

                    <div class="duplicate-groups-list scrollbar-thin">
                      <div class="duplicate-group-card" style="border: none; padding: 0; background: transparent; margin-bottom: 0; box-shadow: none;">
                        <div class="group-files-list">
                          {#each orphanedRemnants.files as file}
                            <div class="duplicate-file-item" class:previewing={previewingFile === file.relative_path}>
                              <div class="file-info-col" onclick={() => handlePreviewNote(file.relative_path, file.title)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && handlePreviewNote(file.relative_path, file.title)}>
                                <span class="file-path" style="display: flex; align-items: center; gap: 6px;">
                                  <span>{file.file_type === 'history' ? '📁' : '📄'}</span>
                                  <span style="font-weight: 500;">{file.title}</span>
                                </span>
                                <span class="file-meta" style="margin-top: 2px;">
                                  Yol: {file.relative_path} • Boyut: {formatBytes(file.size_bytes)} • Değiştirilme: {new Date(file.last_modified).toLocaleString()}
                                </span>
                              </div>
                              <button class="btn btn-danger btn-xs" onclick={() => handleDeleteOrphanedFile(file.relative_path)} disabled={loadingAction !== 'none'}>
                                Sil
                              </button>
                            </div>
                          {/each}
                        </div>
                      </div>
                    </div>
                    
                    <div class="orphaned-actions" style="margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border-subtle);">
                      <span class="selected-count">{orphanedRemnants.files.length} dosya kalıcı olarak silinecek</span>
                      <button class="btn btn-danger btn-sm" onclick={handleDeleteSelectedRemnants} disabled={loadingAction !== 'none'}>
                        {#if loadingAction === 'delete_remnants'}
                          <div class="spinner-sm"></div>Temizleniyor...
                        {:else}
                          Tüm Kalıntıları Temizle
                        {/if}
                      </button>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>

            <!-- Preview Drawer Component (Unified for duplicates and remnants) -->
            {#if previewingFile}
              <div class="preview-drawer" transition:slide={{ axis: 'x', duration: 200 }}>
                <div class="drawer-header">
                  <div class="drawer-title-row">
                    <span class="drawer-icon">📝</span>
                    <h4>Preview: {previewingTitle || 'Untitled'}</h4>
                  </div>
                  <button class="close-btn" onclick={closePreview}>&times;</button>
                </div>
                <div class="drawer-body scrollbar-thin">
                  <span class="drawer-path-sub">{previewingFile}</span>
                  {#if loadingPreview}
                    <div class="preview-loading">
                      <div class="spinner-sm"></div> Yükleniyor...
                    </div>
                  {:else}
                    <pre class="preview-content">{previewNoteContent || '(Boş Not)'}</pre>
                  {/if}
                </div>
              </div>
            {/if}
          </div>

          <!-- Section 3: Cloud Synchronization Sync Self Healing -->
          <div class="maintenance-group">
            <div class="group-title-row">
              <span class="group-title">Synchronization Self-Healing</span>
              <span class="group-subtitle">Troubleshoot sync conflicts, queue blocks, or stale connections</span>
            </div>

            <div class="maintenance-card">
              <div class="card-icon-container">
                <svg class="card-icon card-icon-danger" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"></path>
                  <line x1="12" y1="9" x2="12" y2="13"></line>
                  <line x1="12" y1="17" x2="12.01" y2="17"></line>
                </svg>
              </div>
              <div class="card-info">
                <span class="action-title">Reset Sync Queue</span>
                <span class="action-desc">Purges the persistent transaction sync queue. Safe fallback if you have a failing "poison-pill" action blocking synchronization loops.</span>
              </div>
              <button class="btn btn-danger" onclick={handleResetSyncQueue} disabled={loadingAction !== 'none'}>
                {#if loadingAction === 'reset_queue'}
                  <div class="spinner-sm"></div>Resetting...
                {:else}
                  Reset Queue
                {/if}
              </button>
            </div>

            <div class="maintenance-card">
              <div class="card-icon-container">
                <svg class="card-icon card-icon-warning" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M17.5 19A3.5 3.5 0 0 0 21 15.5c0-2.79-2.54-4.5-5-4.5-.42-1.89-1.74-3.5-3.5-3.5C10 7.5 7.5 10 7.5 12.5c0 .35.03.68.08 1M5 19H3a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2h4"></path>
                  <path d="M22 19H12a2 2 0 0 1-2-2v-4a2 2 0 0 1 2-2h4"></path>
                </svg>
              </div>
              <div class="card-info">
                <span class="action-title">Clear Remote Tracking Cache</span>
                <span class="action-desc">Purges `remote_state.json`. Clears out-of-sync local metadata state caches. On the next sync cycle, a complete comparative comparison with WebDAV is run.</span>
              </div>
              <button class="btn btn-warning" onclick={handleClearSyncCache} disabled={loadingAction !== 'none'}>
                {#if loadingAction === 'clear_cache'}
                  <div class="spinner-sm"></div>Clearing...
                {:else}
                  Clear Cache
                {/if}
              </button>
            </div>
          </div>
        </div>
      {:else if activeTab === 'templates'}
        <div class="content-header">
          <h2>Templates</h2>
          <p>Configure defaults and select template files for your notes.</p>
        </div>

        <div class="settings-section">
          <div class="form-group">
            <label for="default-daily-template">Default Daily Note Template</label>
            <select id="default-daily-template" bind:value={editorDefaultDailyTemplate} class="select-control">
              <option value="">None (Empty Note)</option>
              {#each templateNotes as template}
                <option value={template.id}>{template.title} ({template.file_path})</option>
              {/each}
            </select>
            <span class="input-desc">
              Select the default template file from the <code>.templates/</code> directory to be used for your daily notes.
            </span>
          </div>

          {#if templateNotes.length === 0}
            <div class="alert alert-info">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <circle cx="8" cy="8" r="6"/>
                <line x1="8" y1="5" x2="8" y2="8"/>
                <line x1="8" y1="11" x2="8" y2="11"/>
              </svg>
              <span>No templates found in the <code>.templates/</code> folder. Create a folder named <code>.templates</code> and add markdown notes to define templates.</span>
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Bottom Actions Footer -->
    <div class="settings-footer">
      {#if $settingsError}
        <span class="footer-error">{$settingsError}</span>
      {/if}
      <button class="btn btn-secondary" onclick={handleClose} disabled={$loadingSettings}>Cancel</button>
      <button class="btn btn-primary" onclick={handleSave} disabled={$loadingSettings}>
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
    margin-bottom: 52px;
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
    color: var(--color-orange, #ffd60a);
  }
  
  .maintenance-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 24px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .maintenance-group:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  .group-title-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 12px;
  }

  .group-title {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .group-subtitle {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .maintenance-card-group {
    display: flex;
    flex-direction: column;
    margin-bottom: 4px;
  }
  
  .maintenance-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    padding: 12px 16px;
    transition: all 0.15s ease;
  }
  
  .maintenance-card:hover {
    border-color: var(--border-strong);
    background-color: var(--bg-elevated-2);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .card-icon-container {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background-color: rgba(255, 255, 255, 0.04);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    border: 1px solid var(--border-subtle);
    transition: all 0.15s ease;
  }

  .maintenance-card:hover .card-icon-container {
    background-color: rgba(255, 255, 255, 0.08);
    border-color: var(--border-normal);
  }

  .card-icon {
    width: 16px;
    height: 16px;
    color: var(--text-secondary);
    transition: color 0.15s ease;
  }

  .maintenance-card:hover .card-icon {
    color: var(--accent);
  }

  .card-icon-danger {
    color: var(--color-red) !important;
    opacity: 0.85;
  }

  .maintenance-card:hover .card-icon-danger {
    color: var(--color-red) !important;
    opacity: 1;
  }

  .card-icon-warning {
    color: var(--color-orange) !important;
    opacity: 0.85;
  }

  .maintenance-card:hover .card-icon-warning {
    color: var(--color-orange) !important;
    opacity: 1;
  }
  
  .card-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  
  .action-title {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
  }
  
  .action-desc {
    font-size: 11.5px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  
  .btn-warning {
    background-color: rgba(245, 158, 11, 0.08) !important;
    color: var(--color-orange, #ff9f0a) !important;
    border: 1px solid rgba(245, 158, 11, 0.25) !important;
  }
  
  .btn-warning:hover:not(:disabled) {
    background-color: rgba(245, 158, 11, 0.16) !important;
    border-color: rgba(245, 158, 11, 0.4) !important;
  }
  
  .btn-danger {
    background-color: rgba(239, 68, 68, 0.08) !important;
    color: var(--color-red, #ff453a) !important;
    border: 1px solid rgba(239, 68, 68, 0.25) !important;
  }
  
  .btn-danger:hover:not(:disabled) {
    background-color: rgba(239, 68, 68, 0.16) !important;
    border-color: rgba(239, 68, 68, 0.4) !important;
  }
  
  .close-alert {
    background: none;
    border: none;
    color: currentColor;
    font-size: 18px;
    position: relative;
    margin-left: auto;
    /*right: 35px;*/
    /*top: 50%;*/
    /*transform: translateY(-50%);*/ 
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.2s;
  }
  
  .close-alert:hover {
    opacity: 1;
  }
  
  .orphaned-box {
    background-color: rgba(255, 255, 255, 0.015);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    margin-top: 4px;
    margin-bottom: 12px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: all 0.2s ease;
  }

  .box-healthy {
    border-left: 3px solid var(--color-green) !important;
    background-color: rgba(48, 209, 88, 0.03);
  }

  .box-action {
    border-left: 3px solid var(--accent) !important;
    background-color: rgba(10, 132, 255, 0.03);
  }
  
  .box-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 8px;
  }
  
  .box-title {
    font-weight: 600;
    font-size: 12.5px;
    color: var(--text-primary);
  }
  
  .box-total-size {
    font-size: 11.5px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }
  
  .empty-orphaned {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 16px;
    text-align: center;
  }
  
  .success-indicator {
    font-size: 24px;
  }
  
  .empty-text {
    font-size: 12px;
    color: var(--text-secondary);
  }
  
  .orphaned-list {
    max-height: 180px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-right: 4px;
  }
  
  .orphaned-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.12s ease;
  }
  
  .orphaned-item:hover {
    background-color: var(--bg-control-hover);
    border-color: var(--border-normal);
  }
  
  .orphaned-item input[type="checkbox"] {
    accent-color: var(--accent);
    cursor: pointer;
  }
  
  .item-details {
    display: flex;
    justify-content: space-between;
    flex: 1;
    font-size: 11.5px;
  }
  
  .item-name {
    color: var(--text-primary);
    word-break: break-all;
    padding-right: 12px;
  }
  
  .item-size {
    color: var(--text-secondary);
    white-space: nowrap;
    font-family: var(--font-mono);
  }
  
  .orphaned-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid var(--border-subtle);
    padding-top: 10px;
    margin-top: 4px;
  }
  
  .selected-count {
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  
  .spinner-sm {
    display: inline-block;
    vertical-align: middle;
    margin-right: 6px;
  }
 
  .duplicate-groups-list {
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-right: 4px;
  }
 
  .duplicate-group-card {
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    overflow: hidden;
    transition: border-color 0.15s ease;
  }

  .duplicate-group-card:hover {
    border-color: var(--border-strong);
  }
 
  .group-header {
    background-color: rgba(255, 255, 255, 0.03);
    padding: 8px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
  }
 
  .group-note-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
  }
 
  .group-note-id {
    font-size: 9.5px;
    font-family: var(--font-mono);
    color: var(--text-tertiary);
  }
 
  .group-files-list {
    display: flex;
    flex-direction: column;
  }
 
  .duplicate-file-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
    transition: background-color 0.12s ease;
  }
 
  .duplicate-file-item:last-child {
    border-bottom: none;
  }
 
  .duplicate-file-item:hover {
    background-color: rgba(255, 255, 255, 0.03);
  }
 
  .duplicate-file-item.previewing {
    background-color: var(--bg-selected);
  }
 
  .file-info-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: pointer;
    text-align: left;
    outline: none;
  }
 
  .file-path {
    font-size: 12px;
    font-weight: 500;
    color: var(--accent);
    word-break: break-all;
    font-family: var(--font-mono);
  }
 
  .file-info-col:hover .file-path {
    text-decoration: underline;
  }
 
  .file-meta {
    font-size: 10px;
    color: var(--text-secondary);
  }

  .btn-xs {
    padding: 2px 6px;
    font-size: 10.5px;
    border-radius: var(--radius-sm);
  }
 
  /* Preview Drawer Styling */
  .preview-drawer {
    position: absolute;
    top: 0;
    right: 0;
    width: 320px;
    height: 100%;
    background-color: var(--bg-elevated);
    border-left: 1px solid var(--border-strong);
    box-shadow: var(--shadow-lg);
    z-index: 10;
    display: flex;
    flex-direction: column;
    text-align: left;
    animation: zoomIn 0.15s ease-out;
  }
 
  .drawer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-normal);
    background-color: rgba(0, 0, 0, 0.08);
  }

  .drawer-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }

  .drawer-icon {
    font-size: 14px;
  }
 
  .drawer-header h4 {
    margin: 0;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
 
  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 20px;
    cursor: pointer;
    opacity: 0.7;
    transition: opacity 0.2s;
    line-height: 1;
    padding: 0 4px;
  }
 
  .close-btn:hover {
    opacity: 1;
  }
 
  .drawer-body {
    flex: 1;
    padding: 14px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
 
  .drawer-path-sub {
    font-size: 10px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    word-break: break-all;
    background-color: rgba(0, 0, 0, 0.1);
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
  }
 
  .preview-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 40px 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
 
  .preview-content {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    font-family: var(--font-mono, monospace);
    color: var(--text-primary);
    background-color: rgba(0, 0, 0, 0.15);
    padding: 10px;
    border-radius: var(--radius-sm, 4px);
    white-space: pre-wrap;
    word-break: break-word;
    border: 1px solid rgba(255, 255, 255, 0.03);
  }
</style>
