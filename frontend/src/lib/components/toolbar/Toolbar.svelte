<script lang="ts">
  import { createNewNote, activeNote, selectedFolder, selectNote, activeNoteLocked } from '../../stores/notes';
  import { editorViewMode, showAttachments, loadAttachments } from '../../stores/editor';
  import { vaultInfo } from '../../stores/vault';
  import { get } from 'svelte/store';
  import SyncStatus from '../sync/SyncStatus.svelte';
  import SearchBar from '../search/SearchBar.svelte';
  import * as ipc from '../../services/ipc';
  import { onMount } from 'svelte';

  const info = $derived($vaultInfo);
  const viewMode = $derived($editorViewMode);
  const attachmentsVisible = $derived($showAttachments);
  const hasActiveNote = $derived($activeNote !== null);

  let isVaultUnlocked = $state(false);
  let isConfigured = $state(false);

  async function checkLockStatus() {
    try {
      isConfigured = await ipc.isVaultConfigured();
      if (isConfigured) {
        const unlocked = await ipc.isVaultSessionUnlocked();
        isVaultUnlocked = unlocked;
        
        // If session is locked, and we have an encrypted note currently displayed as unlocked, auto-lock it!
        if (!unlocked) {
          const active = get(activeNote);
          if (active && active.is_encrypted && !get(activeNoteLocked)) {
            activeNoteLocked.set(true);
            activeNote.update(n => n ? { ...n, body: '' } : null);
          }
        }
      } else {
        isVaultUnlocked = false;
      }
    } catch (e) {
      console.error(e);
    }
  }

  onMount(() => {
    checkLockStatus();
  });

  async function handleNewNote() {
    try {
      const folder = get(selectedFolder);
      await createNewNote(folder);
    } catch (e) {
      console.error('Failed to create new note:', e);
    }
  }

  function setViewMode(mode: 'edit' | 'preview' | 'live') {
    editorViewMode.set(mode);
  }

  async function handleQuickLock() {
    try {
      await ipc.lockVaultInstantly();
      await checkLockStatus();
      const active = get(activeNote);
      if (active) {
        await selectNote(active.id);
      }
    } catch (e) {
      console.error(e);
    }
  }

  function toggleAttachments() {
    showAttachments.update(v => {
      const newVal = !v;
      if (newVal) {
        loadAttachments().catch(console.error);
      }
      return newVal;
    });
  }
</script>

