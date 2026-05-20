<script lang="ts">
  import { syncStatus, triggerSyncNow, syncConflicts } from '../../stores/sync';

  $: status = $syncStatus;
  $: conflicts = $syncConflicts;

  async function handleSyncClick() {
    if (status.status === 'Syncing') return;
    try {
      await triggerSyncNow();
    } catch (e) {
      console.error('Manual sync failed:', e);
    }
  }

  function formatDate(isoStr?: string) {
    if (!isoStr) return 'Never';
    const d = new Date(isoStr);
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="sync-status-badge {status.status.toLowerCase()}" class:clickable={status.status !== 'Syncing'} onclick={handleSyncClick}>
  <div class="icon-container" class:spin={status.status === 'Syncing'}>
    {#if status.status === 'Syncing'}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
        <path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
      </svg>
    {:else if status.status === 'Error'}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4 text-rose-400">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
      </svg>
    {:else}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
        <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
      </svg>
    {/if}
  </div>

  <div class="sync-info">
    <span class="status-text">
      {#if status.status === 'Syncing'}
        Syncing...
      {:else if status.status === 'Error'}
        Sync Error
      {:else}
        Synced
      {/if}
    </span>
    <span class="time-text">
      {status.status === 'Syncing' ? 'Updating cloud' : `Last: ${formatDate(status.last_sync_time)}`}
    </span>
  </div>
</div>

<style>
  .sync-status-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 9999px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 0.75rem;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    color: #94a3b8;
    user-select: none;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .clickable {
    cursor: pointer;
  }

  .clickable:hover {
    background-color: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.1);
    color: #f1f5f9;
  }

  .sync-status-badge.syncing {
    background-color: rgba(99, 102, 241, 0.05);
    border-color: rgba(99, 102, 241, 0.2);
    color: #818cf8;
  }

  .sync-status-badge.error {
    background-color: rgba(244, 63, 94, 0.05);
    border-color: rgba(244, 63, 94, 0.2);
    color: #f43f5e;
  }

  .icon-container {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spin {
    animation: spin 1.2s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .sync-info {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    line-height: 1.2;
  }

  .status-text {
    font-weight: 600;
  }

  .time-text {
    font-size: 0.65rem;
    opacity: 0.7;
  }
</style>
