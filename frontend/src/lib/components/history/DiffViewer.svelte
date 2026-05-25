<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import type { SnapshotDiffDto, DiffChunk } from '../../types';
  import { createEventDispatcher } from 'svelte';

  export let diff: SnapshotDiffDto;
  export let isOpen: boolean;

  const dispatch = createEventDispatcher();

  function close() {
    isOpen = false;
    dispatch('close');
  }

  function restore() {
    dispatch('restore', diff.timestamp);
  }

  function formatDate(iso: string) {
    const d = new Date(iso);
    return d.toLocaleString('tr-TR', { 
      year: 'numeric', month: 'short', day: 'numeric', 
      hour: '2-digit', minute: '2-digit', second: '2-digit' 
    });
  }

  function getChunkClass(tag: string): string {
    if (tag === 'Insert') return 'diff-insert';
    if (tag === 'Delete') return 'diff-delete';
    if (tag === 'Separator') return 'diff-separator';
    return 'diff-equal';
  }
</script>

{#if isOpen}
  <div class="diff-overlay" transition:fade={{ duration: 150 }} onclick={close} onkeydown={(e) => e.key === 'Escape' && close()} role="dialog" aria-modal="true" tabindex="-1">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="diff-modal" transition:scale={{ duration: 200, start: 0.95 }} onclick={(e) => e.stopPropagation()} role="document">
      
      <div class="modal-header">
        <div class="modal-titles">
          <h2>Version History Diff</h2>
          <p class="modal-subtitle">Comparing snapshot <strong>{formatDate(diff.timestamp)}</strong> with current note.</p>
        </div>
        <button class="close-btn" onclick={close} aria-label="Close">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <path d="M1 1l12 12m0-12L1 13"/>
          </svg>
        </button>
      </div>

      <div class="modal-content diff-viewer">
        {#each diff.body_chunks as chunk}
          {#if chunk.text}
            <div class="diff-chunk {getChunkClass(chunk.tag)}">
              {#if chunk.tag === 'Insert'}
                <span class="diff-sign">+</span>
              {:else if chunk.tag === 'Delete'}
                <span class="diff-sign">-</span>
              {:else if chunk.tag === 'Separator'}
                <span class="diff-sign">…</span>
              {:else}
                <span class="diff-sign">&nbsp;</span>
              {/if}
              <pre>{chunk.text}</pre>
            </div>
          {/if}
        {/each}
      </div>

      <div class="modal-footer">
        <button class="btn-cancel" onclick={close}>Cancel</button>
        <button class="btn-restore" onclick={restore}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 8a5 5 0 1 1 5 5v-2a3 3 0 1 0-3-3H3z"/>
            <path d="M1 6l2 2 2-2"/>
          </svg>
          Restore This Version
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .diff-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    padding: 20px;
  }

  .diff-modal {
    background-color: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.2);
    width: 100%;
    max-width: 800px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background-color: var(--bg-secondary);
  }

  .modal-titles h2 {
    margin: 0 0 4px 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .modal-subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 6px;
    border-radius: 4px;
    display: flex;
    transition: all 0.15s;
  }

  .close-btn:hover {
    background-color: var(--bg-control-hover);
    color: var(--text-primary);
  }

  .close-btn svg {
    width: 12px;
    height: 12px;
  }

  .modal-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    background-color: var(--bg-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.5;
  }

  .diff-chunk {
    display: flex;
    width: 100%;
  }

  .diff-chunk pre {
    margin: 0;
    padding: 0 8px;
    white-space: pre-wrap;
    word-break: break-all;
    flex: 1;
  }

  .diff-sign {
    user-select: none;
    width: 20px;
    flex-shrink: 0;
    text-align: center;
    color: var(--text-tertiary);
    font-weight: bold;
    border-right: 1px solid var(--border-subtle);
  }

  .diff-insert {
    background-color: rgba(46, 160, 67, 0.15); /* GitHub green */
    color: #3fb950;
  }
  .diff-insert .diff-sign { color: #3fb950; border-color: rgba(46, 160, 67, 0.4); }

  .diff-delete {
    background-color: rgba(248, 81, 73, 0.15); /* GitHub red */
    color: #ff7b72;
    text-decoration: line-through;
    opacity: 0.8;
  }
  .diff-delete .diff-sign { color: #ff7b72; border-color: rgba(248, 81, 73, 0.4); text-decoration: none; }

  .diff-equal {
    color: var(--text-secondary);
  }

  .diff-separator {
    background-color: var(--bg-secondary);
    color: var(--text-disabled);
    font-style: italic;
    border-top: 1px dashed var(--border-subtle);
    border-bottom: 1px dashed var(--border-subtle);
    margin: 8px 0;
    padding: 4px 0;
    font-weight: 500;
  }
  .diff-separator .diff-sign {
    color: var(--text-disabled);
    border-color: var(--border-subtle);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border-subtle);
    background-color: var(--bg-secondary);
  }

  .btn-cancel, .btn-restore {
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    font-family: var(--font-sans);
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
  }

  .btn-cancel:hover {
    background-color: var(--bg-control-hover);
    color: var(--text-primary);
  }

  .btn-restore {
    background-color: var(--accent);
    border: 1px solid var(--accent);
    color: #ffffff;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-restore svg {
    width: 14px;
    height: 14px;
  }

  .btn-restore:hover {
    background-color: var(--accent-hover);
  }
</style>
