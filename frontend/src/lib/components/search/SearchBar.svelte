<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { executeSearch, searchQuery, searchResults, searching } from '../../stores/search';
  import { selectNote } from '../../stores/notes';
  import { debounce } from '../../utils/debounce';

  let inputEl: HTMLInputElement;
  let isOpen = false;
  let localQuery = '';
  let activeIndex = -1;

  $: results = $searchResults;
  $: isSearching = $searching;

  const debouncedSearch = debounce((q: string) => {
    executeSearch(q);
  }, 250);

  function handleInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    localQuery = value;
    activeIndex = -1;
    debouncedSearch(value);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeSearch();
      e.preventDefault();
    } else if (e.key === 'ArrowDown') {
      if (results.length > 0) {
        activeIndex = (activeIndex + 1) % results.length;
        e.preventDefault();
      }
    } else if (e.key === 'ArrowUp') {
      if (results.length > 0) {
        activeIndex = (activeIndex - 1 + results.length) % results.length;
        e.preventDefault();
      }
    } else if (e.key === 'Enter') {
      if (activeIndex >= 0 && activeIndex < results.length) {
        handleSelect(results[activeIndex].id);
        e.preventDefault();
      } else if (results.length > 0) {
        handleSelect(results[0].id);
        e.preventDefault();
      }
    }
  }

  function handleSelect(noteId: string) {
    selectNote(noteId);
    closeSearch();
  }

  function openSearch() {
    isOpen = true;
    setTimeout(() => {
      if (inputEl) inputEl.focus();
    }, 50);
  }

  function closeSearch() {
    isOpen = false;
    localQuery = '';
    executeSearch('');
    activeIndex = -1;
  }

  function handleGlobalKeyDown(e: KeyboardEvent) {
    // CMD+K or Ctrl+K triggers search modal
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      if (isOpen) {
        closeSearch();
      } else {
        openSearch();
      }
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleGlobalKeyDown);
  });
</script>

