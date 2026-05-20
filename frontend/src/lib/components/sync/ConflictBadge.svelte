<script lang="ts">
  import { syncConflicts } from '../../stores/sync';

  $: conflicts = $syncConflicts;
  let showModal = false;

  function closeConflicts() {
    showModal = false;
  }

  function toggleConflicts() {
    if (conflicts.length > 0) {
      showModal = !showModal;
    }
  }

  function getFilename(path: string): string {
    return path.split('/').pop() || path;
  }
</script>

{#if conflicts.length > 0}
  <div class="conflict-badge-container">
    <button class="conflict-trigger hover-glow" onclick={toggleConflicts}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4">
        <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126ZM12 15.75h.007v.008H12v-.008Z" />
      </svg>
      <span>{conflicts.length} Sync Conflict{conflicts.length > 1 ? 's' : ''}</span>
    </button>

    {#if showModal}
      <div class="conflicts-overlay" onclick={closeConflicts}></div>
      <div class="conflicts-dropdown border-glow">
        <div class="dropdown-header">
          <h3>Sync Conflicts Resolved</h3>
          <button class="close-btn" onclick={closeConflicts}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4.5 h-4.5">
              <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="dropdown-body scrollbar-thin">
          <p class="explanation">
            To prevent data loss, the sync engine has safely preserved both versions. Your local version remains active, and the remote conflicting version has been archived in the vault's conflict directory.
          </p>

          <div class="conflict-list">
            {#each conflicts as conflict}
              <div class="conflict-item">
                <div class="conflict-file-info">
                  <span class="file-name">{conflict.filename}</span>
                  <span class="detail-label">Saved remote copy to:</span>
                  <span class="archive-path">{getFilename(conflict.remote_path)}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .conflict-badge-container {
    position: relative;
    display: inline-block;
  }

  .conflict-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 9999px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 0.75rem;
    font-weight: 600;
    background-color: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
    color: #f59e0b;
    cursor: pointer;
    user-select: none;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .conflict-trigger:hover {
    background-color: rgba(245, 158, 11, 0.2);
    border-color: rgba(245, 158, 11, 0.5);
    box-shadow: 0 0 12px rgba(245, 158, 11, 0.25);
  }

  .conflicts-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 999;
  }

  .conflicts-dropdown {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 320px;
    max-height: 400px;
    background-color: #0f131c;
    border-radius: 12px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.5);
    transform-origin: top right;
    animation: scaleIn 0.18s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .border-glow {
    border: 1px solid rgba(245, 158, 11, 0.2);
  }

  @keyframes scaleIn {
    from {
      opacity: 0;
      transform: scale(0.95) translateY(-4px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .dropdown-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background-color: rgba(255, 255, 255, 0.01);
  }

  .dropdown-header h3 {
    margin: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: #cbd5e1;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #475569;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    color: #f8fafc;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .dropdown-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .explanation {
    font-size: 0.75rem;
    line-height: 1.4;
    color: #94a3b8;
    margin: 0 0 16px 0;
  }

  .conflict-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .conflict-item {
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    padding: 10px;
  }

  .conflict-file-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .file-name {
    font-size: 0.8rem;
    font-weight: 500;
    color: #f8fafc;
  }

  .detail-label {
    font-size: 0.65rem;
    color: #475569;
    margin-top: 4px;
  }

  .archive-path {
    font-size: 0.7rem;
    font-family: monospace;
    color: #f59e0b;
    word-break: break-all;
  }

  /* Sleek Scrollbar */
  .scrollbar-thin::-webkit-scrollbar {
    width: 4px;
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
