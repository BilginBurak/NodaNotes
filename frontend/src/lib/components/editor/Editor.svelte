<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { getEditorExtensions } from './extensions';
  import DiffViewer from '../history/DiffViewer.svelte';
  import {
    activeNote, updateActiveNoteBody, saveActiveNote, renameActiveNote,
    activeNoteDirty, lastSavedAt, updateActiveNoteTags, notesList
  } from '../../stores/notes';
  import {
    editorViewMode, snapshotsList, showSnapshots,
    loadNoteSnapshots, restoreNoteSnapshot, loadingEditorMetadata
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

  $: currentNote = $activeNote;
  $: viewMode    = $editorViewMode;
  $: displaySnapshots = $showSnapshots;
  $: isDirty    = $activeNoteDirty;
  $: savedAt    = currentNote ? new Date(currentNote.updated_at) : null;

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

  // Not değişince editörü güncelle — editor boşluğunu önlemek için
  // CodeMirror setState kullanıyoruz; hidden/visible değişimi requestMeasure ile handle ediliyor
  $: if (currentNote && currentNote.id !== lastNoteId) {
    lastNoteId = currentNote.id;
    if (editorView) {
      const state = EditorState.create({
        doc: currentNote.body,
        extensions: getEditorExtensions(handleDocChange),
      });
      editorView.setState(state);
      // Yeniden ölçüm — hidden durumdan dönülürse boyutları düzeltir
      setTimeout(() => editorView?.requestMeasure(), 10);
    }
    updateStats(currentNote.body);
  }

  // viewMode değişince CodeMirror boyutlarını yeniden ölçtür
  $: if (editorView && viewMode !== 'preview') {
    setTimeout(() => editorView?.requestMeasure(), 0);
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
      extensions: getEditorExtensions(handleDocChange),
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
          <div class="title-spacer"></div> <!-- Sol tarafta flex boşluğu -->
          
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
        </div>

        <!-- Panel alanı -->
        <div class="editor-panels" class:split={viewMode === 'split'}>
          <div
            class="panel-editor"
            class:panel-hidden={viewMode === 'preview'}
            use:editorAction
            aria-hidden={viewMode === 'preview'}
          ></div>

          {#if viewMode === 'split'}
            <div class="panel-divider" role="separator"></div>
          {/if}

          <div
            class="panel-preview"
            class:panel-hidden={viewMode === 'edit'}
            aria-hidden={viewMode === 'edit'}
          >
            <Preview content={currentNote.body} />
          </div>
        </div>

        <!-- Tag management panel -->
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
                  <button class="restore-btn" onclick={() => openDiff(snap.timestamp)}>
                    Preview Diff
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

  .panel-divider {
    width: 1px;
    background-color: var(--border-subtle);
    flex-shrink: 0;
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
</style>
