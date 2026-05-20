<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { getEditorExtensions } from './extensions';
  import { activeNote, updateActiveNoteBody, saveActiveNote, activeNoteDirty } from '../../stores/notes';
  import { editorViewMode, snapshotsList, showSnapshots, loadNoteSnapshots, restoreNoteSnapshot, loadingEditorMetadata } from '../../stores/editor';
  import Preview from './Preview.svelte';

  let editorElement: HTMLDivElement;
  let editorView: EditorView | null = null;
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  // Track the current note ID to detect note switches
  let lastNoteId: string | null = null;

  $: currentNote = $activeNote;
  $: viewMode = $editorViewMode;
  $: displaySnapshots = $showSnapshots;

  // React to note changes: reload content into editor
  $: if (currentNote && currentNote.id !== lastNoteId) {
    lastNoteId = currentNote.id;
    if (editorView) {
      const state = EditorState.create({
        doc: currentNote.body,
        extensions: getEditorExtensions(handleDocChange),
      });
      editorView.setState(state);
    }
    // Pre-fetch snapshots for this note in background
    loadNoteSnapshots(currentNote.id);
  }

  function handleDocChange(newContent: string) {
    updateActiveNoteBody(newContent);
    
    // Auto-save debounced (1500ms of typing silence)
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      try {
        await saveActiveNote();
      } catch (err) {
        console.error('Failed to auto-save:', err);
      }
    }, 1500);
  }

  onMount(() => {
    const initialState = EditorState.create({
      doc: currentNote ? currentNote.body : '',
      extensions: getEditorExtensions(handleDocChange),
    });

    editorView = new EditorView({
      state: initialState,
      parent: editorElement,
    });
  });

  onDestroy(() => {
    if (editorView) editorView.destroy();
    if (saveTimeout) clearTimeout(saveTimeout);
  });

  async function handleRestore(timestamp: string) {
    if (!currentNote) return;
    if (confirm('Are you sure you want to restore this historical snapshot? This will overwrite the current content.')) {
      await restoreNoteSnapshot(currentNote.id, timestamp);
    }
  }

  function formatDate(isoStr: string) {
    const d = new Date(isoStr);
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  }
</script>

<div class="editor-shell">
  {#if !currentNote}
    <div class="editor-empty">
      <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25" />
      </svg>
      <h2>No Note Selected</h2>
      <p>Select a note from the list or create a new one to start writing.</p>
    </div>
  {:else}
    <div class="editor-workspace">
      <!-- Editor & Preview Panel -->
      <div class="editor-panels" class:split-layout={viewMode === 'split'}>
        <div class="panel-editor" class:hidden={viewMode === 'preview'} bind:this={editorElement}></div>
        
        {#if viewMode === 'split'}
          <div class="panel-divider"></div>
        {/if}

        <div class="panel-preview" class:hidden={viewMode === 'edit'}>
          <Preview content={currentNote.body} />
        </div>
      </div>

      <!-- History Snapshots Side Panel -->
      {#if displaySnapshots}
        <div class="snapshots-sidebar border-left">
          <div class="sidebar-header">
            <h3>Version History</h3>
            <button class="close-btn" onclick={() => showSnapshots.set(false)}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
                <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="snapshots-list scrollbar-thin">
            {#if $loadingEditorMetadata}
              <div class="metadata-loading">
                <div class="spinner"></div>
                <span>Loading history...</span>
              </div>
            {:else if $snapshotsList.length === 0}
              <div class="empty-state">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
                </svg>
                <p>No snapshots recorded yet. A version is saved every time you edit.</p>
              </div>
            {:else}
              {#each $snapshotsList as snap (snap.id)}
                <div class="snapshot-card">
                  <div class="snapshot-info">
                    <span class="timestamp">{formatDate(snap.timestamp)}</span>
                    <span class="file-name">{snap.file_path.split('/').pop()}</span>
                  </div>
                  <button class="restore-btn hover-glow" onclick={() => handleRestore(snap.timestamp)}>
                    Restore
                  </button>
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    flex: 1;
    height: 100%;
    background-color: #0a0d14;
    overflow: hidden;
  }

  .editor-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: #475569;
    padding: 40px;
    text-align: center;
  }

  .empty-icon {
    width: 64px;
    height: 64px;
    margin-bottom: 16px;
    color: #334155;
  }

  .editor-empty h2 {
    color: #94a3b8;
    font-size: 1.25rem;
    font-weight: 500;
    margin: 0 0 8px 0;
  }

  .editor-empty p {
    color: #475569;
    max-width: 320px;
    font-size: 0.9rem;
    margin: 0;
  }

  .editor-workspace {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .editor-panels {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .panel-editor, .panel-preview {
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .panel-editor {
    background-color: #0a0d14;
  }

  .panel-preview {
    background-color: #0d1117;
  }

  .hidden {
    display: none !important;
  }

  .split-layout .panel-editor {
    border-right: 1px solid rgba(255, 255, 255, 0.05);
  }

  .panel-divider {
    width: 1px;
    background-color: rgba(255, 255, 255, 0.05);
  }

  /* Version History Sidebar */
  .snapshots-sidebar {
    width: 280px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: #090b10;
  }

  .border-left {
    border-left: 1px solid rgba(255, 255, 255, 0.05);
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .sidebar-header h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #475569;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    color: #f8fafc;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .snapshots-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .snapshot-card {
    background-color: #0f131c;
    border: 1px solid rgba(255, 255, 255, 0.03);
    padding: 12px;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: border-color 0.2s ease;
  }

  .snapshot-card:hover {
    border-color: rgba(99, 102, 241, 0.3);
  }

  .snapshot-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .snapshot-info .timestamp {
    color: #e2e8f0;
    font-size: 0.85rem;
    font-weight: 500;
  }

  .snapshot-info .file-name {
    color: #475569;
    font-size: 0.7rem;
    font-family: monospace;
  }

  .restore-btn {
    align-self: flex-end;
    background: rgba(99, 102, 241, 0.15);
    border: 1px solid rgba(99, 102, 241, 0.3);
    color: #818cf8;
    padding: 4px 10px;
    font-size: 0.75rem;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.15s ease;
  }

  .restore-btn:hover {
    background: #6366f1;
    color: #ffffff;
    box-shadow: 0 0 8px rgba(99, 102, 241, 0.4);
  }

  .metadata-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: #475569;
    padding: 40px 0;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid rgba(255, 255, 255, 0.05);
    border-top-color: #6366f1;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .empty-state {
    text-align: center;
    padding: 30px 10px;
    color: #475569;
  }

  .empty-state svg {
    width: 32px;
    height: 32px;
    color: #334155;
    margin-bottom: 8px;
  }

  .empty-state p {
    font-size: 0.75rem;
    margin: 0;
    line-height: 1.4;
  }

  /* Sleek modern scrollbar */
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
