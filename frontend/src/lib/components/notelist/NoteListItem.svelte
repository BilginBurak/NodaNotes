<script lang="ts">
  import type { NoteListItemDto } from '../../types';
  import { selectNote, activeNote, draggedItem, renamingNote, renameNoteById } from '../../stores/notes';
  import { openContextMenu } from '../../stores/contextMenu';

  let { item, active = false } = $props<{
    item: NoteListItemDto;
    active?: boolean;
  }>();

  function formatDate(isoStr: string): string {
    const d = new Date(isoStr);
    if (isNaN(d.getTime())) return 'Invalid Date';
    const day = String(d.getDate()).padStart(2, '0');
    const month = String(d.getMonth() + 1).padStart(2, '0');
    const year = d.getFullYear();
    const hours = String(d.getHours()).padStart(2, '0');
    const minutes = String(d.getMinutes()).padStart(2, '0');
    return `${day}.${month}.${year} ${hours}.${minutes}`;
  }

  function handleSelect() {
    selectNote(item.id);
  }

  // ── Drag & Drop ─────────────────────────────────────
  function handleDragStart(e: DragEvent) {
    e.stopPropagation();
    draggedItem.set({
      type: 'note',
      id: item.id,
      relPath: item.file_path
    });
    if (e.dataTransfer) {
      e.dataTransfer.setData('text/plain', JSON.stringify({
        type: 'note',
        id: item.id,
        relPath: item.file_path
      }));
      e.dataTransfer.effectAllowed = 'move';
    }
  }

  const isRenaming = $derived($renamingNote === item.id);
  let renameValue = $state('');

  $effect(() => {
    if ($renamingNote === item.id) {
      renameValue = item.title;
    }
  });

  let isSubmitting = false;

  async function commitRename() {
    if (isSubmitting) return;
    isSubmitting = true;
    const val = renameValue.trim();
    if (!val || val === item.title) {
      renamingNote.set(null);
      isSubmitting = false;
      return;
    }
    try {
      await renameNoteById(item.id, val);
    } catch (err: any) {
      alert(err.message || 'Failed to rename note');
    } finally {
      renamingNote.set(null);
      isSubmitting = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Enter') {
      commitRename();
    } else if (e.key === 'Escape') {
      renamingNote.set(null);
    }
  }

  function autoFocus(el: HTMLInputElement) {
    setTimeout(() => {
      el.focus();
      el.select();
    }, 10);
  }
</script>

<div
  class="note-item"
  class:active
  draggable="true"
  ondragstart={handleDragStart}
  onclick={handleSelect}
  oncontextmenu={(e) => openContextMenu(e, 'note', item.file_path, item.id, item.title)}
  role="button"
  tabindex="0"
  onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSelect(); } }}
  aria-current={active ? 'true' : undefined}
  aria-label={item.title || 'Untitled'}
>
  <span class="active-bar" aria-hidden="true"></span>

  <div class="note-content">
    <div class="note-header">
      {#if isRenaming}
        <input
          type="text"
          class="rename-input"
          bind:value={renameValue}
          use:autoFocus
          onclick={(e) => e.stopPropagation()}
          onkeydown={handleKeyDown}
          onblur={commitRename}
        />
      {:else}
        <span class="note-title">{item.title || 'Untitled'}</span>
      {/if}
      <span class="note-date">{formatDate(item.updated_at)}</span>
    </div>

    {#if item.tags && item.tags.length > 0}
      <div class="tags-row">
        {#each item.tags.slice(0, 3) as tag}
          <span class="tag">#{tag}</span>
        {/each}
        {#if item.tags.length > 3}
          <span class="tag tag-more">+{item.tags.length - 3}</span>
        {/if}
      </div>
    {:else}
      <div class="tags-row">
        <span class="no-tags">No tags</span>
      </div>
    {/if}
  </div>
</div>

<style>
  /* ── Note item ── */
  .note-item {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: stretch;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    outline: none;
    box-sizing: border-box;
    border-bottom: 1px solid var(--border-subtle);
    transition: background-color 0.1s ease;
    user-select: none;
    -webkit-user-drag: element;
  }

  .note-item:hover { background-color: var(--bg-hover); }
  .note-item.active { background-color: var(--bg-selected); }

  /* Sol aktif çizgi */
  .active-bar {
    display: block;
    width: 3px;
    background-color: transparent;
    border-radius: 0 var(--radius-pill) var(--radius-pill) 0;
    flex-shrink: 0;
    transition: background-color 0.15s ease;
    align-self: stretch;
    margin: 10px 0;
  }

  .note-item.active .active-bar { background-color: var(--accent); }

  /* İçerik */
  .note-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 10px 14px 10px 10px;
    flex: 1;
    overflow: hidden;
    gap: 4px;
  }

  .note-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .note-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    transition: color 0.1s ease;
    letter-spacing: -0.1px;
  }

  .note-item:hover .note-title,
  .note-item.active .note-title { color: var(--text-primary); }

  .note-date {
    font-size: 10px;
    color: var(--text-tertiary);
    white-space: nowrap;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .note-item.active .note-date { color: var(--accent); opacity: 0.8; }

  /* Tag'lar */
  .tags-row {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    min-height: 16px;
  }

  .tag {
    font-size: 10px;
    color: var(--accent);
    background-color: var(--accent-muted);
    border-radius: 3px;
    padding: 1px 5px;
    font-weight: 500;
    white-space: nowrap;
  }

  .tag-more {
    color: var(--text-tertiary);
    background-color: var(--bg-control);
  }

  .no-tags {
    font-size: 10px;
    color: var(--text-disabled);
    font-style: italic;
  }

  .rename-input {
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    background-color: var(--bg-input, rgba(255, 255, 255, 0.05));
    border: 1.5px solid var(--accent);
    border-radius: var(--radius-sm, 4px);
    padding: 1px 4px;
    margin: -2px -5px;
    width: calc(100% - 20px);
    box-sizing: border-box;
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-muted);
  }
</style>
