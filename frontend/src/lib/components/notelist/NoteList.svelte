<script lang="ts">
  import { notesList, activeNote } from '../../stores/notes';
  import NoteListItem from './NoteListItem.svelte';
  import VirtualList from '../common/VirtualList.svelte';

  let localFilter = '';

  $: notes = $notesList;
  $: activeId = $activeNote ? $activeNote.id : null;

  // Local filter for quick client-side filtering on title/tags
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

<div class="note-list-panel border-right">
  <!-- Note List Header & Local Filter -->
  <div class="list-header">
    <div class="title-row">
      <h2>Notes</h2>
      <span class="note-count-badge">{notes.length}</span>
    </div>
    
    <div class="filter-input-wrapper">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="search-icon w-3.5 h-3.5">
        <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.637 10.637Z" />
      </svg>
      <input
        type="text"
        placeholder="Filter current list..."
        bind:value={localFilter}
      />
      {#if localFilter}
        <button class="clear-btn" onclick={clearFilter}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class="w-3 h-3">
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
          </svg>
        </button>
      {/if}
    </div>
  </div>

  <!-- Virtual Notes Scroll Area -->
  <div class="list-body">
    {#if filteredNotes.length === 0}
      <div class="list-empty-state">
        {#if localFilter}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-8 h-8">
            <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.637 10.637Z" />
          </svg>
          <p>No results match "{localFilter}"</p>
        {:else}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" class="w-8 h-8">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.042A8.967 8.967 0 0 0 6 3.75c-1.052 0-2.062.18-3 .512v14.25A8.987 8.987 0 0 1 6 18c2.305 0 4.408.867 6 2.292m0-14.25a8.966 8.966 0 0 1 6-2.292c1.052 0 2.062.18 3 .512v14.25A8.987 8.987 0 0 0 18 18a8.967 8.967 0 0 0-6 2.292m0-14.25v14.25" />
          </svg>
          <p>Vault is empty. Create a new note from the toolbar to start writing.</p>
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
    width: 260px;
    height: 100%;
    background-color: #080a0f;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
  }

  .border-right {
    border-right: 1px solid rgba(255, 255, 255, 0.05);
  }

  .list-header {
    padding: 16px 16px 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
  }

  .title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .title-row h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .note-count-badge {
    background-color: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: #94a3b8;
    font-size: 0.7rem;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 9999px;
  }

  .filter-input-wrapper {
    display: flex;
    align-items: center;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    padding: 4px 8px;
    gap: 8px;
    transition: all 0.2s ease;
  }

  .filter-input-wrapper:focus-within {
    border-color: rgba(99, 102, 241, 0.4);
    background-color: rgba(255, 255, 255, 0.03);
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.08);
  }

  .search-icon {
    color: #475569;
  }

  .filter-input-wrapper input {
    background: transparent;
    border: none;
    outline: none;
    color: #e2e8f0;
    font-size: 0.78rem;
    width: 100%;
    font-family: inherit;
  }

  .filter-input-wrapper input::placeholder {
    color: #475569;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: #475569;
    cursor: pointer;
    padding: 2px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .clear-btn:hover {
    color: #f8fafc;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .list-body {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .list-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 16px;
    text-align: center;
    color: #475569;
    height: 100%;
    box-sizing: border-box;
  }

  .list-empty-state svg {
    color: #27272a;
    margin-bottom: 12px;
  }

  .list-empty-state p {
    font-size: 0.75rem;
    margin: 0;
    line-height: 1.4;
  }
</style>
