<script lang="ts">
  import { createNewNote, activeNote } from '../../stores/notes';
  import { editorViewMode, showSnapshots } from '../../stores/editor';
  import { vaultInfo } from '../../stores/vault';
  import SyncStatus from '../sync/SyncStatus.svelte';
  import ConflictBadge from '../sync/ConflictBadge.svelte';
  import SearchBar from '../search/SearchBar.svelte';

  $: info = $vaultInfo;
  $: viewMode = $editorViewMode;
  $: snapshotsVisible = $showSnapshots;
  $: hasActiveNote = $activeNote !== null;

  async function handleNewNote() {
    try {
      await createNewNote();
    } catch (e) {
      console.error('Failed to create new note:', e);
    }
  }

  function setViewMode(mode: 'edit' | 'preview' | 'split') {
    editorViewMode.set(mode);
  }

  function toggleSnapshots() {
    showSnapshots.update(v => !v);
  }
</script>

<header class="toolbar border-bottom" data-tauri-drag-region>
  <!-- macOS Traffic Lights Gap -->
  <div class="macos-gap" data-tauri-drag-region></div>

  <!-- Left: Vault Information & Add Button -->
  <div class="left-section" data-tauri-drag-region>
    {#if info}
      <div class="vault-info-pill" data-tauri-drag-region>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="w-3.5 h-3.5 icon">
          <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A9 9 0 0 1 12 3v0a9 9 0 0 1 9 9v.75m-18 0a2.25 2.25 0 0 0 2.25 2.25h13.5A2.25 2.25 0 0 0 21 12.75m-18 0V12a9 9 0 0 1 9-9v0a9 9 0 0 1 9 9v.75m-18 0a2.25 2.25 0 0 1 2.25-2.25h13.5A2.25 2.25 0 0 1 21 12.75m-18 0v1.5a2.25 2.25 0 0 0 2.25 2.25h13.5a2.25 2.25 0 0 0 2.25-2.25v-1.5m-18 0V12a9 9 0 0 0 9 9v0a9 9 0 0 0 9-9v-.75" />
        </svg>
        <span class="vault-name">{info.name}</span>
      </div>
      <button class="new-note-btn hover-glow" onclick={handleNewNote} title="New Note (Cmd+N)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
        </svg>
        <span>New Note</span>
      </button>
    {/if}
  </div>

  <!-- Center: Search Widget -->
  <div class="center-section" data-tauri-drag-region>
    {#if info}
      <SearchBar />
    {/if}
  </div>

  <!-- Right: Views, Sync & Conflicts -->
  <div class="right-section" data-tauri-drag-region>
    {#if info}
      <ConflictBadge />
      <SyncStatus />

      {#if hasActiveNote}
        <div class="view-toggles border-all">
          <button
            class="toggle-btn"
            class:active={viewMode === 'edit'}
            onclick={() => setViewMode('edit')}
            title="Edit Mode"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75 22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3-4.5 16.5" />
            </svg>
          </button>
          <button
            class="toggle-btn"
            class:active={viewMode === 'split'}
            onclick={() => setViewMode('split')}
            title="Split Mode"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9 4.5v15m6-15v15m-9-15h12a1.5 1.5 0 0 1 1.5 1.5v12a1.5 1.5 0 0 1-1.5 1.5H5.25a1.5 1.5 0 0 1-1.5-1.5V6a1.5 1.5 0 0 1 1.5-1.5Z" />
            </svg>
          </button>
          <button
            class="toggle-btn"
            class:active={viewMode === 'preview'}
            onclick={() => setViewMode('preview')}
            title="Preview Mode"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" />
            </svg>
          </button>
        </div>

        <button
          class="snapshots-toggle-btn hover-glow"
          class:active={snapshotsVisible}
          onclick={toggleSnapshots}
          title="Toggle Note History"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
          </svg>
        </button>
      {/if}
    {/if}
  </div>
</header>

<style>
  .toolbar {
    height: 48px;
    background-color: #0b0e14;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    user-select: none;
    flex-shrink: 0;
  }

  .border-bottom {
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  /* macOS Traffic Light Padding Gap */
  .macos-gap {
    width: 72px;
    height: 100%;
    flex-shrink: 0;
    display: none;
  }

  :global(.platform-darwin) .macos-gap {
    display: block;
  }

  .left-section, .right-section {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 100%;
  }

  .left-section {
    flex: 1;
    justify-content: flex-start;
  }

  .center-section {
    display: flex;
    justify-content: center;
    align-items: center;
    flex-shrink: 0;
  }

  .right-section {
    flex: 1;
    justify-content: flex-end;
  }

  .vault-info-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.04);
    border-radius: 6px;
    color: #94a3b8;
  }

  .vault-info-pill .icon {
    color: #6366f1;
  }

  .vault-name {
    font-size: 0.78rem;
    font-weight: 600;
  }

  .new-note-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    background-color: #6366f1;
    border: none;
    border-radius: 6px;
    color: #ffffff;
    font-family: inherit;
    font-size: 0.78rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .new-note-btn:hover {
    background-color: #4f46e5;
    box-shadow: 0 0 12px rgba(99, 102, 241, 0.4);
  }

  /* View mode and snap toggles styling */
  .border-all {
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .view-toggles {
    display: flex;
    background-color: rgba(255, 255, 255, 0.01);
    border-radius: 6px;
    overflow: hidden;
  }

  .toggle-btn {
    background: transparent;
    border: none;
    color: #475569;
    padding: 6px 10px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .toggle-btn:hover {
    color: #94a3b8;
    background-color: rgba(255, 255, 255, 0.03);
  }

  .toggle-btn.active {
    color: #818cf8;
    background-color: rgba(99, 102, 241, 0.1);
  }

  .snapshots-toggle-btn {
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    color: #475569;
    padding: 6px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .snapshots-toggle-btn:hover {
    color: #94a3b8;
    background-color: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.1);
  }

  .snapshots-toggle-btn.active {
    color: #818cf8;
    background-color: rgba(99, 102, 241, 0.1);
    border-color: rgba(99, 102, 241, 0.3);
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.15);
  }
</style>