<div class="search-bar-widget">
  <button class="search-trigger hover-glow" onclick={openSearch}>
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
      <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.637 10.637Z" />
    </svg>
    <span class="placeholder-text">Search notes...</span>
    <kbd class="shortcut-key">⌘K</kbd>
  </button>

  {#if isOpen}
    <!-- Backdrop overlay -->
    <div class="search-modal-backdrop" onclick={closeSearch}></div>

    <!-- Search Modal Container -->
    <div class="search-modal border-glow">
      <div class="search-input-wrapper">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="search-icon w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.637 10.637Z" />
        </svg>
        <input
          bind:this={inputEl}
          type="text"
          placeholder="Type to search notes using fuzzy full-text indexing..."
          value={localQuery}
          oninput={handleInput}
          onkeydown={handleKeyDown}
        />
        {#if localQuery}
          <button class="clear-btn" onclick={() => { localQuery = ''; executeSearch(''); if (inputEl) inputEl.focus(); }}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        {/if}
      </div>

      <div class="search-results-panel scrollbar-thin">
        {#if isSearching}
          <div class="search-state-info">
            <div class="spinner"></div>
            <span>Indexing matching notes...</span>
          </div>
        {:else if !localQuery}
          <div class="search-state-info idle">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904L9 21l8.982-11.725h-5.945L13 3 4.018 14.725h5.795z" />
            </svg>
            <p>Find notes instantly. Powered by SQLite FTS5.</p>
            <span class="tip">Type terms to search titles or markdown body</span>
          </div>
        {:else if results.length === 0}
          <div class="search-state-info empty">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
            </svg>
            <p>No matching notes found for "{localQuery}"</p>
          </div>
        {:else}
          <div class="results-list">
            {#each results as result, idx}
              <button
                class="result-item"
                class:active={idx === activeIndex}
                onclick={() => handleSelect(result.id)}
                onmouseenter={() => activeIndex = idx}
              >
                <div class="result-title-row">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="doc-icon w-4 h-4">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
                  </svg>
                  <span class="note-title">{result.title}</span>
                </div>
                {#if result.snippet}
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  <p class="note-snippet">{@html result.snippet}</p>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="search-footer">
        <div class="key-help"><kbd>↑↓</kbd> <span>Navigate</span></div>
        <div class="key-help"><kbd>Enter</kbd> <span>Select</span></div>
        <div class="key-help"><kbd>Esc</kbd> <span>Close</span></div>
      </div>
    </div>
  {/if}
</div>

<style>
  .search-bar-widget {
    display: inline-block;
  }

  .search-trigger {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    height: 32px;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    color: #94a3b8;
    cursor: pointer;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 0.8rem;
    transition: all 0.2s ease;
    user-select: none;
  }

  .search-trigger:hover {
    background-color: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.1);
    color: #f1f5f9;
  }

  .placeholder-text {
    width: 90px;
    text-align: left;
  }

  .shortcut-key {
    font-family: inherit;
    font-size: 0.7rem;
    background-color: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 1px 4px;
    color: #475569;
    font-weight: 500;
  }

  .search-modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(3, 7, 18, 0.6);
    backdrop-filter: blur(4px);
    z-index: 10000;
  }

  .search-modal {
    position: fixed;
    top: 15%;
    left: 50%;
    transform: translateX(-50%);
    width: 600px;
    max-width: 90vw;
    background-color: #0d111a;
    border-radius: 12px;
    z-index: 10001;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7);
    animation: slideDown 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .border-glow {
    border: 1px solid rgba(99, 102, 241, 0.15);
    box-shadow: 0 0 30px rgba(99, 102, 241, 0.05);
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translate(-50%, -20px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0) scale(1);
    }
  }

  .search-input-wrapper {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    gap: 12px;
    background-color: rgba(255, 255, 255, 0.01);
  }

  .search-icon {
    color: #475569;
    flex-shrink: 0;
  }

  .search-input-wrapper input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #f1f5f9;
    font-size: 0.95rem;
    font-family: inherit;
  }

  .search-input-wrapper input::placeholder {
    color: #475569;
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: #475569;
    cursor: pointer;
    padding: 4px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .clear-btn:hover {
    color: #e2e8f0;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .search-results-panel {
    max-height: 380px;
    overflow-y: auto;
    background-color: #090b10;
  }

  .search-state-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 48px 24px;
    color: #64748b;
    text-align: center;
    gap: 8px;
  }

  .search-state-info svg {
    width: 40px;
    height: 40px;
    color: #334155;
    margin-bottom: 8px;
  }

  .search-state-info.idle svg {
    color: rgba(99, 102, 241, 0.3);
  }

  .search-state-info p {
    margin: 0;
    font-size: 0.85rem;
    color: #94a3b8;
  }

  .search-state-info .tip {
    font-size: 0.75rem;
    color: #475569;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid rgba(255, 255, 255, 0.05);
    border-top-color: #6366f1;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: 8px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .results-list {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .result-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    background: transparent;
    border: 1px solid transparent;
    padding: 10px 14px;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .result-item.active {
    background-color: rgba(99, 102, 241, 0.07);
    border-color: rgba(99, 102, 241, 0.2);
  }

  .result-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .doc-icon {
    color: #475569;
  }

  .result-item.active .doc-icon {
    color: #818cf8;
  }

  .note-title {
    font-size: 0.88rem;
    font-weight: 500;
    color: #cbd5e1;
  }

  .result-item.active .note-title {
    color: #f8fafc;
  }

  .note-snippet {
    margin: 4px 0 0 24px;
    font-size: 0.75rem;
    color: #475569;
    line-height: 1.4;
    max-height: 40px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .result-item.active .note-snippet {
    color: #94a3b8;
  }

  /* Style snippet matches coming from backend highlight */
  :global(.note-snippet mark) {
    background-color: rgba(99, 102, 241, 0.3);
    color: #f8fafc;
    border-radius: 2px;
    padding: 0 2px;
    font-weight: 500;
  }

  .search-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 10px 18px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    background-color: rgba(255, 255, 255, 0.01);
    gap: 16px;
  }

  .key-help {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.7rem;
    color: #475569;
  }

  .key-help kbd {
    background-color: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    padding: 1px 4px;
    font-family: inherit;
    font-weight: 500;
  }

  /* Scrollbar styling */
  .scrollbar-thin::-webkit-scrollbar {
    width: 6px;
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
