<script lang="ts">
  import { notesList, activeNote } from '../../stores/notes';
  import NoteListItem from './NoteListItem.svelte';
  import VirtualList from '../common/VirtualList.svelte';

  let localFilter = '';

  $: notes = $notesList;
  $: activeId = $activeNote ? $activeNote.id : null;

  $: filteredNotes = notes.filter((n) => {
    if (!localFilter.trim()) return true;
    const term = localFilter.toLowerCase();
    const titleMatch = (n.title || '').toLowerCase().includes(term);
    const tagMatch = n.tags ? n.tags.some((t) => t.toLowerCase().includes(term)) : false;
    return titleMatch || tagMatch;
  });

  function clearFilter() {
    localFilter = '';
  }
</script>

<div class="note-list-panel">
  <!-- Header: başlık + sayaç -->
  <div class="list-header">
    <div class="title-row">
      <h2 class="panel-title">Notes</h2>
      <span class="count-badge" aria-label="{notes.length} notes">{notes.length}</span>
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
</style>
