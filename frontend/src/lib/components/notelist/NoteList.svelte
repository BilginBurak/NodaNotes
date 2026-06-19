<script lang="ts">
  import { notesList, activeNote, selectedFolder, draggedItem, moveNote, activeViewMode, selectTrashNote, selectConflictNote, selectedTag } from '../../stores/notes';
  import { resolveAttachmentUrl } from '../../utils/attachment';
  import { get } from 'svelte/store';
  import { trashList, attachmentsWithMetadataList, loadAttachmentsWithMetadata, removeAttachment, triggerQuickLook } from '../../stores/editor';
  import { syncConflicts } from '../../stores/sync';
  import { vaultInfo } from '../../stores/vault';
  import NoteListItem from './NoteListItem.svelte';
  import VirtualList from '../common/VirtualList.svelte';
  import { openContextMenu } from '../../stores/contextMenu';
  import type { TrashEntry, ConflictEntry, NoteListItemDto } from '../../types';

  let localFilter = $state('');

  const notes = $derived($notesList);
  const activeId = $derived($activeNote ? $activeNote.id : null);
  const currentFolder = $derived($selectedFolder);
  const viewMode = $derived($activeViewMode);
  const trashItems = $derived($trashList);
  const conflictItems = $derived($syncConflicts);
  const info = $derived($vaultInfo);
  const attachments = $derived($attachmentsWithMetadataList);



  function insertAttachmentMarkup(name: string) {
    const uri = `noda://attachments/${name}`;
    const isImage = /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(name);
    const markup = isImage ? `![${name}](${uri})` : `[${name}](${uri})`;
    window.dispatchEvent(new CustomEvent('noda:insert-markup', { detail: { markup } }));
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

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Panel title based on view mode
  const panelTitle = $derived(
    viewMode === 'trash' ? 'Deleted Notes' :
    viewMode === 'conflicts' ? 'Sync Conflicts' :
    viewMode === 'attachments' ? 'Attachments' :
    viewMode === 'daily' ? 'Daily Notes' :
    viewMode === 'encrypted' ? 'Encrypted Notes' :
    $selectedTag ? `Tag: #${$selectedTag}` :
    currentFolder === '' ? (info?.name ?? 'Vault Root') :
    currentFolder ? currentFolder.split('/').pop() ?? 'Folder' : 'All Notes'
  );

  // Filtered notes only used in normal, daily or encrypted mode
  const filteredNotes = $derived(viewMode !== 'normal' && viewMode !== 'daily' && viewMode !== 'encrypted' ? [] : notes.filter((n) => {
    if ($selectedTag) {
      const hasYamlTag = n.tags && n.tags.includes($selectedTag);
      const hasInlineTag = n.inline_tags && n.inline_tags.includes($selectedTag);
      if (!hasYamlTag && !hasInlineTag) {
        return false;
      }
    }
    
    if (viewMode === 'daily') {
      if (!n.file_path.startsWith('Daily Notes/')) {
        return false;
      }
    } else if (viewMode === 'encrypted') {
      if (!n.is_encrypted) {
        return false;
      }
    } else if (!$selectedTag) {
      // Normal folder filter only if not viewing all notes for a tag
      if (currentFolder !== null && currentFolder !== '__trash__' && currentFolder !== '__conflicts__' && currentFolder !== '__encrypted__') {
        const lastSlash = n.file_path.lastIndexOf('/');
        const noteDir = lastSlash !== -1 ? n.file_path.substring(0, lastSlash) : '';
        if (noteDir !== currentFolder && !noteDir.startsWith(currentFolder + '/')) {
          return false;
        }
      }
    }
    if (!localFilter.trim()) return true;
    const term = localFilter.toLowerCase();
    const titleMatch = (n.title || '').toLowerCase().includes(term);
    const tagMatch = n.tags ? n.tags.some((t) => t.toLowerCase().includes(term)) : false;
    return titleMatch || tagMatch;
  }).sort((a, b) => {
    if (viewMode === 'daily') {
      return (b.title || '').localeCompare(a.title || '');
    }
    return 0;
  }));

  // Filtered trash items
  const filteredTrash = $derived(viewMode !== 'trash' ? [] : trashItems.filter((n) => {
    if (!localFilter.trim()) return true;
    return (n.title || '').toLowerCase().includes(localFilter.toLowerCase());
  }));

  // Filtered conflict items
  const filteredConflicts = $derived(viewMode !== 'conflicts' ? [] : conflictItems.filter((n) => {
    if (!localFilter.trim()) return true;
    return (n.title || '').toLowerCase().includes(localFilter.toLowerCase());
  }));

  // Filtered attachments
  const filteredAttachments = $derived(viewMode !== 'attachments' ? [] : attachments.filter((a) => {
    if (!localFilter.trim()) return true;
    return a.name.toLowerCase().includes(localFilter.toLowerCase());
  }));

  // Total count for badge
  const totalCount = $derived(
    viewMode === 'trash' ? filteredTrash.length :
    viewMode === 'conflicts' ? filteredConflicts.length :
    viewMode === 'attachments' ? filteredAttachments.length :
    filteredNotes.length
  );

  function clearFilter() {
    localFilter = '';
  }

  function handlePanelContextMenu(e: MouseEvent) {
    if (viewMode !== 'normal') return;
    if (currentFolder !== null) {
      openContextMenu(e, 'folder', currentFolder);
    } else {
      openContextMenu(e, 'root', '');
    }
  }

  let dragOverActive = $state(false);

  $effect(() => {
    if ($draggedItem === null) {
      dragOverActive = false;
    }
  });

  function handleDragOver(e: DragEvent) {
    if (viewMode !== 'normal') return;
    e.preventDefault();
    e.stopPropagation();
    dragOverActive = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.stopPropagation();
    dragOverActive = false;
  }

  async function handleDrop(e: DragEvent) {
    if (viewMode !== 'normal') return;
    e.preventDefault();
    e.stopPropagation();
    dragOverActive = false;

    let data = get(draggedItem);

    if (!data && e.dataTransfer) {
      const rawData = e.dataTransfer.getData('text/plain');
      if (rawData) {
        try {
          data = JSON.parse(rawData);
        } catch (err) {
          console.error('Failed to parse fallback dataTransfer rawData:', err);
        }
      }
    }

    if (!data) return;

    try {
      if (data.type === 'note') {
        await moveNote(data.id, currentFolder || '');
      }
      draggedItem.set(null);
    } catch (err) {
      console.error('Drop to NoteList failed:', err);
    }
  }
</script>


<div 
  class="note-list-panel" 
  class:drag-over={dragOverActive}
  oncontextmenu={handlePanelContextMenu}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
>
  <!-- Header: başlık + sayaç -->
  <div class="list-header">
    <div class="title-row">
      <h2 class="panel-title" class:title-trash={viewMode === 'trash'} class:title-conflict={viewMode === 'conflicts'}>
        {#if viewMode === 'trash'}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="panel-title-icon" aria-hidden="true">
            <polyline points="2,4 14,4"/>
            <path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1M6 7v5M10 7v5"/>
            <path d="M3 4l1 9a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1l1-9"/>
          </svg>
        {:else if viewMode === 'conflicts'}
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="panel-title-icon" aria-hidden="true">
            <path d="M8 1.5L14 12.5H2L8 1.5z"/>
            <line x1="8" y1="6" x2="8" y2="9"/>
            <circle cx="8" cy="11" r="0.5" fill="currentColor"/>
          </svg>
        {/if}
        {panelTitle}
      </h2>
      <span class="count-badge" aria-label="{totalCount} notes">{totalCount}</span>
    </div>

    <!-- Filtre alanı -->
    <div class="filter-wrap" class:focused={false}>
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" class="filter-icon" aria-hidden="true">
        <circle cx="6" cy="6" r="4.5"/>
        <line x1="9.5" y1="9.5" x2="13" y2="13"/>
      </svg>
      <input
        type="text"
        placeholder="Filter..."
        bind:value={localFilter}
        aria-label="Filter notes"
        autocomplete="off"
        spellcheck="false"
      />
      {#if localFilter}
        <button class="clear-btn" onclick={clearFilter} aria-label="Clear filter">
          <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
            <line x1="2" y1="2" x2="10" y2="10"/>
            <line x1="10" y1="2" x2="2" y2="10"/>
          </svg>
        </button>
      {/if}
    </div>
  </div>

  <!-- Not listesi -->
  <div class="list-body">
    {#if viewMode === 'trash'}
      {#if filteredTrash.length === 0}
        <div class="empty-state">
          <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
            <polyline points="4,8 28,8"/>
            <path d="M10 8V6a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v2M12 14v8M20 14v8"/>
            <path d="M6 8l2 18a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2l2-18"/>
          </svg>
          <p>Trash is empty</p>
        </div>
      {:else}
        <ul class="special-list" role="listbox">
          {#each filteredTrash as entry (entry.id)}
            <li>
              <button 
                class="special-item trash-item" 
                class:active={$activeNote?.id === entry.id}
                onclick={() => selectTrashNote(entry.id)}
                role="option" 
                aria-selected={$activeNote?.id === entry.id}
              >
                <div class="special-item-icon trash-icon" aria-hidden="true">
                  <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
                    <polyline points="2,3.5 12,3.5"/>
                    <path d="M4.5 3.5V3a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v.5M5 6v3.5M9 6v3.5"/>
                    <path d="M3 3.5l.75 7.5a.75.75 0 0 0 .75.75h4.5a.75.75 0 0 0 .75-.75L10.5 3.5"/>
                  </svg>
                </div>
                <div class="special-item-content">
                  <span class="special-item-title">{entry.title || 'Untitled'}</span>
                  <span class="special-item-meta">{entry.deleted_at ? new Date(entry.deleted_at).toLocaleDateString() : ''}</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}

    {:else if viewMode === 'conflicts'}
      {#if filteredConflicts.length === 0}
        <div class="empty-state">
          <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
            <path d="M16 4L28 24H4L16 4z"/>
            <line x1="16" y1="12" x2="16" y2="19"/>
            <circle cx="16" cy="22" r="1"/>
          </svg>
          <p>No sync conflicts</p>
        </div>
      {:else}
        <ul class="special-list" role="listbox">
          {#each filteredConflicts as entry (entry.id)}
            <li>
              <button 
                class="special-item conflict-item"
                class:active={$activeNote?.id === entry.id}
                onclick={() => selectConflictNote(entry.id, entry.archived_path)}
                role="option"
                aria-selected={$activeNote?.id === entry.id}
              >
                <div class="special-item-icon conflict-icon" aria-hidden="true">
                  <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
                    <path d="M7 1.5L13 11.5H1L7 1.5z"/>
                    <line x1="7" y1="5" x2="7" y2="8"/>
                    <circle cx="7" cy="9.5" r="0.4" fill="currentColor"/>
                  </svg>
                </div>
                <div class="special-item-content">
                  <span class="special-item-title">{entry.title || entry.file_path.split('/').pop()}</span>
                  <span class="special-item-meta conflict-date">{entry.detected_at ? new Date(entry.detected_at).toLocaleDateString() : ''}</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}

    {:else if viewMode === 'attachments'}
      {#if filteredAttachments.length === 0}
        <div class="empty-state">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true" style="width: 32px; height: 32px; color: var(--text-disabled); margin-bottom: 8px;">
            <path d="M2 13V3a1 1 0 0 1 1-1h6l4 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
            <polyline points="8 2 8 6 12 6"/>
          </svg>
          <p>No attachments found</p>
        </div>
      {:else}
        <ul class="special-list attachments-list" role="listbox">
          {#each filteredAttachments as attachment (attachment.name)}
            {@const isImg = /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(attachment.name)}
            <li>
              <div 
                class="special-item attachment-item"
                onclick={() => triggerQuickLook(attachment.name)}
                role="option"
                aria-selected={false}
                draggable="true"
                ondragstart={(e) => e.dataTransfer && e.dataTransfer.setData('text/noda-attachment', attachment.name)}
              >
                {#if isImg}
                   <div class="attachment-thumb" aria-hidden="true">
                     <img src={resolveAttachmentUrl(`noda://attachments/${attachment.name}`)} alt={attachment.name} loading="lazy" />
                   </div>
                {:else}
                   <div class="attachment-thumb doc-icon" aria-hidden="true">
                     <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                       <path d="M2 13V3a1 1 0 0 1 1-1h6l4 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
                       <polyline points="8 2 8 6 12 6"/>
                     </svg>
                   </div>
                {/if}
                <div class="special-item-content">
                  <span class="special-item-title" title={attachment.name}>{attachment.name}</span>
                  <span class="special-item-meta">{formatBytes(attachment.size)}</span>
                </div>
                
                <!-- Hover actions -->
                <div class="attachment-card-actions">
                  <button 
                    class="item-action-btn insert-text-btn" 
                    onclick={(e) => { e.stopPropagation(); insertAttachmentMarkup(attachment.name); }} 
                    title="Insert into active note"
                  >
                    Insert
                  </button>
                  <button 
                    class="item-action-btn delete-icon-btn" 
                    onclick={(e) => { e.stopPropagation(); handleDeleteAttachment(attachment.name); }} 
                    title="Delete attachment"
                  >
                    <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                      <polyline points="2,3.5 12,3.5"/>
                      <path d="M5 3.5V3a.5.5 0 0 1 .5-.5h3a.5.5 0 0 1 .5.5v.5M5.5 6.5v3.5M8.5 6.5v3.5"/>
                      <path d="M3 3.5l.75 7.5a.75.75 0 0 0 .75.75h4.5a.75.75 0 0 0 .75-.75L10.5 3.5"/>
                    </svg>
                  </button>
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}

    {:else}
      {#if filteredNotes.length === 0}
        <div class="empty-state">
          {#if localFilter}
            <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
              <circle cx="14" cy="14" r="9"/>
              <line x1="21" y1="21" x2="28" y2="28"/>
            </svg>
            <p>No results for "{localFilter}"</p>
          {:else}
            <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M6 10a4 4 0 0 1 4-4h12a4 4 0 0 1 4 4v14a4 4 0 0 1-4 4H10a4 4 0 0 1-4-4V10z"/>
              <line x1="11" y1="14" x2="21" y2="14"/>
              <line x1="11" y1="18" x2="17" y2="18"/>
            </svg>
            <p>No notes yet.<br/>Create one from the toolbar.</p>
          {/if}
        </div>
      {:else}
        <VirtualList items={filteredNotes} itemHeight={62} let:item>
          <NoteListItem {item} active={item.id === activeId} />
        </VirtualList>
      {/if}
    {/if}
  </div>
</div>



<style>
  .note-list-panel {
    width: 240px;
    height: 100%;
    background-color: var(--bg-notelist);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
    transition: background-color 0.15s ease, border-color 0.15s ease;
    box-sizing: border-box;
  }

  .note-list-panel.drag-over {
    background-color: var(--accent-muted);
    outline: 2px dashed var(--accent);
    outline-offset: -2px;
  }

  /* Header */
  .list-header {
    padding: 12px 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-title {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.2px;
  }

  /* Not sayısı badge */
  .count-badge {
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    color: var(--text-tertiary);
    font-size: 11px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: var(--radius-pill);
    line-height: 1.6;
  }

  /* Filtre alanı */
  .filter-wrap {
    display: flex;
    align-items: center;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 4px 8px;
    gap: 6px;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .filter-wrap:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }

  .filter-icon {
    width: 13px;
    height: 13px;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .filter-wrap input {
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
    width: 100%;
    user-select: text;
  }

  .filter-wrap input::placeholder {
    color: var(--text-placeholder);
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 2px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: all 0.12s ease;
  }

  .clear-btn svg {
    width: 10px;
    height: 10px;
    display: block;
  }

  .clear-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  /* Gövde */
  .list-body {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  /* Boş durum */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 48px 16px;
    text-align: center;
    color: var(--text-tertiary);
    height: 100%;
    box-sizing: border-box;
  }

  .empty-state svg {
    width: 32px;
    height: 32px;
    opacity: 0.35;
  }

  .empty-state p {
    font-size: 12px;
    margin: 0;
    line-height: 1.5;
  }

  /* ── Panel Title ── */
  .panel-title {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .panel-title-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    opacity: 0.7;
  }

  .title-trash {
    color: var(--color-red, #ff453a);
  }

  .title-conflict {
    color: var(--color-orange, #ff9f0a);
  }

  /* ── Special Lists (Trash / Conflicts) ── */
  .special-list {
    list-style: none;
    margin: 0;
    padding: 6px 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    height: 100%;
    box-sizing: border-box;
  }

  .special-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    background: transparent;
    border: 1px solid transparent;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
    transition: all 0.1s ease;
  }

  .special-item:hover {
    background-color: var(--bg-hover);
    border-color: var(--border-subtle);
  }

  .special-item.active {
    background-color: var(--bg-selected);
    border-color: var(--border-normal);
  }

  .special-item-icon {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .special-item-icon svg {
    width: 14px;
    height: 14px;
  }

  .trash-icon {
    background-color: var(--color-red-muted, rgba(255, 69, 58, 0.15));
    color: var(--color-red, #ff453a);
  }

  .conflict-icon {
    background-color: var(--color-orange-muted, rgba(255, 159, 10, 0.15));
    color: var(--color-orange, #ff9f0a);
  }

  .special-item-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow: hidden;
    flex: 1;
  }

  .special-item-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.1s ease;
  }

  .special-item:hover .special-item-title,
  .special-item.active .special-item-title {
    color: var(--text-primary);
  }

  .special-item-meta {
    font-size: 10px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .conflict-date {
    color: var(--color-orange, #ff9f0a);
    opacity: 0.8;
  }

  .list-body {
    overflow-y: auto;
  }

  /* ── Attachment Item Styling ── */
  .attachment-item {
    position: relative;
    user-select: none;
  }

  .attachment-thumb {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    overflow: hidden;
    flex-shrink: 0;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .attachment-thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .attachment-thumb.doc-icon svg {
    width: 14px;
    height: 14px;
    color: var(--text-tertiary);
  }

  /* Action buttons shown on hover */
  .attachment-card-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s ease;
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    background: var(--bg-notelist);
    padding-left: 8px;
    box-shadow: -8px 0 8px var(--bg-notelist);
  }

  .attachment-item:hover .attachment-card-actions {
    opacity: 1;
    pointer-events: auto;
  }

  .item-action-btn {
    height: 22px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s ease;
    font-family: var(--font-sans);
  }

  .insert-text-btn {
    background-color: var(--accent);
    border: none;
    color: white;
    padding: 0 8px;
  }

  .insert-text-btn:hover {
    background-color: var(--accent-hover);
  }

  .delete-icon-btn {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-tertiary);
    width: 22px;
  }

  .delete-icon-btn svg {
    width: 11px;
    height: 11px;
    display: block;
  }

  .delete-icon-btn:hover {
    color: var(--color-red, #ff453a);
    background-color: var(--color-red-muted);
    border-color: rgba(255, 69, 58, 0.2);
  }
</style>

