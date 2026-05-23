<script lang="ts">
  import { syncConflicts } from '../../stores/sync';

  $: conflicts = $syncConflicts;
  let showModal = false;

  function closeConflicts() { showModal = false; }
  function toggleConflicts() { if (conflicts.length > 0) { showModal = !showModal; } }
  function getFilename(path: string): string { return path.split('/').pop() || path; }
</script>

{#if conflicts.length > 0}
  <div class="conflict-container">
    <button
      class="conflict-btn"
      onclick={toggleConflicts}
      title="{conflicts.length} sync conflict{conflicts.length > 1 ? 's' : ''}"
      aria-label="{conflicts.length} sync conflicts"
      aria-expanded={showModal}
    >
      <!-- Uyarı üçgeni -->
      <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M7 1.5L13 12.5H1L7 1.5z"/>
        <line x1="7" y1="6" x2="7" y2="9"/>
        <line x1="7" y1="11" x2="7" y2="11" stroke-width="2.4"/>
      </svg>
      <span class="conflict-count">{conflicts.length}</span>
    </button>

    {#if showModal}
      <div class="dropdown-backdrop" onclick={closeConflicts} role="presentation"></div>
      <div class="dropdown-panel" role="dialog" aria-modal="true" aria-label="Sync conflicts">
        <div class="panel-header">
          <h3>Sync Conflicts</h3>
          <button class="close-btn" onclick={closeConflicts} aria-label="Close">
            <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
              <line x1="2" y1="2" x2="10" y2="10"/>
              <line x1="10" y1="2" x2="2" y2="10"/>
            </svg>
          </button>
        </div>

        <div class="panel-body scrollbar-thin">
          <p class="panel-desc">
            Your local version was kept. The remote conflicting version was archived in
            <code>.noda/conflicts/</code>.
          </p>

          <div class="conflict-list">
            {#each conflicts as conflict}
              <div class="conflict-item">
                <div class="conflict-info">
                  <span class="conflict-name">{conflict.title}</span>
                  <span class="conflict-detail">Archived as:</span>
                  <span class="conflict-path">{getFilename(conflict.archived_path)}</span>
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
  .conflict-container {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  /* Kompakt uyarı butonu */
  .conflict-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 7px;
    border-radius: var(--radius-pill);
    font-family: var(--font-sans);
    font-size: 11px;
    font-weight: 600;
    background-color: var(--color-orange-muted);
    border: 1px solid rgba(255, 159, 10, 0.30);
    color: var(--color-orange);
    cursor: pointer;
    user-select: none;
    transition: all 0.12s ease;
    line-height: 1;
  }

  .conflict-btn svg {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    display: block;
  }

  .conflict-btn:hover {
    background-color: rgba(255, 159, 10, 0.22);
    border-color: rgba(255, 159, 10, 0.50);
  }

  .conflict-count {
    font-variant-numeric: tabular-nums;
  }

  /* Backdrop */
  .dropdown-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
  }

  /* Dropdown paneli */
  .dropdown-panel {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 300px;
    max-height: 360px;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-lg);
    z-index: 10000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow-lg);
    animation: slideDown 0.16s cubic-bezier(0.16, 1, 0.3, 1);
    transform-origin: top right;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 3px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
  }

  .close-btn svg {
    width: 10px;
    height: 10px;
    display: block;
  }

  .close-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 14px;
  }

  .panel-desc {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    margin: 0 0 12px 0;
  }

  .panel-desc code {
    font-family: var(--font-mono);
    font-size: 11px;
    background-color: var(--bg-control);
    border-radius: 3px;
    padding: 1px 4px;
    color: var(--text-secondary);
  }

  .conflict-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .conflict-item {
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 9px 10px;
  }

  .conflict-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .conflict-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .conflict-detail {
    font-size: 10px;
    color: var(--text-tertiary);
    margin-top: 3px;
  }

  .conflict-path {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--color-orange);
    word-break: break-all;
  }
</style>
