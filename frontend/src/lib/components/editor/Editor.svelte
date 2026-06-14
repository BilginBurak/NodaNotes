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
    selectNote, focusEditorAtEnd
  } from '../../stores/notes';
  import { recoverFromTrash, emptyTrashPermanently } from '../../stores/editor';
  import { resolveKeepLocal, resolveKeepRemote } from '../../stores/sync';
  import {
    editorViewMode, snapshotsList, showSnapshots,
    loadNoteSnapshots, restoreNoteSnapshot, deleteNoteSnapshot, loadingEditorMetadata,
    showAttachments, attachmentsList, removeAttachment, attachmentsWithMetadataList, loadAttachmentsWithMetadata
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
  let lastSnapshotTime = Date.now();

  let currentDiff: import('../../types').SnapshotDiffDto | null = null;
  let isDiffOpen = false;

  let showInfoPopover = false;
  let metadata: import('../../types').NoteMetadataDto | null = null;
  let loadingMetadata = false;

  function toggleSnapshots() {
    showSnapshots.update(v => {
      const newVal = !v;
      if (newVal && currentNote) {
        loadNoteSnapshots(currentNote.id).catch(console.error);
      }
      return newVal;
    });
  }

  function toggleInfoPopover(e: MouseEvent) {
    e.stopPropagation();
    showInfoPopover = !showInfoPopover;
    if (showInfoPopover) {
      loadNoteMetadata();
    }
  }

  async function loadNoteMetadata() {
    if (!currentNote) return;
    loadingMetadata = true;
    try {
      metadata = await ipc.getNoteMetadata(currentNote.id);
    } catch (e) {
      console.error('Failed to load note metadata:', e);
    } finally {
      loadingMetadata = false;
    }
  }

  $: if (showInfoPopover && currentNote) {
    loadNoteMetadata();
  }

  function formatFullDate(isoStr: string): string {
    if (!isoStr) return '';
    return new Date(isoStr).toLocaleString(undefined, {
      year: 'numeric', month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit', second: '2-digit'
    });
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

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
  $: displayAttachments = $showAttachments;
  $: attachments = $attachmentsList;
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
  $: currentNoteInlineTags = currentNote?.inline_tags || [];

  // Extract all unique tags in other notes
  $: allExistingTags = Array.from(
    new Set(
      ($notesList || []).flatMap(note => [...(note.tags || []), ...(note.inline_tags || [])])
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

  function focusInlineTag(tag: string) {
    if (!editorView) return;
    const docText = editorView.state.doc.toString();
    const searchStr = `#${tag}`;
    const index = docText.indexOf(searchStr);
    if (index !== -1) {
      editorView.focus();
      editorView.dispatch({
        selection: { anchor: index, head: index + searchStr.length },
        scrollIntoView: true
      });
    }
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

  // focusEditorAtEnd tetiklendiğinde kürsörü en sona al ve odakla
  $: if ($focusEditorAtEnd && editorView && currentNote) {
    focusEditorAtEnd.set(false);
    setTimeout(() => {
      if (editorView) {
        const docLength = editorView.state.doc.length;
        editorView.dispatch({
          selection: { anchor: docLength, head: docLength },
          scrollIntoView: true
        });
        editorView.focus();
      }
    }, 50);
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
        const intervalMins = $appConfig?.history?.snapshot_interval_mins ?? 5;
        const now = Date.now();
        const timeDiffMins = (now - lastSnapshotTime) / 60000;
        
        if (timeDiffMins >= intervalMins) {
          await saveActiveNote(true);
          lastSnapshotTime = now;
        } else {
          await saveActiveNote(false);
        }
      } catch (err) {
        console.error('Auto-save failed:', err);
      }
    }, delay);
  }

  async function handleManualSave() {
    if (saveTimeout) clearTimeout(saveTimeout);
    try {
      await saveActiveNote(true, 'Manual');
      lastSnapshotTime = Date.now();
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

  // ── RICH EDITOR FUNCTIONALITY ──
  
  function insertFormatting(type: string) {
    if (!editorView) return;
    const state = editorView.state;
    const mainSelection = state.selection.main;
    const { from, to } = mainSelection;
    const selectedText = state.doc.sliceString(from, to);

    let replacement = '';
    let cursorOffset = 0;

    switch (type) {
      case 'bold':
        replacement = `**${selectedText || 'bold'}**`;
        cursorOffset = selectedText ? replacement.length : 2;
        break;
      case 'italic':
        replacement = `*${selectedText || 'italic'}*`;
        cursorOffset = selectedText ? replacement.length : 1;
        break;
      case 'inline-code':
        replacement = `\`${selectedText || 'code'}\``;
        cursorOffset = selectedText ? replacement.length : 1;
        break;
      case 'code-block':
        replacement = `\n\`\`\`\n${selectedText || 'code'}\n\`\`\`\n`;
        cursorOffset = selectedText ? replacement.length + 5 : 5;
        break;
      case 'link':
        replacement = `[${selectedText || 'link text'}](https://)`;
        cursorOffset = selectedText ? replacement.length - 1 : 1;
        break;
      case 'blockquote':
        return toggleLinePrefix('> ');
      case 'h1':
        return toggleLinePrefix('# ');
      case 'h2':
        return toggleLinePrefix('## ');
      case 'h3':
        return toggleLinePrefix('### ');
      case 'bullet-list':
        return toggleLinePrefix('- ');
      case 'number-list':
        return toggleLinePrefix('1. ');
      case 'todo-list':
        return toggleLinePrefix('- [ ] ');
    }

    editorView.dispatch({
      changes: { from, to, insert: replacement },
      selection: { anchor: from + cursorOffset },
      scrollIntoView: true
    });
    editorView.focus();
  }

  function toggleLinePrefix(prefix: string) {
    if (!editorView) return;
    const state = editorView.state;
    const mainSelection = state.selection.main;
    const from = mainSelection.from;
    const line = state.doc.lineAt(from);
    
    const lineText = line.text;
    let newText = '';
    let changeOffset = 0;

    if (lineText.startsWith(prefix)) {
      newText = lineText.slice(prefix.length);
      changeOffset = -prefix.length;
    } else {
      newText = prefix + lineText;
      changeOffset = prefix.length;
    }

    editorView.dispatch({
      changes: { from: line.from, to: line.to, insert: newText },
      selection: { anchor: mainSelection.anchor + changeOffset },
      scrollIntoView: true
    });
    editorView.focus();
  }

  async function handleImportMarkdown() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        filters: [{ name: 'Markdown / Text', extensions: ['md', 'txt'] }],
        multiple: false
      });
      if (selected) {
        const currentFolder = $selectedFolder;
        const { loadNotes, selectNote } = await import('../../stores/notes');
        const importedNote = await ipc.importNote(selected, currentFolder);
        await loadNotes();
        await selectNote(importedNote.id);
      }
    } catch (err: any) {
      console.error('Failed to import markdown file:', err);
      alert('Markdown import failed: ' + (err.message || err));
    }
  }

  async function handleAddAttachment() {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        filters: [{ name: 'All Files (*.*)', extensions: ['*'] }],
        multiple: false
      });
      if (selected) {
        const uri = await ipc.addAttachment(selected);
        await loadAttachmentsWithMetadata(); // Reload attachments panel list!
        const fileName = uri.split('/').pop() || 'file';
        const isImage = /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(fileName);
        
        let markdownMarkup = '';
        if (isImage) {
          markdownMarkup = `![${fileName}](${uri})`;
        } else {
          markdownMarkup = `[${fileName}](${uri})`;
        }

        if (editorView) {
          const mainSelection = editorView.state.selection.main;
          const { from, to } = mainSelection;
          editorView.dispatch({
            changes: { from, to, insert: markdownMarkup },
            selection: { anchor: from + markdownMarkup.length },
            scrollIntoView: true
          });
          editorView.focus();
        }
      }
    } catch (err: any) {
      console.error('Failed to add attachment:', err);
      alert('Failed to add attachment: ' + (err.message || err));
    }
  }

  function handleInsertAttachmentMarkup(name: string) {
    if (!editorView) return;
    const uri = `noda://attachments/${name}`;
    const isImage = /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(name);
    
    let markdownMarkup = '';
    if (isImage) {
      markdownMarkup = `![${name}](${uri})`;
    } else {
      markdownMarkup = `[${name}](${uri})`;
    }

    const mainSelection = editorView.state.selection.main;
    const { from, to } = mainSelection;
    editorView.dispatch({
      changes: { from, to, insert: markdownMarkup },
      selection: { anchor: from + markdownMarkup.length },
      scrollIntoView: true
    });
    editorView.focus();
  }

  async function handleDeleteAttachment(name: string) {
    if (confirm(`Are you sure you want to permanently delete the attachment "${name}"? This cannot be undone.`)) {
      try {
        await removeAttachment(name);
      } catch (err) {
        console.error('Failed to delete attachment:', err);
        alert('Failed to delete attachment.');
      }
    }
  }

  // ── Drag & Drop variables and handlers ──
  let isDraggingFile = false;
  let showRecentDropdown = false;

  function toggleRecentDropdown(e: MouseEvent) {
    e.stopPropagation();
    showRecentDropdown = !showRecentDropdown;
    if (showRecentDropdown) {
      loadAttachmentsWithMetadata();
    }
  }

  function handleWindowClick() {
    showRecentDropdown = false;
    showInfoPopover = false;
  }

  function handleInsertMarkupEvent(e: Event) {
    const detail = (e as CustomEvent).detail;
    if (detail && detail.markup && editorView) {
      const mainSelection = editorView.state.selection.main;
      const { from, to } = mainSelection;
      editorView.dispatch({
        changes: { from, to, insert: detail.markup },
        selection: { anchor: from + detail.markup.length },
        scrollIntoView: true
      });
      editorView.focus();
    }
  }

  function handleEditorDragOver(e: DragEvent) {
    if (isReadOnly) return;
    if (e.dataTransfer && (e.dataTransfer.types.includes('Files') || e.dataTransfer.types.includes('text/noda-attachment'))) {
      e.preventDefault();
      e.stopPropagation();
      if (e.dataTransfer.types.includes('Files')) {
        isDraggingFile = true;
      }
    }
  }

  function handleEditorDragLeave(e: DragEvent) {
    e.stopPropagation();
    isDraggingFile = false;
  }

  async function handleEditorDrop(e: DragEvent) {
    if (isReadOnly) return;
    e.preventDefault();
    e.stopPropagation();
    isDraggingFile = false;

    if (e.dataTransfer) {
      const internalAttachment = e.dataTransfer.getData('text/noda-attachment');
      if (internalAttachment) {
        handleInsertAttachmentMarkup(internalAttachment);
        return;
      }

      if (e.dataTransfer.files.length > 0) {
        const files = Array.from(e.dataTransfer.files);
        const mdFiles = files.filter(f => f.name.endsWith('.md') || f.name.endsWith('.txt'));
        const otherFiles = files.filter(f => !f.name.endsWith('.md') && !f.name.endsWith('.txt'));

        if (mdFiles.length > 0) {
          const currentFolder = $selectedFolder;
          for (const file of mdFiles) {
            try {
              const text = await file.text();
              const originalName = file.name.replace(/\.md$|\.txt$/, '');
              const imported = await ipc.importNoteFromContent(originalName, text, currentFolder);
              await saveActiveNote();
              await selectNote(imported.id);
            } catch (err) {
              console.error('Failed to drag-import note:', err);
              alert('Failed to import dropped note: ' + file.name);
            }
          }
        }

        if (otherFiles.length > 0) {
          for (const file of otherFiles) {
            try {
              const buffer = await file.arrayBuffer();
              const bytesArray = Array.from(new Uint8Array(buffer));
              const uri = await ipc.addAttachmentBytes(file.name, bytesArray);
              await loadAttachmentsWithMetadata(); // Reload attachments panel list!
            
            const isImage = /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(file.name);
            let markdownMarkup = '';
            if (isImage) {
              markdownMarkup = `![${file.name}](${uri})`;
            } else {
              markdownMarkup = `[${file.name}](${uri})`;
            }

            if (editorView) {
              const mainSelection = editorView.state.selection.main;
              const { from, to } = mainSelection;
              editorView.dispatch({
                changes: { from, to, insert: markdownMarkup },
                selection: { anchor: from + markdownMarkup.length },
                scrollIntoView: true
              });
              editorView.focus();
            }
          } catch (err) {
            console.error('Failed to drag-upload attachment:', err);
            alert('Failed to upload attachment: ' + file.name);
          }
        }
      }
    }
  }
  }

  const handleGlobalDragEnd = () => {
    isDraggingFile = false;
  };

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeyDown);
    window.addEventListener('click', handleWindowClick);
    window.addEventListener('noda:insert-markup', handleInsertMarkupEvent);
    window.addEventListener('dragend', handleGlobalDragEnd);
    window.addEventListener('drop', handleGlobalDragEnd, true);
  });

  onDestroy(() => {
    if (saveTimeout) clearTimeout(saveTimeout);
    window.removeEventListener('keydown', handleGlobalKeyDown);
    window.removeEventListener('click', handleWindowClick);
    window.removeEventListener('noda:insert-markup', handleInsertMarkupEvent);
    window.removeEventListener('dragend', handleGlobalDragEnd);
    window.removeEventListener('drop', handleGlobalDragEnd, true);
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

              <button
                class="action-btn"
                class:active-btn={displaySnapshots}
                onclick={toggleSnapshots}
                title="Version history"
                aria-pressed={displaySnapshots}
              >
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <circle cx="8" cy="8" r="6.5"/>
                  <polyline points="8,4.5 8,8 10.5,10"/>
                </svg>
                History
              </button>

              <div class="note-info-wrapper">
                <button
                  class="action-btn"
                  class:active-btn={showInfoPopover}
                  onclick={toggleInfoPopover}
                  title="Note Info"
                  aria-pressed={showInfoPopover}
                >
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <circle cx="8" cy="8" r="7"/>
                    <line x1="8" y1="11" x2="8" y2="8"/>
                    <line x1="8" y1="5" x2="8.01" y2="5" stroke-width="2"/>
                  </svg>
                  Info
                </button>

                {#if showInfoPopover}
                  <div class="note-info-popover scrollbar-thin" onclick={(e) => e.stopPropagation()}>
                    <div class="popover-header">
                      <h4>Note Information</h4>
                      <button class="popover-close-btn" onclick={() => showInfoPopover = false} aria-label="Close">✕</button>
                    </div>
                    {#if loadingMetadata}
                      <div class="popover-loading">
                        <div class="spinner"></div>
                        <span>Loading metadata…</span>
                      </div>
                    {:else if metadata}
                      <div class="popover-body">
                        <div class="info-row">
                          <span class="info-label">Title:</span>
                          <span class="info-value">{metadata.title || 'Untitled'}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">File Name:</span>
                          <span class="info-value code-font">{metadata.file_name}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">Relative Path:</span>
                          <span class="info-value code-font">{metadata.relative_path}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">Absolute Path:</span>
                          <span class="info-value code-font">{metadata.absolute_path}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">Created At:</span>
                          <span class="info-value">{formatFullDate(metadata.created_at)}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">Last Saved:</span>
                          <span class="info-value">{formatFullDate(metadata.updated_at)}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">Sync Status:</span>
                          <span class="info-value">
                            {#if metadata.last_upload_time}
                              Uploaded to Cloud ({formatFullDate(metadata.last_upload_time)})
                            {:else}
                              Not yet synced
                            {/if}
                          </span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">History Snapshots:</span>
                          <span class="info-value">{metadata.history_count} versions stored</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">File Size:</span>
                          <span class="info-value">{formatBytes(metadata.file_size_bytes)}</span>
                        </div>
                        <div class="info-row">
                          <span class="info-label">Statistics:</span>
                          <span class="info-value">{metadata.word_count} words · {metadata.char_count} chars</span>
                        </div>
                        {#if metadata.tags && metadata.tags.length > 0}
                          <div class="info-row tags-row">
                            <span class="info-label">Tags:</span>
                            <div class="info-tags">
                              {#each metadata.tags as t}
                                <span class="tag-pill">#{t}</span>
                              {/each}
                            </div>
                          </div>
                        {/if}
                      </div>
                    {:else}
                      <div class="popover-error">Failed to load note details.</div>
                    {/if}
                  </div>
                {/if}
              </div>
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
          <!-- Formatlama araç çubuğu -->
          {#if !isReadOnly && viewMode !== 'preview'}
            <div class="editor-formatting-toolbar">
              <div class="toolbar-group">
                <button class="tool-btn btn-h1" onclick={() => insertFormatting('h1')} title="Heading 1">H1</button>
                <button class="tool-btn btn-h2" onclick={() => insertFormatting('h2')} title="Heading 2">H2</button>
                <button class="tool-btn btn-h3" onclick={() => insertFormatting('h3')} title="Heading 3">H3</button>
              </div>

              <div class="toolbar-divider"></div>

              <div class="toolbar-group">
                <button class="tool-btn" onclick={() => insertFormatting('bold')} title="Bold (⌘B)">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M4 2h5a3.5 3.5 0 0 1 0 7H4V2z"/>
                    <path d="M4 9h6a3.5 3.5 0 0 1 0 7H4V9z"/>
                  </svg>
                </button>
                <button class="tool-btn" onclick={() => insertFormatting('italic')} title="Italic (⌘I)">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="11" y1="2" x2="5" y2="14"/>
                    <line x1="4" y1="2" x2="10" y2="2"/>
                    <line x1="6" y1="14" x2="12" y2="14"/>
                  </svg>
                </button>
              </div>

              <div class="toolbar-divider"></div>

              <div class="toolbar-group">
                <button class="tool-btn" onclick={() => insertFormatting('bullet-list')} title="Bullet List">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="6" y1="3" x2="14" y2="3"/>
                    <line x1="6" y1="8" x2="14" y2="8"/>
                    <line x1="6" y1="13" x2="14" y2="13"/>
                    <circle cx="2" cy="3" r="1" fill="currentColor"/>
                    <circle cx="2" cy="8" r="1" fill="currentColor"/>
                    <circle cx="2" cy="13" r="1" fill="currentColor"/>
                  </svg>
                </button>
                <button class="tool-btn" onclick={() => insertFormatting('number-list')} title="Numbered List">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="6" y1="3" x2="14" y2="3"/>
                    <line x1="6" y1="8" x2="14" y2="8"/>
                    <line x1="6" y1="13" x2="14" y2="13"/>
                    <text x="0" y="10" font-size="8" font-weight="bold" fill="currentColor">1.</text>
                  </svg>
                </button>
                <button class="tool-btn" onclick={() => insertFormatting('todo-list')} title="Checklist">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="2" y="2" width="12" height="12" rx="2"/>
                    <path d="M5 8l2 2 4-4"/>
                  </svg>
                </button>
              </div>

              <div class="toolbar-divider"></div>

              <div class="toolbar-group">
                <button class="tool-btn" onclick={() => insertFormatting('blockquote')} title="Blockquote">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M3 12h3a1.5 1.5 0 0 0 1.5-1.5V7A1.5 1.5 0 0 0 6 5.5H3v6.5zM10 12h3a1.5 1.5 0 0 0 1.5-1.5V7A1.5 1.5 0 0 0 13 5.5h-3v6.5z"/>
                  </svg>
                </button>
                <button class="tool-btn" onclick={() => insertFormatting('inline-code')} title="Inline Code">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="4 11 1 8 4 5"/>
                    <polyline points="12 5 15 8 12 11"/>
                  </svg>
                </button>
                <button class="tool-btn" onclick={() => insertFormatting('code-block')} title="Code Block">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="2" y="2" width="12" height="12" rx="2"/>
                    <path d="M5 6l2 2-2 2M11 10h-3"/>
                  </svg>
                </button>
              </div>

              <div class="toolbar-divider"></div>

              <div class="toolbar-group">
                <button class="tool-btn" onclick={() => insertFormatting('link')} title="Insert Link">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M10 6.5A2.5 2.5 0 0 0 5.75 3L3.5 5.25a2.5 2.5 0 0 0 0 3.5m2.5 1.75a2.5 2.5 0 0 0 4.25 3.5l2.25-2.25a2.5 2.5 0 0 0 0-3.5"/>
                    <line x1="6" y1="10" x2="10" y2="6"/>
                  </svg>
                </button>
                <button class="tool-btn btn-accent" onclick={handleAddAttachment} title="Add Image / Attachment">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M2 13V3a1 1 0 0 1 1-1h6l4 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
                    <polyline points="8 2 8 6 12 6"/>
                    <line x1="8" y1="8" x2="8" y2="12"/>
                    <line x1="6" y1="10" x2="10" y2="10"/>
                  </svg>
                </button>

                <div class="toolbar-dropdown-wrapper">
                  <button class="tool-btn" class:active-btn={showRecentDropdown} onclick={toggleRecentDropdown} title="Recent Attachments">
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M2 13V3a1 1 0 0 1 1-1h6l4 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
                      <polyline points="8 2 8 6 12 6"/>
                    </svg>
                    <span class="chevron-arrow">▼</span>
                  </button>

                  {#if showRecentDropdown}
                    {@const recentAttachments = $attachmentsWithMetadataList.slice(0, 20)}
                    <div class="recent-attachments-dropdown scrollbar-thin" onclick={(e) => e.stopPropagation()}>
                      <div class="dropdown-header">Recent Attachments</div>
                      {#if recentAttachments.length === 0}
                        <div class="dropdown-empty">No attachments yet</div>
                      {:else}
                        <div class="dropdown-list">
                          {#each recentAttachments as item (item.name)}
                            {@const isImg = /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(item.name)}
                            <div 
                              class="dropdown-item"
                              onclick={() => { handleInsertAttachmentMarkup(item.name); showRecentDropdown = false; }}
                              draggable="true"
                              ondragstart={(e) => e.dataTransfer && e.dataTransfer.setData('text/noda-attachment', item.name)}
                            >
                              {#if isImg}
                                <img class="item-preview" src="noda://attachments/{item.name}" alt={item.name} loading="lazy" />
                              {:else}
                                <div class="item-preview doc-preview">
                                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8">
                                    <path d="M2 13V3a1 1 0 0 1 1-1h6l4 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
                                  </svg>
                                </div>
                              {/if}
                              <span class="item-name" title={item.name}>{item.name}</span>
                            </div>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>

              <div style="flex: 1;"></div>

              <div class="toolbar-group">
                <button class="import-note-btn" onclick={handleImportMarkdown} title="Import Markdown/Text file">
                  <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="2" y="2" width="12" height="12" rx="2"/>
                    <polyline points="5 8 8 11 11 8"/>
                    <line x1="8" y1="4" x2="8" y2="11"/>
                  </svg>
                  <span>Import File</span>
                </button>
              </div>
            </div>
          {/if}

          <div 
            class="editor-panels-container"
            ondragover={handleEditorDragOver}
            ondragleave={handleEditorDragLeave}
            ondrop={handleEditorDrop}
          >
            {#if isDraggingFile}
              <div class="editor-drag-overlay">
                <div class="drag-message">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="drag-icon">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="17 8 12 3 7 8" />
                    <line x1="12" y1="3" x2="12" y2="15" />
                  </svg>
                  <h3>Dosyaları Buraya Bırakın</h3>
                  <p>Markdown (.md) veya metin (.txt) dosyaları doğrudan not olarak içe aktarılacaktır. Görseller ve diğer dosyalar ise eklenti (Attachment) olarak eklenecektir.</p>
                </div>
              </div>
            {/if}

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
                <span class="tag-pill" title="YAML Frontmatter Tag">
                  #{tag}
                  <button class="remove-tag-btn" onclick={() => removeTag(tag)} aria-label="Remove {tag}">
                    <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                      <line x1="4" y1="4" x2="10" y2="10"/>
                      <line x1="10" y1="4" x2="4" y2="10"/>
                    </svg>
                  </button>
                </span>
              {/each}

              {#each currentNoteInlineTags as tag}
                <span class="tag-pill tag-pill-inline" title="Inline Tag (Click to focus)" onclick={() => focusInlineTag(tag)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && focusInlineTag(tag)}>
                  #{tag}
                  <span class="tag-source-label">inline</span>
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
                    <div class="snap-meta-row">
                      <span class="snap-date">{formatDate(snap.timestamp)}</span>
                      {#if snap.reason}
                        <span class="snap-reason-badge" class:badge-blur={snap.reason === 'Blur'} class:badge-autosave={snap.reason === 'AutoSave'} class:badge-presync={snap.reason === 'Pre-Sync'} class:badge-appexit={snap.reason === 'App-Exit'}>{snap.reason}</span>
                      {/if}
                    </div>
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

      <!-- Attachments side-panel is removed as they are now fully integrated in the Left note list panel and editor drop-down! -->
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
  .snap-meta-row { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
  .snap-date { color: var(--text-secondary); font-size: 11px; font-weight: 500; }
  .snap-reason-badge {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 4px;
    font-weight: 600;
    text-transform: uppercase;
    background-color: var(--color-orange-muted);
    color: var(--color-orange);
  }
  .snap-reason-badge.badge-blur {
    background-color: hsla(210, 80%, 50%, 0.15);
    color: hsl(210, 80%, 55%);
  }
  .snap-reason-badge.badge-autosave {
    background-color: hsla(140, 70%, 45%, 0.15);
    color: hsl(140, 70%, 50%);
  }
  .snap-reason-badge.badge-presync {
    background-color: hsla(280, 70%, 50%, 0.15);
    color: hsl(280, 70%, 55%);
  }
  .snap-reason-badge.badge-appexit {
    background-color: hsla(0, 70%, 50%, 0.15);
    color: hsl(0, 70%, 55%);
  }
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

  .tag-pill-inline {
    background-color: rgba(142, 142, 147, 0.12);
    border: 1px solid var(--border-normal);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .tag-pill-inline:hover {
    border-color: var(--accent);
    background-color: var(--accent-muted);
    color: var(--accent);
    transform: translateY(-0.5px);
  }

  .tag-source-label {
    font-size: 8px;
    text-transform: uppercase;
    opacity: 0.5;
    background-color: rgba(0, 0, 0, 0.2);
    padding: 0.5px 3px;
    border-radius: 2px;
    margin-left: 2px;
    letter-spacing: 0.3px;
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

  /* ── Formatting Toolbar Styles ── */
  .editor-formatting-toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background-color: var(--toolbar-bg, rgba(30, 30, 30, 0.7));
    backdrop-filter: blur(20px) saturate(1.5);
    -webkit-backdrop-filter: blur(20px) saturate(1.5);
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    flex-shrink: 0;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
    z-index: 10;
    opacity: 0.4;
    transition: opacity 1.5s cubic-bezier(0.25, 1, 0.5, 1), background-color 0.3s cubic-bezier(0.25, 1, 0.5, 1);
  }

  .editor-formatting-toolbar:hover {
    transition: opacity 0.5s cubic-bezier(0.25, 1, 0.5, 1), background-color 0.3s cubic-bezier(0.25, 1, 0.5, 1);

    opacity: 1;
  }

  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 2px;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: var(--radius-sm, 6px);
    padding: 2px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .toolbar-divider {
    width: 1px;
    height: 18px;
    background-color: var(--border-subtle, rgba(255, 255, 255, 0.12));
    margin: 0 4px;
  }

  .tool-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: var(--text-secondary, #cccccc);
    cursor: pointer;
    transition: all 0.1s cubic-bezier(0.4, 0, 0.2, 1);
    font-family: var(--font-sans);
    font-weight: 600;
    font-size: 11px;
  }

  .tool-btn:hover {
    background-color: var(--bg-hover, rgba(255, 255, 255, 0.08));
    color: var(--text-primary, #ffffff);
  }

  .tool-btn:active {
    transform: scale(0.92);
  }

  .tool-btn svg {
    width: 14px;
    height: 14px;
  }

  .tool-btn.btn-h1 { font-size: 12px; font-weight: 800; }
  .tool-btn.btn-h2 { font-size: 11px; font-weight: 700; }
  .tool-btn.btn-h3 { font-size: 10px; font-weight: 600; }

  .tool-btn.btn-accent {
    color: var(--accent, #0a84ff);
  }
  .tool-btn.btn-accent:hover {
    background-color: rgba(10, 132, 255, 0.15);
    color: var(--accent-hover, #2f96ff);
  }

  .import-note-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    background-color: var(--accent, #0a84ff);
    color: #ffffff;
    border: none;
    border-radius: 4px;
    font-family: var(--font-sans);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .import-note-btn:hover {
    background-color: var(--accent-hover, #2f96ff);
    box-shadow: 0 0 8px rgba(10, 132, 255, 0.4);
  }

  .import-note-btn:active {
    transform: scale(0.95);
  }

  .import-note-btn svg {
    width: 12px;
    height: 12px;
  }

  /* ── Drag & Drop Overlay Styles ── */
  .editor-panels-container {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    height: 100%;
  }

  .editor-drag-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(20, 20, 21, 0.88);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    padding: 40px;
    animation: fadeIn 0.18s cubic-bezier(0.4, 0, 0.2, 1);
    pointer-events: none;
  }

  .drag-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 16px;
    max-width: 440px;
    padding: 32px;
    background-color: var(--bg-elevated, #1c1c1e);
    border: 1.5px dashed var(--accent, #0a84ff);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.05);
    animation: slideUp 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .drag-icon {
    width: 48px;
    height: 48px;
    color: var(--accent, #0a84ff);
    animation: bounce 2s infinite ease-in-out;
  }

  .drag-message h3 {
    margin: 0;
    color: var(--text-primary, #ffffff);
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.2px;
  }

  .drag-message p {
    margin: 0;
    color: var(--text-secondary, #aaaaaa);
    font-size: 12px;
    line-height: 1.6;
  }

  @keyframes bounce {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-8px); }
  }


  /* ── Toolbar Dropdown Styles ── */
  .toolbar-dropdown-wrapper {
    position: relative;
    display: inline-block;
  }

  .chevron-arrow {
    font-size: 8px;
    margin-left: 3px;
    opacity: 0.6;
  }

  .tool-btn.active-btn {
    background-color: var(--bg-control-hover);
    color: var(--accent);
  }

  .recent-attachments-dropdown {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    width: 260px;
    max-height: 320px;
    background-color: var(--bg-control);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md, 8px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 6px;
    animation: dropdownSlideDown 0.15s ease;
  }

  @keyframes dropdownSlideDown {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .dropdown-header {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 4px;
  }

  .dropdown-empty {
    padding: 24px;
    text-align: center;
    color: var(--text-disabled);
    font-size: 11px;
  }

  .dropdown-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: var(--radius-sm, 4px);
    cursor: pointer;
    transition: all 0.1s ease;
    user-select: none;
  }

  .dropdown-item:hover {
    background-color: var(--bg-hover);
  }

  .item-preview {
    width: 24px;
    height: 24px;
    border-radius: 3px;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--border-subtle);
  }

  .doc-preview {
    background-color: var(--bg-control-hover);
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .doc-preview svg {
    width: 12px;
    height: 12px;
  }

  .item-name {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .dropdown-item:hover .item-name {
    color: var(--text-primary);
  }

  /* Note Info Wrapper & Popover */
  .note-info-wrapper {
    position: relative;
    display: inline-block;
  }

  .note-info-popover {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    width: 320px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    padding: 12px 14px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 10px;
    animation: slideDown 0.15s cubic-bezier(0.16, 1, 0.3, 1);
    max-height: 400px;
    overflow-y: auto;
    user-select: text;
  }

  .popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 6px;
  }

  .popover-header h4 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .popover-close-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    font-size: 11px;
    padding: 2px 5px;
    border-radius: 3px;
    transition: all 0.12s ease;
  }

  .popover-close-btn:hover {
    color: var(--text-secondary);
    background-color: var(--bg-control);
  }

  .popover-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text-secondary);
    padding: 20px 0;
    font-size: 12px;
  }

  .popover-loading .spinner {
    width: 14px;
    height: 14px;
    border: 1.5px solid var(--border-normal);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  .popover-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .info-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: 6px;
    border-bottom: 1px dashed var(--border-subtle);
  }

  .info-row:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .info-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .info-value {
    font-size: 12px;
    color: var(--text-primary);
    word-break: break-all;
  }

  .info-value.code-font {
    font-family: var(--font-mono);
    font-size: 11px;
    background-color: rgba(255, 255, 255, 0.04);
    padding: 2px 4px;
    border-radius: 3px;
  }

  .tags-row {
    border-bottom: none;
  }

  .info-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 2px;
  }

  .info-tags .tag-pill {
    font-size: 10px;
    padding: 2px 6px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
  }

  .popover-error {
    color: var(--color-red);
    font-size: 12px;
    text-align: center;
    padding: 10px 0;
  }

  .action-btn.active-btn {
    color: var(--accent) !important;
    background-color: var(--accent-muted) !important;
    border-color: var(--accent-border) !important;
  }
</style>
