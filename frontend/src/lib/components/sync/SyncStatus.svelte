<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { syncStatus, triggerSyncNow } from '../../stores/sync';

  $: status = $syncStatus;
  let syncing = false;

  let now = Date.now();
  let intervalId: any;

  onMount(() => {
    intervalId = setInterval(() => { now = Date.now(); }, 15000);
  });

  onDestroy(() => {
    if (intervalId) clearInterval(intervalId);
  });

  async function handleSyncClick() {
    if (status.status === 'Syncing' || syncing) return;
    syncing = true;
    try {
      await triggerSyncNow();
    } catch (e) {
      console.error('Manual sync failed:', e);
    } finally {
      syncing = false;
    }
  }

  function formatTime(isoStr: string | undefined, _now: number): string {
    if (!isoStr) return 'Never';
    const d = new Date(isoStr);
    const diffMins = Math.floor((_now - d.getTime()) / 60000);
    if (diffMins < 1)   return 'Just now';
    if (diffMins < 60)  return `${diffMins}m ago`;
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }

  $: isBusy = status.status === 'Syncing' || syncing;
  $: statusClass = isBusy ? 'syncing' : status.status === 'Error' ? 'error' : 'idle';
</script>

<button
  class="sync-badge {statusClass}"
  class:clickable={!isBusy}
  onclick={handleSyncClick}
  disabled={isBusy}
  title={
    isBusy ? 'Syncing…'
    : status.status === 'Error' ? `Error: ${status.error_message ?? 'Unknown'}. Click to retry.`
    : `Last sync: ${formatTime(status.last_sync_time, now)}. Click to sync now.`
  }
  aria-label="Sync status: {status.status}"
>
  <!-- İkon -->
  <span class="sync-icon" class:spin={isBusy}>
    {#if status.status === 'Error' && !isBusy}
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
        <circle cx="7" cy="7" r="5.5"/>
        <line x1="7" y1="4.5" x2="7" y2="7.5"/>
        <line x1="7" y1="9.5" x2="7" y2="9.5" stroke-width="2.4"/>
      </svg>
    {:else}
      <!-- Sync/ok ikonu -->
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M13 7A6 6 0 1 1 7 1"/>
        <polyline points="10,1 13,1 13,4"/>
      </svg>
    {/if}
  </span>

  <!-- Metin — sadece syncing/error durumunda göster -->
  {#if isBusy || status.status === 'Error'}
    <span class="sync-label">
      {isBusy ? 'Syncing…' : 'Sync Error'}
    </span>
  {/if}
</button>

<style>
  .sync-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 8px;
    border-radius: var(--radius-pill);
    font-family: var(--font-sans);
    font-size: 11px;
    font-weight: 500;
    background-color: transparent;
    border: 1px solid transparent;
    color: var(--text-tertiary);
    user-select: none;
    transition: all 0.15s ease;
    cursor: default;
    line-height: 1;
  }

  .sync-badge:disabled { cursor: default; }

  .sync-badge.clickable {
    cursor: pointer;
  }

  .sync-badge.clickable:hover {
    background-color: var(--bg-control);
    border-color: var(--border-subtle);
    color: var(--text-secondary);
  }

  .sync-badge.syncing {
    color: var(--accent);
    background-color: var(--accent-muted);
    border-color: var(--accent-border);
  }

  .sync-badge.error {
    color: var(--color-red);
    background-color: var(--color-red-muted);
    border-color: rgba(255, 69, 58, 0.30);
    cursor: pointer;
  }

  .sync-badge.error:hover {
    background-color: rgba(255, 69, 58, 0.22);
    border-color: rgba(255, 69, 58, 0.50);
  }

  .sync-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .sync-icon svg {
    width: 13px;
    height: 13px;
    display: block;
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  .sync-label { white-space: nowrap; }
</style>
