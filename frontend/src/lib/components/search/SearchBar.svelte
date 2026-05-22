<script lang="ts">
  import { searchModalOpen } from '../../stores/search';

  function openSearch() {
    searchModalOpen.set(true);
  }
</script>

<button class="search-trigger" onclick={openSearch} aria-label="Search notes (⌘K)">
  <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
    <circle cx="6" cy="6" r="4.5"/>
    <line x1="9.5" y1="9.5" x2="13" y2="13"/>
  </svg>
  <span class="trigger-text">Search…</span>
  <kbd class="shortcut">⌘K</kbd>
</button>

<style>
  .search-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    height: 28px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    color: var(--text-tertiary);
    cursor: pointer;
    font-family: var(--font-sans);
    font-size: 12px;
    transition: all 0.12s ease;
    user-select: none;
    min-width: 160px;
  }

  .search-trigger svg {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
  }

  .search-trigger:hover {
    background-color: var(--bg-control-hover);
    border-color: var(--border-normal);
    color: var(--text-secondary);
  }

  .trigger-text {
    flex: 1;
    text-align: left;
  }

  .shortcut {
    font-family: var(--font-sans);
    font-size: 11px;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-normal);
    border-radius: 3px;
    padding: 0 4px;
    color: var(--text-disabled);
    line-height: 1.5;
    font-weight: 500;
  }
</style>
