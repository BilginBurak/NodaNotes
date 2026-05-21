<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { syncStatus, lastSyncReport } from '../../stores/sync';

  export let wordCount: number = 0;
  export let charCount: number = 0;
  export let isDirty: boolean = false;
  export let savedAt: Date | null = null;

  $: status = $syncStatus;
  $: report = $lastSyncReport;

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

    if (diffSecs < 10)  return 'Just saved';
    if (diffSecs < 60)  return `Saved ${diffSecs}s ago`;
    if (diffMins < 60)  return `Saved ${diffMins}m ago`;
    return `Saved at ${date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`;
  }

  function formatSyncTime(isoStr: string | undefined, _now: number): string {
    if (!isoStr) return 'Never synced';
    const d = new Date(isoStr);
    const diffMs = _now - d.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    if (diffMins < 1)   return 'Just synced';
    if (diffMins < 60)  return `Synced ${diffMins}m ago`;
    return `Synced at ${d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}`;
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

  <!-- Sağ: kayıt ve sync bilgisi -->
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

    <span class="status-sep" aria-hidden="true">·</span>

    <!-- Sync durumu -->
    <span
      class="status-item"
      class:syncing={status.status === 'Syncing'}
      class:sync-error={status.status === 'Error'}
      title={status.status === 'Error' ? status.error_message : 'Sync status'}
    >
      {#if status.status === 'Syncing'}
        <span class="sync-spinner" aria-hidden="true"></span>
        Syncing…
      {:else if status.status === 'Error'}
        <svg viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
          <circle cx="5" cy="5" r="4"/>
          <line x1="5" y1="3" x2="5" y2="5.5"/>
          <line x1="5" y1="7" x2="5" y2="7" stroke-width="2"/>
        </svg>
        Sync Error
      {:else}
        {formatSyncTime(status.last_sync_time, now)}
      {/if}
    </span>

    {#if report}
      <span class="status-sep" aria-hidden="true">·</span>
      <span class="status-item sync-summary" title="Last sync: {report.uploads} up, {report.downloads} down">
        ↑{report.uploads} ↓{report.downloads}
        {#if report.conflicts > 0}
          <span class="conflict-indicator">· {report.conflicts}⚠</span>
        {/if}
      </span>
    {/if}
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

  /* Syncing */
  .status-item.syncing {
    color: var(--accent);
  }

  .status-item.sync-error {
    color: var(--color-red);
  }

  /* Küçük dönen sync göstergesi */
  .sync-spinner {
    display: inline-block;
    width: 8px;
    height: 8px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  /* Son sync özeti */
  .sync-summary {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.2px;
  }

  .conflict-indicator {
    color: var(--color-orange);
    font-weight: 600;
  }
</style>
