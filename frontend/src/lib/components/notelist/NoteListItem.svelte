<script lang="ts">
  import type { NoteListItemDto } from '../../types';
  import { selectNote, renameNoteById, activeNote } from '../../stores/notes';
  import { sendNoteToTrash } from '../../stores/editor';

  export let item: NoteListItemDto;
  export let active: boolean = false;

  // Bağlam menüsü state
  let contextMenuVisible = false;
  let contextMenuX = 0;
  let contextMenuY = 0;
  let renaming = false;
  let renameValue = '';
  let renameInputEl: HTMLInputElement;

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
    if (!renaming) selectNote(item.id);
  }

  // ── Bağlam menüsü ───────────────────────────────────────

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();

    // Viewport sınırlarını aşmayı önle
    const menuWidth = 180;
    const menuHeight = 130;
    contextMenuX = Math.min(e.clientX, window.innerWidth  - menuWidth  - 8);
    contextMenuY = Math.min(e.clientY, window.innerHeight - menuHeight - 8);
    contextMenuVisible = true;
  }

  function closeContextMenu() {
    contextMenuVisible = false;
  }

  function handleRename() {
    closeContextMenu();
    renameValue = item.title || 'Untitled';
    renaming = true;
    setTimeout(() => {
      renameInputEl?.focus();
      renameInputEl?.select();
    }, 30);
  }

  async function commitRename() {
    renaming = false;
    const trimmed = renameValue.trim();
    if (trimmed && trimmed !== item.title) {
      try {
        await renameNoteById(item.id, trimmed);
      } catch (e) {
        console.error('Rename failed:', e);
      }
    }
  }

  function cancelRename() {
    renaming = false;
    renameValue = '';
  }

  function handleRenameKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); commitRename(); }
    else if (e.key === 'Escape') { e.preventDefault(); cancelRename(); }
  }

  async function handleTrash() {
    closeContextMenu();
    try {
      await sendNoteToTrash(item.id);
    } catch (e) {
      console.error('Trash failed:', e);
    }
  }

  function handleCopyId() {
    closeContextMenu();
    navigator.clipboard.writeText(item.id).catch(() => {});
  }
</script>

<!-- Global context menu kapat overlay -->
{#if contextMenuVisible}
  <div
    class="context-overlay"
    onclick={closeContextMenu}
    oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    role="presentation"
  ></div>
{/if}

<button
  class="note-item"
  class:active
  onclick={handleSelect}
  oncontextmenu={handleContextMenu}
  aria-current={active ? 'true' : undefined}
  aria-label={item.title || 'Untitled'}
>
  <span class="active-bar" aria-hidden="true"></span>

  <div class="note-content">
    {#if renaming}
      <!-- Yeniden adlandırma girişi — item içinde inline -->
      <input
        class="rename-input"
        bind:this={renameInputEl}
        bind:value={renameValue}
        onblur={commitRename}
        onkeydown={handleRenameKeyDown}
        onclick={(e) => e.stopPropagation()}
        aria-label="Rename note"
        autocomplete="off"
        spellcheck="false"
      />
    {:else}
      <div class="note-header">
        <span class="note-title">{item.title || 'Untitled'}</span>
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
    {/if}
  </div>
</button>

<!-- Bağlam menüsü — konumlanmış portal -->
{#if contextMenuVisible}
  <div
    class="context-menu"
    style="top: {contextMenuY}px; left: {contextMenuX}px;"
    role="menu"
    aria-label="Note options"
  >
    <button class="ctx-item" role="menuitem" onclick={handleRename}>
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M9.5 2.5 11.5 4.5 5 11H3v-2L9.5 2.5z"/>
      </svg>
      Rename
    </button>

    <div class="ctx-sep" role="separator"></div>

    <button class="ctx-item" role="menuitem" onclick={handleCopyId}>
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="5" y="5" width="8" height="8" rx="1.5"/>
        <path d="M9 5V3a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h2"/>
      </svg>
      Copy Note ID
    </button>

    <div class="ctx-sep" role="separator"></div>

    <button class="ctx-item ctx-danger" role="menuitem" onclick={handleTrash}>
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <polyline points="2,3.5 12,3.5"/>
        <path d="M4.5 3.5V2.5a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1v1M5 6v4M9 6v4"/>
        <path d="M2.5 3.5l.7 8a1 1 0 0 0 1 .9h5.6a1 1 0 0 0 1-.9l.7-8"/>
      </svg>
      Move to Trash
    </button>
  </div>
{/if}

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

  /* ── Rename input ── */
  .rename-input {
    width: 100%;
    background-color: var(--bg-elevated);
    border: 1.5px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 3px 6px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
    font-weight: 600;
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-muted);
    user-select: text;
  }

  /* ── Bağlam menüsü overlay ── */
  .context-overlay {
    position: fixed;
    inset: 0;
    z-index: 9998;
  }

  /* ── Bağlam menüsü ── */
  .context-menu {
    position: fixed;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    z-index: 9999;
    min-width: 180px;
    padding: 4px;
    animation: slideDown 0.12s ease;
  }

  .ctx-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    transition: all 0.1s ease;
    user-select: none;
  }

  .ctx-item svg {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    display: block;
  }

  .ctx-item:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .ctx-danger { color: var(--color-red); }
  .ctx-danger:hover { background-color: var(--color-red-muted); color: var(--color-red); }

  .ctx-sep {
    height: 1px;
    background-color: var(--border-subtle);
    margin: 3px 6px;
  }
</style>
