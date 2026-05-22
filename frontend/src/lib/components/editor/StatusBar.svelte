<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let wordCount: number = 0;
  export let charCount: number = 0;
  export let isDirty: boolean = false;
  export let savedAt: Date | null = null;

  let now = Date.now();
  let intervalId: any;

  onMount(() => {
    intervalId = setInterval(() => { now = Date.now(); }, 15000); // 15 saniyede bir güncelle
  });

  onDestroy(() => {
    if (intervalId) clearInterval(intervalId);
  });

  function formatRelativeTime(date: Date | null, _now: number): string {
    if (!date) return 'Never saved';
    const diffMs = _now - date.getTime();
    const diffSecs = Math.floor(diffMs / 1000);
    const diffMins = Math.floor(diffSecs / 60);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffSecs < 10)  return 'Just saved';
    if (diffSecs < 60)  return `Saved ${diffSecs}s ago`;
    if (diffMins < 60)  return `Saved ${diffMins}m ago`;
    if (diffHours < 24) return `Saved ${diffHours}h ago`;
    return `Saved ${diffDays}d ago`;
  }
</script>

<div class="status-bar" role="status" aria-label="Document status">
  <!-- Sol: kelime ve karakter sayısı -->
  <div class="status-group">
    <span class="status-item" title="Word count">
      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true">
        <line x1="1" y1="3" x2="11" y2="3"/>
        <line x1="1" y1="6" x2="9" y2="6"/>
        <line x1="1" y1="9" x2="7" y2="9"/>
      </svg>
      {wordCount.toLocaleString()} words
    </span>
    <span class="status-sep" aria-hidden="true">·</span>
    <span class="status-item" title="Character count">
      {charCount.toLocaleString()} chars
    </span>
  </div>

  <!-- Sağ: kayıt bilgisi -->
  <div class="status-group">
    <!-- Kayıt durumu -->
    <span
      class="status-item"
      class:dirty={isDirty}
      class:saved={!isDirty && savedAt !== null}
      title={isDirty ? 'Unsaved changes' : 'All changes saved'}
    >
      {#if isDirty}
        <svg viewBox="0 0 10 10" fill="currentColor" aria-hidden="true">
          <circle cx="5" cy="5" r="4"/>
        </svg>
        Unsaved
      {:else}
        {formatRelativeTime(savedAt, now)}
      {/if}
    </span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
    padding: 0 16px;
    background-color: var(--bg-notelist);
    border-top: 1px solid var(--border-subtle);
    flex-shrink: 0;
    user-select: none;
  }

  .status-group {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-disabled);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .status-item svg {
    width: 10px;
    height: 10px;
    flex-shrink: 0;
  }

  .status-sep {
    color: var(--text-disabled);
    font-size: 10px;
    opacity: 0.5;
  }

  /* Kaydedilmemiş değişiklik */
  .status-item.dirty {
    color: var(--color-orange);
  }

  .status-item.saved {
    color: var(--color-green);
    opacity: 0.8;
  }
</style>
