<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { getEditorExtensions, livePreviewCompartment, editorModeCompartment, livePreviewPlugin } from './extensions';
  import DiffViewer from '../history/DiffViewer.svelte';
  import {
    activeNote, updateActiveNoteBody, saveActiveNote, renameActiveNote,
    activeNoteDirty, lastSavedAt, updateActiveNoteTags, notesList,
    viewingTrashNote, viewingConflictNote, activeViewMode, selectedFolder,
    selectNote
  } from '../../stores/notes';
  import { recoverFromTrash, emptyTrashPermanently } from '../../stores/editor';
  import { resolveKeepLocal, resolveKeepRemote } from '../../stores/sync';
  import {
    editorViewMode, snapshotsList, showSnapshots,
    loadNoteSnapshots, restoreNoteSnapshot, deleteNoteSnapshot, loadingEditorMetadata
  } from '../../stores/editor';
  import * as ipc from '../../services/ipc';
  import { appConfig } from '../../stores/settings';
  import Preview from './Preview.svelte';
  import StatusBar from './StatusBar.svelte';

  let editorView: EditorView | null = null;
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;
  let lastNoteId: string | null = null;
  let wordCount = 0;
  let charCount = 0;

  let currentDiff: import('../../types').SnapshotDiffDto | null = null;
  let isDiffOpen = false;

  let remoteNote: import('../../types').NoteDto | null = null;
  let loadingRemote = false;

  $: if (isConflict) {
    loadRemoteConflictNote(isConflict.archivedPath);
  } else {
    remoteNote = null;
  }

  async function loadRemoteConflictNote(archivedPath: string) {
    loadingRemote = true;
    try {
      remoteNote = await ipc.getConflictNote(archivedPath);
    } catch (e) {
      console.error('Failed to load remote conflict note:', e);
    } finally {
      loadingRemote = false;
    }
  }

  $: currentNote = $activeNote;
  $: viewMode    = $editorViewMode;
  $: displaySnapshots = $showSnapshots;
  $: isDirty    = $activeNoteDirty;
  $: savedAt    = currentNote ? new Date(currentNote.updated_at) : null;
  $: isTrash     = $viewingTrashNote;
  $: isConflict  = $viewingConflictNote;
  $: isReadOnly  = isTrash || isConflict !== null;

  let tagInput = '';
  let showSuggestions = false;
  let activeSuggestionIndex = 0;
  let suggestionEl: HTMLDivElement;

  $: currentNoteTags = currentNote?.tags || [];

  // Extract all unique tags in other notes
  $: allExistingTags = Array.from(
    new Set(
      ($notesList || []).flatMap(note => note.tags || [])
    )
  ).sort();

  // Filter suggestions based on typed input and not already present on currentNote
  $: suggestions = allExistingTags.filter(t => 
    t.toLowerCase().includes(tagInput.toLowerCase()) && 
    !currentNoteTags.includes(t)
  );

  function addTag(tag: string) {
    const trimmed = tag.trim().replace(/#/g, '');
    if (trimmed && !currentNoteTags.includes(trimmed)) {
      const newTags = [...currentNoteTags, trimmed];
      updateActiveNoteTags(newTags);
    }
    tagInput = '';
    showSuggestions = false;
    activeSuggestionIndex = 0;
  }

  function removeTag(tagToRemove: string) {
    const newTags = currentNoteTags.filter(t => t !== tagToRemove);
    updateActiveNoteTags(newTags);
  }

  function handleTagInputKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (showSuggestions && suggestions.length > 0 && activeSuggestionIndex >= 0) {
        addTag(suggestions[activeSuggestionIndex]);
      } else if (tagInput.trim()) {
        addTag(tagInput);
      }
    } else if (e.key === ',' || e.key === ' ') {
      if (tagInput.trim()) {
        e.preventDefault();
        addTag(tagInput);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (!showSuggestions) {
        showSuggestions = true;
      } else if (suggestions.length > 0) {
        activeSuggestionIndex = (activeSuggestionIndex + 1) % suggestions.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (showSuggestions && suggestions.length > 0) {
        activeSuggestionIndex = (activeSuggestionIndex - 1 + suggestions.length) % suggestions.length;
      }
    } else if (e.key === 'Escape') {
      showSuggestions = false;
    } else if (e.key === 'Backspace' && !tagInput && currentNoteTags.length > 0) {
      removeTag(currentNoteTags[currentNoteTags.length - 1]);
    }
  }

  // Not değişince veya içeriği dışarıdan/snapshot'tan güncellendiğinde editörü güncelle
  $: if (currentNote) {
    if (currentNote.id !== lastNoteId) {
      lastNoteId = currentNote.id;
      if (editorView) {
        const state = EditorState.create({
          doc: currentNote.body,
          extensions: getEditorExtensions(handleDocChange, viewMode),
        });
        editorView.setState(state);
        // Yeniden ölçüm — hidden durumdan dönülürse boyutları düzeltir
        setTimeout(() => editorView?.requestMeasure(), 10);
      }
      updateStats(currentNote.body);
    } else if (editorView && editorView.state.doc.toString() !== currentNote.body && !isDirty) {
      const state = EditorState.create({
        doc: currentNote.body,
        extensions: getEditorExtensions(handleDocChange, viewMode),
      });
      editorView.setState(state);
      updateStats(currentNote.body);
    }
  }

  // viewMode değişince CodeMirror Live Preview uzantısını güncelle ve boyutları ölç
  $: if (editorView) {
    editorView.dispatch({
      effects: [
        livePreviewCompartment.reconfigure(viewMode === 'live' ? [livePreviewPlugin] : []),
        editorModeCompartment.reconfigure(EditorView.editorAttributes.of({
          class: viewMode === 'live' ? 'cm-mode-live' : 'cm-mode-edit'
        }))
      ]
    });
    if (viewMode !== 'preview') {
      setTimeout(() => editorView?.requestMeasure(), 0);
    }
  }

  function updateStats(content: string) {
    charCount = content.length;
    const words = content.trim().split(/\s+/).filter(Boolean);
    wordCount = content.trim() ? words.length : 0;
  }

  function handleDocChange(newContent: string) {
    updateActiveNoteBody(newContent);
    updateStats(newContent);
    if (saveTimeout) clearTimeout(saveTimeout);
    
    const delay = $appConfig?.editor?.auto_save_delay_ms ?? 1500;
    saveTimeout = setTimeout(async () => {
      try {
        await saveActiveNote();
      } catch (err) {
        console.error('Auto-save failed:', err);
      }
    }, delay);
  }

  async function handleManualSave() {
    if (saveTimeout) clearTimeout(saveTimeout);
    try {
      await saveActiveNote();
    } catch (err) {
      console.error('Manual save failed:', err);
    }
  }

  // Cmd+S kısayolu
  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      handleManualSave();
    }
  }

  async function handleTitleChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const newTitle = input.value.trim() || 'Untitled';
    if (currentNote && currentNote.title !== newTitle) {
      try {
        await renameActiveNote(newTitle);
      } catch (err) {
        console.error('Failed to rename note:', err);
        input.value = currentNote.title; // revert on fail
      }
    }
  }

  function editorAction(node: HTMLElement) {
    const initialState = EditorState.create({
      doc: currentNote ? currentNote.body : '',
      extensions: getEditorExtensions(handleDocChange, viewMode),
    });
    editorView = new EditorView({
      state: initialState,
      parent: node,
    });

    if (currentNote) {
      updateStats(currentNote.body);
    }

    return {
      destroy() {
        editorView?.destroy();
        editorView = null;
      }
    };
  }

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeyDown);
  });

  onDestroy(() => {
    if (saveTimeout) clearTimeout(saveTimeout);
    window.removeEventListener('keydown', handleGlobalKeyDown);
  });

  async function openDiff(timestamp: string) {
    if (!currentNote) return;
    try {
      currentDiff = await ipc.compareSnapshot(currentNote.id, timestamp);
      isDiffOpen = true;
    } catch (e) {
      console.error("Failed to load snapshot diff:", e);
      alert("Diff could not be loaded.");
    }
  }

  async function handleRestore(timestamp: string) {
    if (!currentNote) return;
    isDiffOpen = false;
    currentDiff = null;
    if (confirm('Are you sure you want to restore this version? Current unsaved changes will be lost.')) {
      await restoreNoteSnapshot(currentNote.id, timestamp);
    }
  }

  async function handleDeleteSnapshot(timestamp: string) {
    if (!currentNote) return;
    if (confirm('Are you sure you want to permanently delete this snapshot? This cannot be undone.')) {
      try {
        await deleteNoteSnapshot(currentNote.id, timestamp);
      } catch (e) {
        console.error('Failed to delete snapshot:', e);
        alert('Failed to delete snapshot.');
      }
    }
  }

  function formatDate(isoStr: string): string {
    return new Date(isoStr).toLocaleString(undefined, {
      month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<div class="editor-shell">
  {#if !currentNote}
    <!-- Boş durum -->
    <div class="editor-empty">
      <div class="empty-icon">
        <svg viewBox="0 0 48 48" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="8" y="6" width="32" height="36" rx="4"/>
          <line x1="15" y1="16" x2="33" y2="16"/>
          <line x1="15" y1="22" x2="33" y2="22"/>
          <line x1="15" y1="28" x2="25" y2="28"/>
        </svg>
      </div>
      <h2>No Note Selected</h2>
      <p>Select a note from the list or create one with <kbd>⌘N</kbd></p>
    </div>

  {:else}
    <div class="editor-workspace">
      <!-- Editor + Önizleme paneli -->
      <div class="editor-area">
        <!-- Başlık bar — not adı + aksiyon butonları -->
        <div class="note-titlebar">
          <!-- Trash / Conflict Banner (overrides normal titlebar content when in special mode) -->
          {#if isTrash && currentNote}
            <div class="special-mode-banner trash-banner">
              <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" class="banner-icon" aria-hidden="true">
                <polyline points="2,3.5 12,3.5"/>
                <path d="M5 3.5V3a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v.5M5.5 6.5v3.5M8.5 6.5v3.5"/>
                <path d="M3 3.5l.75 7.5a.75.75 0 0 0 .75.75h4.5a.75.75 0 0 0 .75-.75L10.5 3.5"/>
              </svg>
              <span class="banner-label">Deleted Note <span class="banner-title">{currentNote.title || 'Untitled'}</span></span>
              <div class="banner-actions">
                <button class="banner-btn restore-btn" onclick={async () => { const id = currentNote.id; await recoverFromTrash(id); selectedFolder.set(null); activeViewMode.set('normal'); await selectNote(id); }} title="Restore note">
                  <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
                    <path d="M1 6a5 5 0 1 0 1-2.9"/>
                    <polyline points="1,2 1,6 5,6"/>
                  </svg>
                  Restore
                </button>
                <button class="banner-btn delete-btn" onclick={async () => { if(confirm('Permanently delete this note?')) { await emptyTrashPermanently(currentNote.id); activeNote.set(null); viewingTrashNote.set(false); selectedFolder.set('__trash__'); } }} title="Delete permanently">
                  <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
                    <line x1="2" y1="2" x2="10" y2="10"/>
                    <line x1="10" y1="2" x2="2" y2="10"/>
                  </svg>
                  Delete Forever
                </button>
              </div>
            </div>

          {:else if isConflict && currentNote}
            <div class="special-mode-banner conflict-banner">
              <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" class="banner-icon" aria-hidden="true">
                <path d="M7 1.5L13 11.5H1L7 1.5z"/>
                <line x1="7" y1="5" x2="7" y2="8"/>
                <circle cx="7" cy="9.5" r="0.4" fill="currentColor"/>
              </svg>
              <span class="banner-label">Sync Conflict <span class="banner-title">{currentNote.title || 'Untitled'}</span></span>
              <div class="banner-actions">
                <button class="banner-btn keep-local-btn" onclick={async () => { const id = isConflict.noteId; await resolveKeepLocal(isConflict.archivedPath); selectedFolder.set(null); activeViewMode.set('normal'); await selectNote(id); }} title="Keep your local version">
                  Keep Local
                </button>
                <button class="banner-btn keep-remote-btn" onclick={async () => { const id = isConflict.noteId; await resolveKeepRemote(isConflict.noteId, isConflict.archivedPath); selectedFolder.set(null); activeViewMode.set('normal'); await selectNote(id); }} title="Use the synced (remote) version">
                  Use Remote
                </button>
              </div>
            </div>

          {:else}
            <!-- Normal titlebar -->
            <div class="title-spacer"></div>

            <input
              type="text"
              class="note-title-input"
              value={currentNote.title || ''}
              placeholder="Untitled"
              onblur={handleTitleChange}
              onkeydown={(e) => { if (e.key === 'Enter') e.currentTarget.blur() }}
              title={currentNote.file_path}
            />

            <div class="note-actions">
              {#if isDirty}
                <span class="unsaved-dot" title="Unsaved changes" aria-label="Unsaved changes"></span>
              {/if}
              <button
                class="action-btn"
                class:accent={isDirty}
                onclick={handleManualSave}
                title="Save (⌘S)"
                disabled={!isDirty}
              >
                <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M2 12V4.5L4.5 2h7a.5.5 0 0 1 .5.5V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
                  <rect x="4" y="2" width="5" height="4" rx=".5"/>
                  <rect x="3.5" y="8" width="7" height="5" rx=".5"/>
                </svg>
                Save
              </button>
            </div>
          {/if}
        </div>

        <!-- Panel alanı -->
        {#if isConflict}
          <div class="conflict-compare-container">
            {#if loadingRemote}
              <div class="compare-loading">
                <div class="spinner"></div>
                <span>Loading remote version...</span>
              </div>
            {:else if remoteNote && currentNote}
              <div class="compare-split">
                <!-- Left Pane: Local Version -->
                <div class="compare-pane pane-local">
                  <div class="pane-header">
                    <span class="pane-badge badge-local">Local Version</span>
                    <span class="pane-date">Last modified: {formatDate(currentNote.updated_at)}</span>
                  </div>
                  <div class="pane-content scrollbar-thin">
                    <h1 class="pane-title">{currentNote.title || 'Untitled'}</h1>
                    <div class="pane-tags">
                      {#each currentNoteTags as tag}
                        <span class="tag-pill">#{tag}</span>
                      {/each}
                    </div>
                    <div class="pane-body">
                      <Preview content={currentNote.body} />
                    </div>
                  </div>
                  <div class="pane-footer">
                    <button class="btn-keep btn-keep-local" onclick={async () => { const id = isConflict.noteId; await resolveKeepLocal(isConflict.archivedPath); selectedFolder.set(null); activeViewMode.set('normal'); await selectNote(id); }}>
                      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="width:12px; height:12px; margin-right:4px;">
                        <polyline points="1.5,6 4.5,9 10.5,3"/>
                      </svg>
                      Keep Local Version
                    </button>
                  </div>
                </div>

                <!-- Right Pane: Remote Version -->
                <div class="compare-pane pane-remote">
                  <div class="pane-header">
                    <span class="pane-badge badge-remote">Remote Version</span>
                    <span class="pane-date">Last modified: {formatDate(remoteNote.updated_at)}</span>
                  </div>
                  <div class="pane-content scrollbar-thin">
                    <h1 class="pane-title">{remoteNote.title || 'Untitled'}</h1>
                    <div class="pane-tags">
                      {#each remoteNote.tags || [] as tag}
                        <span class="tag-pill">#{tag}</span>
                      {/each}
                    </div>
                    <div class="pane-body">
                      <Preview content={remoteNote.body} />
                    </div>
                  </div>
                  <div class="pane-footer">
                    <button class="btn-keep btn-keep-remote" onclick={async () => { const id = isConflict.noteId; await resolveKeepRemote(isConflict.noteId, isConflict.archivedPath); selectedFolder.set(null); activeViewMode.set('normal'); await selectNote(id); }}>
                      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" style="width:12px; height:12px; margin-right:4px;">
                        <polyline points="1.5,6 4.5,9 10.5,3"/>
                      </svg>
                      Use Remote Version
                    </button>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {:else}
          <div class="editor-panels">
            <div
              class="panel-editor"
              class:panel-hidden={viewMode === 'preview'}
              use:editorAction
              aria-hidden={viewMode === 'preview'}
            ></div>

            <div
              class="panel-preview"
              class:panel-hidden={viewMode !== 'preview'}
              aria-hidden={viewMode !== 'preview'}
            >
              <Preview content={currentNote.body} />
            </div>
          </div>
        {/if}

        <!-- Tag management panel -->
        {#if !isConflict}
          <div class="tag-manager">
            <div class="tag-label">
              <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" class="tag-icon">
                <path d="M11 2.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Z M2.5 7.5L8 2l4.5 4.5L7 12H2.5V7.5z"/>
              </svg>
              <span>Tags:</span>
            </div>

            <div class="tags-container">
              {#each currentNoteTags as tag}
                <span class="tag-pill">
                  #{tag}
                  <button class="remove-tag-btn" onclick={() => removeTag(tag)} aria-label="Remove {tag}">
                    <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                      <line x1="4" y1="4" x2="10" y2="10"/>
                      <line x1="10" y1="4" x2="4" y2="10"/>
                    </svg>
                  </button>
                </span>
              {/each}

              <div class="tag-input-wrapper">
                <input
                  type="text"
                  class="tag-input"
                  placeholder={currentNoteTags.length === 0 ? "Add tags..." : "Add tag..."}
                  bind:value={tagInput}
                  onkeydown={handleTagInputKeyDown}
                  onfocus={() => { showSuggestions = true; }}
                  onblur={() => {
                    setTimeout(() => { showSuggestions = false; }, 200);
                  }}
                />
                
                {#if showSuggestions && suggestions.length > 0}
                  <div class="tag-suggestions" bind:this={suggestionEl}>
                    {#each suggestions as sug, i}
                      <button
                        class="suggestion-item"
                        class:active={i === activeSuggestionIndex}
                        onclick={() => addTag(sug)}
                      >
                        #{sug}
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>

          <!-- Status bar -->
          <StatusBar
            {wordCount}
            {charCount}
            {isDirty}
            {savedAt}
          />
        {/if}
      </div>

      <!-- Geçmiş paneli -->
      {#if displaySnapshots}
        <div class="snapshots-panel">
          <div class="snapshots-header">
            <h3>Version History</h3>
            <button class="icon-close" onclick={() => showSnapshots.set(false)} aria-label="Close">
              <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                <line x1="2" y1="2" x2="12" y2="12"/>
                <line x1="12" y1="2" x2="2" y2="12"/>
              </svg>
            </button>
          </div>

          <div class="snapshots-list scrollbar-thin">
            {#if $loadingEditorMetadata}
              <div class="snap-state">
                <div class="spinner"></div>
                <span>Loading history…</span>
              </div>
            {:else if $snapshotsList.length === 0}
              <div class="snap-state">
                <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
                  <circle cx="16" cy="16" r="13"/>
                  <polyline points="16,9 16,16 20,20"/>
                </svg>
                <p>No snapshots yet.<br/>Auto-created on edits.</p>
              </div>
            {:else}
              {#each $snapshotsList as snap (snap.timestamp)}
                <div class="snap-card">
                  <div class="snap-info">
                    <span class="snap-date">{formatDate(snap.timestamp)}</span>
                    <span class="snap-file">{snap.absolute_path ? snap.absolute_path.split('/').pop() : 'Unknown'}</span>
                  </div>
                  <div class="snap-actions">
                    <button class="restore-btn" onclick={() => openDiff(snap.timestamp)}>
                      Preview Diff
                    </button>
                    <button class="snap-delete-btn" onclick={() => handleDeleteSnapshot(snap.timestamp)} aria-label="Delete snapshot" title="Delete snapshot">
                      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                        <polyline points="2,3.5 12,3.5"/>
                        <path d="M5 3.5V3a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v.5M5.5 6.5v3.5M8.5 6.5v3.5"/>
                        <path d="M3 3.5l.75 7.5a.75.75 0 0 0 .75.75h4.5a.75.75 0 0 0 .75-.75L10.5 3.5"/>
                      </svg>
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if currentDiff && isDiffOpen}
  <DiffViewer 
    diff={currentDiff} 
    isOpen={isDiffOpen} 
    on:close={() => { isDiffOpen = false; currentDiff = null; }}
    on:restore={(e) => handleRestore(e.detail)} 
  />
{/if}

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    flex: 1;
    height: 100%;
    background-color: var(--bg-editor);
    overflow: hidden;
  }

  /* Boş durum */
  .editor-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    height: 100%;
    gap: 12px;
    padding: 40px;
    text-align: center;
  }

  .empty-icon {
    width: 56px;
    height: 56px;
    color: var(--text-disabled);
  }

  .empty-icon svg {
    width: 100%;
    height: 100%;
  }

  .editor-empty h2 {
    color: var(--text-secondary);
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    letter-spacing: -0.2px;
  }

  .editor-empty p {
    color: var(--text-tertiary);
    max-width: 260px;
    font-size: 12px;
    margin: 0;
    line-height: 1.5;
  }

  .editor-empty kbd {
    font-family: var(--font-sans);
    font-size: 11px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-normal);
    border-radius: 3px;
    padding: 1px 5px;
    color: var(--text-secondary);
    font-weight: 500;
  }

  /* Workspace */
  .editor-workspace {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
  }

  .editor-area {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  /* Başlık bar */
  .note-titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    height: 40px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
    background-color: var(--bg-editor);
  }

  .title-spacer {
    flex: 1;
  }

  /* ── Special Mode Banner ── */
  .special-mode-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 100%;
    padding: 0;
  }

  .banner-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .trash-banner .banner-icon { color: var(--color-red, #ff453a); }
  .conflict-banner .banner-icon { color: var(--color-orange, #ff9f0a); }

  .banner-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .banner-title {
    color: var(--text-secondary);
    font-weight: 700;
    text-transform: none;
    letter-spacing: 0;
  }

  .trash-banner .banner-title { color: var(--color-red, #ff453a); }
  .conflict-banner .banner-title { color: var(--color-orange, #ff9f0a); }

  .banner-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .banner-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    padding: 3px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.12s ease;
    white-space: nowrap;
  }

  .banner-btn svg {
    width: 10px;
    height: 10px;
    flex-shrink: 0;
  }

  .restore-btn {
    background-color: var(--accent-muted);
    border-color: var(--accent-border);
    color: var(--accent);
  }
  .restore-btn:hover {
    background-color: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .delete-btn {
    background-color: rgba(255, 69, 58, 0.1);
    border-color: rgba(255, 69, 58, 0.25);
    color: var(--color-red, #ff453a);
  }
  .delete-btn:hover {
    background-color: var(--color-red, #ff453a);
    color: white;
    border-color: var(--color-red, #ff453a);
  }

  .keep-local-btn {
    background-color: var(--accent-muted);
    border-color: var(--accent-border);
    color: var(--accent);
  }
  .keep-local-btn:hover {
    background-color: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .keep-remote-btn {
    background-color: rgba(255, 159, 10, 0.1);
    border-color: rgba(255, 159, 10, 0.25);
    color: var(--color-orange, #ff9f0a);
  }
  .keep-remote-btn:hover {
    background-color: var(--color-orange, #ff9f0a);
    color: white;
    border-color: var(--color-orange, #ff9f0a);
  }



  .note-title-input {
    flex: 2;
    text-align: center;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 4px 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: all 0.15s ease;
    outline: none;
    font-family: var(--font-sans);
  }

  .note-title-input:hover {
    background-color: var(--bg-hover);
  }

  .note-title-input:focus {
    background-color: var(--bg-control);
    border-color: var(--border-subtle);
    text-overflow: clip;
  }

  .note-actions {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  /* Kaydedilmemiş değişiklik noktası */
  .unsaved-dot {
    display: block;
    width: 6px;
    height: 6px;
    background-color: var(--color-orange);
    border-radius: 50%;
    animation: pulse-dot 2s ease-in-out infinite;
  }

  @keyframes pulse-dot {
    0%, 100% { opacity: 0.5; transform: scale(1); }
    50%       { opacity: 1;   transform: scale(1.3); }
  }

  /* Save butonu */
  .action-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text-disabled);
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    padding: 3px 7px;
    transition: all 0.12s ease;
    user-select: none;
  }

  .action-btn svg {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .action-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .action-btn:not(:disabled):hover {
    background-color: var(--bg-control);
    border-color: var(--border-subtle);
    color: var(--text-secondary);
  }

  .action-btn.accent {
    color: var(--accent);
    border-color: var(--accent-border);
    background-color: var(--accent-muted);
  }

  .action-btn.accent:hover {
    background-color: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  /* Panel alanı */
  .editor-panels {
    display: flex;
    flex: 1;
    height: 0; /* flex child olduğu için overflow doğru çalışsın */
    overflow: hidden;
  }

  .panel-editor,
  .panel-preview {
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow: hidden;
    position: relative;
    transition: opacity 0.15s ease;
  }

  /* CodeMirror'u DOM'da tutup görünmez yapıyoruz —
     display:none CodeMirror boyutlarını sıfırlar ve yeniden açılınca boş kalır */
  .panel-hidden {
    position: absolute;
    top: 0;
    left: 0;
    width: 0;
    height: 0;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
  }

  /* Geçmiş paneli */
  .snapshots-panel {
    width: 240px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: var(--bg-notelist);
    border-left: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .snapshots-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .snapshots-header h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .icon-close {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
  }

  .icon-close svg { width: 12px; height: 12px; display: block; }

  .icon-close:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .snapshots-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .snap-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 32px 12px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .snap-state svg { width: 24px; height: 24px; opacity: 0.4; }
  .snap-state p { font-size: 11px; margin: 0; line-height: 1.5; }

  .snap-card {
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    padding: 9px 10px;
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: border-color 0.12s ease;
  }

  .snap-card:hover { border-color: var(--accent-border); }

  .snap-info { display: flex; flex-direction: column; gap: 2px; }
  .snap-date { color: var(--text-secondary); font-size: 11px; font-weight: 500; }
  .snap-file { color: var(--text-tertiary); font-size: 10px; font-family: var(--font-mono); }

  .restore-btn {
    align-self: flex-end;
    background-color: var(--accent-muted);
    border: 1px solid var(--accent-border);
    color: var(--accent);
    padding: 3px 9px;
    font-size: 11px;
    font-weight: 600;
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-family: var(--font-sans);
    transition: all 0.12s ease;
  }

  .restore-btn:hover {
    background-color: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .snap-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }

  .snap-delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: rgba(255, 69, 58, 0.08);
    border: 1px solid rgba(255, 69, 58, 0.18);
    color: var(--color-red, #ff453a);
    padding: 3px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.12s ease;
    width: 21px;
    height: 21px;
  }

  .snap-delete-btn:hover {
    background-color: var(--color-red, #ff453a);
    color: white;
    border-color: var(--color-red, #ff453a);
    transform: scale(1.05);
  }

  .snap-delete-btn svg {
    width: 11px;
    height: 11px;
  }

  .spinner {
    width: 16px; height: 16px;
    border: 2px solid var(--border-normal);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  /* ── Tag Manager Styles ── */
  .tag-manager {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    background-color: var(--bg-elevated);
    border-top: 1px solid var(--border-subtle);
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
    position: relative;
    user-select: none;
    box-sizing: border-box;
  }

  .tag-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .tag-icon {
    width: 12px;
    height: 12px;
    color: var(--text-tertiary);
  }

  .tags-container {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    flex: 1;
    overflow: visible;
  }

  .tag-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--accent);
    background-color: var(--accent-muted);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-sm);
    padding: 2px 6px;
    font-weight: 500;
    white-space: nowrap;
    animation: fadeIn 0.12s ease;
    transition: all 0.12s ease;
  }

  .tag-pill:hover {
    border-color: var(--accent);
    transform: translateY(-0.5px);
  }

  .remove-tag-btn {
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    cursor: pointer;
    color: var(--accent);
    opacity: 0.6;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    width: 12px;
    height: 12px;
    transition: all 0.12s ease;
  }

  .remove-tag-btn svg {
    width: 10px;
    height: 10px;
    stroke-width: 2.5;
  }

  .remove-tag-btn:hover {
    opacity: 1;
    background-color: rgba(255, 255, 255, 0.1);
  }

  .tag-input-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    flex: 1;
    min-width: 120px;
  }

  .tag-input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    font-size: 12px;
    color: var(--text-primary);
    padding: 4px 6px;
    font-family: var(--font-sans);
  }

  .tag-input::placeholder {
    color: var(--text-disabled);
    font-style: italic;
    font-size: 11px;
  }

  /* Tag Suggestions Dropdown */
  .tag-suggestions {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    z-index: 100;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    min-width: 180px;
    max-height: 200px;
    overflow-y: auto;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    animation: slideUp 0.15s ease;
  }

  .suggestion-item {
    display: flex;
    align-items: center;
    width: 100%;
    background: transparent;
    border: none;
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    transition: all 0.1s ease;
  }

  .suggestion-item:hover,
  .suggestion-item.active {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: scale(0.95); }
    to { opacity: 1; transform: scale(1); }
  }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Conflict Comparison split pane styles */
  .conflict-compare-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    height: 100%;
    overflow: hidden;
    background-color: var(--bg-main, #141415);
  }

  .compare-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .compare-split {
    display: flex;
    gap: 16px;
    flex: 1;
    height: 100%;
    padding: 16px;
    overflow: hidden;
  }

  .compare-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    height: 100%;
    overflow: hidden;
    background-color: var(--bg-surface, #1e1e1f);
    border: 1px solid var(--border-normal, #2c2c2e);
    border-radius: var(--radius-md, 8px);
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  .pane-local {
    border-color: rgba(0, 122, 255, 0.25);
  }
  .pane-local:hover {
    border-color: rgba(0, 122, 255, 0.5);
    box-shadow: 0 0 12px rgba(0, 122, 255, 0.15);
  }

  .pane-remote {
    border-color: rgba(245, 158, 11, 0.25);
  }
  .pane-remote:hover {
    border-color: rgba(245, 158, 11, 0.5);
    box-shadow: 0 0 12px rgba(245, 158, 11, 0.15);
  }

  .pane-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-subtle, #2c2c2e);
    background-color: var(--bg-elevated, #181819);
  }

  .pane-badge {
    font-size: 10.5px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 8px;
    border-radius: 20px;
  }

  .badge-local {
    background-color: rgba(0, 122, 255, 0.12);
    color: #38bdf8;
  }

  .badge-remote {
    background-color: rgba(245, 158, 11, 0.12);
    color: #fbbf24;
  }

  .pane-date {
    font-size: 11px;
    color: var(--text-tertiary, #8e8e93);
  }

  .pane-content {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    gap: 12px;
  }

  .pane-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .pane-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pane-body {
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-secondary);
    padding-top: 8px;
  }

  .pane-footer {
    padding: 12px 16px;
    border-top: 1px solid var(--border-subtle, #2c2c2e);
    background-color: var(--bg-elevated, #181819);
  }

  .btn-keep {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-sans);
    padding: 8px 16px;
    border-radius: var(--radius-sm, 6px);
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-keep-local {
    background-color: var(--accent, #007aff);
    color: #ffffff;
    border-color: var(--accent);
  }
  .btn-keep-local:hover {
    background-color: var(--accent-hover, #0062cc);
    border-color: var(--accent-hover);
  }

  .btn-keep-remote {
    background-color: #fbbf24;
    color: #141415;
    border-color: #fbbf24;
  }
  .btn-keep-remote:hover {
    background-color: #f59e0b;
    border-color: #f59e0b;
    color: #141415;
  }
</style>