<header class="toolbar" data-tauri-drag-region>

  <!-- Sol: Marka + Vault bilgisi + Yeni Not butonu -->
  <div class="section section-left" data-tauri-drag-region>
    {#if info}
      <div class="app-brand" data-tauri-drag-region>
        <div class="brand-icon" aria-hidden="true">
          <svg viewBox="0 0 20 20" fill="none">
            <rect width="20" height="20" rx="5" fill="#0a84ff"/>
            <path d="M5 15V5l5 8 5-8v10" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <span class="brand-name">Noda</span>
      </div>

      <div class="vault-pill" data-tauri-drag-region>
        <!-- Vault ikonu — küçük, subtle -->
        <svg viewBox="0 0 16 16" fill="currentColor" class="vault-icon" aria-hidden="true">
          <path d="M2 2a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2zm2-1a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1H4z"/>
          <path d="M5 4h6v1H5V4zm0 2h6v1H5V6zm0 2h4v1H5V8z"/>
        </svg>
        <span class="vault-name">{info.name}</span>
      </div>

      <button class="btn-new-note" onclick={handleNewNote} title="Yeni Not (⌘N)">
        <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true">
          <line x1="7" y1="1" x2="7" y2="13"/>
          <line x1="1" y1="7" x2="13" y2="7"/>
        </svg>
        <span>New Note</span>
      </button>
    {/if}
  </div>

  <!-- Orta: Arama -->
  <div class="section section-center" data-tauri-drag-region>
    {#if info}
      <SearchBar />
    {/if}
  </div>

  <!-- Sağ: View toggle + Geçmiş + Sync -->
  <div class="section section-right" data-tauri-drag-region>
    {#if info}
      {#if hasActiveNote}
        <!-- Segmented control — macOS tarzı görünüm toggle -->
        <div class="segmented-control" role="group" aria-label="View mode">
          <button
            class="seg-btn"
            class:active={viewMode === 'edit'}
            onclick={() => setViewMode('edit')}
            title="Raw Markdown Source"
            aria-pressed={viewMode === 'edit'}
          >
            <!-- Raw / Pencil icon -->
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M11.5 2.5 13.5 4.5 5 13H3v-2L11.5 2.5z"/>
            </svg>
          </button>
          <button
            class="seg-btn"
            class:active={viewMode === 'live'}
            onclick={() => setViewMode('live')}
            title="Live Preview (Obsidian Style)"
            aria-pressed={viewMode === 'live'}
          >
            <!-- Sparkles/WYSIWYG combining Edit and Preview icon -->
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M2 8s2.5-4.5 6-4.5 6 4.5 6 4.5-2.5 4.5-7 4.5" />
              <circle cx="8" cy="8" r="1.8" />
              <path d="M11.5 2.5 L13.5 4.5" />
            </svg>
          </button>
          <button
            class="seg-btn"
            class:active={viewMode === 'preview'}
            onclick={() => setViewMode('preview')}
            title="Reading View (Full HTML Preview)"
            aria-pressed={viewMode === 'preview'}
          >
            <!-- Eye icon -->
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z"/>
              <circle cx="8" cy="8" r="2"/>
            </svg>
          </button>
        </div>

      {/if}

      {#if isConfigured}
        <button 
          class="btn-quick-lock"
          class:unlocked={isVaultUnlocked}
          class:locked={!isVaultUnlocked}
          onclick={handleQuickLock}
          title={isVaultUnlocked ? "Lock Vault Session" : "Vault Session Locked"}
        >
          {#if isVaultUnlocked}
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="7" width="10" height="8" rx="1.5"/>
              <path d="M4.5 7V4.5a3.5 3.5 0 0 1 5.5 -2.8"/>
            </svg>
          {:else}
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" class="locked-icon">
              <rect x="3" y="7" width="10" height="8" rx="1.5"/>
              <path d="M4.5 7V4.5a3.5 3.5 0 0 1 7 0V7"/>
            </svg>
          {/if}
        </button>
      {/if}

      <SyncStatus />
    {/if}
  </div>
</header>

<style>
  .btn-quick-lock {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    padding: 6px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
    margin-right: 2px;
  }
  .btn-quick-lock:hover {
    color: var(--text-secondary);
    background-color: var(--bg-control-hover);
  }
  .btn-quick-lock svg {
    width: 14px;
    height: 14px;
  }
  .btn-quick-lock.unlocked svg {
    color: var(--color-green, #10b981);
  }
  .btn-quick-lock.locked svg {
    color: var(--color-red, #ef4444);
  }
  :global(.platform-darwin) .toolbar {
    padding-left: 80px; /* macOS pencere kontrolleri (traffic lights) için sol boşluk */
  }

  .toolbar {
    height: 48px;
    /* Vibrancy efekti — macOS toolbar hissi */
    background-color: var(--toolbar-bg);
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    user-select: none;
    flex-shrink: 0;
    /* macOS metal/glass görünümü için çok ince üst kenar */
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  }

  /* Marka */
  .app-brand {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-right: 12px;
    cursor: default;
    user-select: none;
  }

  .brand-icon {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .brand-icon svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  .brand-name {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.2px;
  }

  /* Genel bölüm düzeni */
  .section {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
  }

  .section-left {
    flex: 1;
    justify-content: flex-start;
  }

  .section-center {
    flex-shrink: 0;
    justify-content: center;
  }

  .section-right {
    flex: 1;
    justify-content: flex-end;
  }

  /* Vault pill — küçük, bilgilendirici */
  .vault-pill {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.1px;
  }

  .vault-icon {
    width: 12px;
    height: 12px;
    color: var(--accent);
    flex-shrink: 0;
  }

  .vault-name {
    max-width: 120px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Yeni Not butonu — macOS mavi pill */
  .btn-new-note {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    background-color: var(--accent);
    border: none;
    border-radius: var(--radius-sm);
    color: #ffffff;
    font-family: var(--font-sans);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s ease, transform 0.1s ease;
    letter-spacing: 0.1px;
  }

  .btn-new-note svg {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .btn-new-note:hover {
    background-color: var(--accent-hover);
  }

  .btn-new-note:active {
    transform: scale(0.96);
  }

  /* Segmented control — macOS stili grup buton */
  .segmented-control {
    display: flex;
    align-items: center;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 1px;
  }

  .seg-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    padding: 4px 7px;
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
    line-height: 1;
  }

  .seg-btn svg {
    width: 14px;
    height: 14px;
    display: block;
  }

  .seg-btn:hover {
    color: var(--text-secondary);
    background-color: var(--bg-control-hover);
  }

  .seg-btn.active {
    color: var(--accent);
    background-color: var(--bg-selected);
  }

</style>
