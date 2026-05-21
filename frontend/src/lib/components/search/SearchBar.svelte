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
    setTimeout(() => { if (inputEl) inputEl.focus(); }, 50);
  }

  function closeSearch() {
    isOpen = false;
    localQuery = '';
    executeSearch('');
    activeIndex = -1;
  }

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      if (isOpen) { closeSearch(); } else { openSearch(); }
    }
  }

  onMount(() => window.addEventListener('keydown', handleGlobalKeyDown));
  onDestroy(() => window.removeEventListener('keydown', handleGlobalKeyDown));
</script>

<div class="search-widget">
  <!-- Trigger butonu — toolbar içinde minimal arama -->
  <button class="search-trigger" onclick={openSearch} aria-label="Search notes (⌘K)">
    <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
      <circle cx="6" cy="6" r="4.5"/>
      <line x1="9.5" y1="9.5" x2="13" y2="13"/>
    </svg>
    <span class="trigger-text">Search…</span>
    <kbd class="shortcut">⌘K</kbd>
  </button>

  {#if isOpen}
    <!-- Backdrop -->
    <div class="modal-backdrop" onclick={closeSearch} role="presentation"></div>

    <!-- Spotlight benzeri modal -->
    <div class="search-modal" role="dialog" aria-modal="true" aria-label="Search notes">
      <!-- Arama girişi -->
      <div class="search-input-row">
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" class="modal-search-icon" aria-hidden="true">
          <circle cx="7" cy="7" r="5.5"/>
          <line x1="11" y1="11" x2="15" y2="15"/>
        </svg>
        <input
          bind:this={inputEl}
          type="text"
          placeholder="Search notes…"
          value={localQuery}
          oninput={handleInput}
          onkeydown={handleKeyDown}
          autocomplete="off"
          spellcheck="false"
        />
        {#if localQuery}
          <button
            class="clear-btn"
            onclick={() => { localQuery = ''; executeSearch(''); if (inputEl) inputEl.focus(); }}
            aria-label="Clear search"
          >
            <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
              <line x1="2" y1="2" x2="10" y2="10"/>
              <line x1="10" y1="2" x2="2" y2="10"/>
            </svg>
          </button>
        {/if}
      </div>

      <!-- Sonuçlar -->
      <div class="results-area scrollbar-thin">
        {#if isSearching}
          <div class="state-view">
            <div class="spinner"></div>
            <span>Searching…</span>
          </div>
        {:else if !localQuery}
          <div class="state-view idle">
            <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
              <circle cx="14" cy="14" r="9"/>
              <line x1="21" y1="21" x2="28" y2="28"/>
            </svg>
            <p>Search by title, content, or tags</p>
            <span class="hint">Full-text search powered by SQLite FTS5</span>
          </div>
        {:else if results.length === 0}
          <div class="state-view empty">
            <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
              <circle cx="16" cy="16" r="13"/>
              <line x1="10" y1="10" x2="22" y2="22"/>
            </svg>
            <p>No results for "{localQuery}"</p>
          </div>
        {:else}
          <ul class="results-list" role="listbox">
            {#each results as result, idx}
              <li role="option" aria-selected={idx === activeIndex}>
                <button
                  class="result-item"
                  class:active={idx === activeIndex}
                  onclick={() => handleSelect(result.id)}
                  onmouseenter={() => activeIndex = idx}
                >
                  <div class="result-header">
                    <!-- Dokument ikonu -->
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="doc-icon" aria-hidden="true">
                      <path d="M4 2h5.5L13 5.5V14H4V2z"/>
                      <polyline points="9,2 9,6 13,6"/>
                      <line x1="6" y1="9" x2="11" y2="9"/>
                      <line x1="6" y1="11.5" x2="9.5" y2="11.5"/>
                    </svg>
                    <span class="result-title">{result.title}</span>
                  </div>
                  {#if result.snippet}
                    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                    <p class="result-snippet">{@html result.snippet}</p>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <!-- Footer kısayol bilgisi -->
      <div class="modal-footer">
        <div class="hint-group">
          <kbd>↑↓</kbd><span>Navigate</span>
        </div>
        <div class="hint-group">
          <kbd>↵</kbd><span>Select</span>
        </div>
        <div class="hint-group">
          <kbd>Esc</kbd><span>Close</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* ── Trigger ── */
  .search-widget {
    display: inline-block;
    position: relative;
  }

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

  /* ── Modal backdrop ── */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: var(--overlay-bg);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    z-index: 10000;
  }

  /* ── Search modal — Spotlight benzeri ── */
  .search-modal {
    position: fixed;
    top: 12%;
    left: 50%;
    transform: translateX(-50%);
    width: 580px;
    max-width: 90vw;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-xl);
    z-index: 10001;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-xl);
    animation: slideDown 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* Arama girişi satırı */
  .search-input-row {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-subtle);
    gap: 12px;
  }

  .modal-search-icon {
    width: 18px;
    height: 18px;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .search-input-row input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-size: 16px;
    font-family: var(--font-sans);
    letter-spacing: -0.1px;
    user-select: text;
  }

  .search-input-row input::placeholder {
    color: var(--text-placeholder);
  }

  .clear-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
    flex-shrink: 0;
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

  /* Sonuç alanı */
  .results-area {
    max-height: 360px;
    overflow-y: auto;
    background-color: var(--bg-notelist);
  }

  /* Durum görünümleri */
  .state-view {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 48px 24px;
    text-align: center;
    color: var(--text-tertiary);
  }

  .state-view svg {
    width: 36px;
    height: 36px;
    opacity: 0.35;
  }

  .state-view.idle svg {
    color: var(--accent);
    opacity: 0.4;
  }

  .state-view p {
    font-size: 14px;
    margin: 0;
    color: var(--text-secondary);
  }

  .state-view .hint {
    font-size: 11px;
    color: var(--text-disabled);
  }

  /* Spinner */
  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-normal);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  /* Sonuç listesi */
  .results-list {
    list-style: none;
    margin: 0;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .result-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    background: transparent;
    border: 1px solid transparent;
    padding: 9px 12px;
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
    transition: all 0.1s ease;
    gap: 4px;
  }

  .result-item.active,
  .result-item:hover {
    background-color: var(--bg-selected);
    border-color: var(--accent-border);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .doc-icon {
    width: 14px;
    height: 14px;
    color: var(--text-tertiary);
    flex-shrink: 0;
    display: block;
  }

  .result-item.active .doc-icon,
  .result-item:hover .doc-icon {
    color: var(--accent);
  }

  .result-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .result-item.active .result-title,
  .result-item:hover .result-title {
    color: var(--text-primary);
  }

  .result-snippet {
    margin: 0 0 0 22px;
    font-size: 11px;
    color: var(--text-tertiary);
    line-height: 1.4;
    max-height: 36px;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  /* Snippet eşleşme vurgusu — backend'den gelen <mark> */
  :global(.result-snippet mark) {
    background-color: rgba(10, 132, 255, 0.25);
    color: var(--accent-hover);
    border-radius: 2px;
    padding: 0 2px;
    font-weight: 600;
  }

  /* Footer */
  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 14px;
    padding: 10px 18px;
    border-top: 1px solid var(--border-subtle);
    background-color: var(--bg-window);
  }

  .hint-group {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-disabled);
  }

  .hint-group kbd {
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-normal);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: var(--font-sans);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-tertiary);
    line-height: 1.5;
  }
</style>
