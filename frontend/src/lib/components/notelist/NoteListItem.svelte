<script lang="ts">
  import type { NoteListItemDto } from '../../types';
  import { selectNote } from '../../stores/notes';

  export let item: NoteListItemDto;
  export let active: boolean = false;

  function formatDate(isoStr: string) {
    const d = new Date(isoStr);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    
    return d.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric'
    });
  }

  function handleSelect() {
    selectNote(item.id);
  }
</script>

<button
  class="note-list-item-card"
  class:active={active}
  onclick={handleSelect}
>
  <div class="note-card-border-glow"></div>
  <div class="note-card-content">
    <div class="note-card-header">
      <span class="note-title">{item.title || 'Untitled'}</span>
      <span class="note-time">{formatDate(item.updated)}</span>
    </div>

    {#if item.tags && item.tags.length > 0}
      <div class="tags-row">
        {#each item.tags.slice(0, 3) as tag}
          <span class="tag-pill">#{tag}</span>
        {/each}
        {#if item.tags.length > 3}
          <span class="tag-pill overflow">+{item.tags.length - 3}</span>
        {/if}
      </div>
    {:else}
      <div class="tags-row empty">
        <span class="no-tags">No tags</span>
      </div>
    {/if}
  </div>
</button>

<style>
  .note-list-item-card {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: transparent;
    border: none;
    padding: 0 12px;
    cursor: pointer;
    text-align: left;
    outline: none;
    box-sizing: border-box;
  }

  .note-card-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    width: 100%;
    height: calc(100% - 8px);
    margin-top: 4px;
    padding: 0 12px;
    border-radius: 8px;
    background-color: rgba(255, 255, 255, 0.01);
    border: 1px solid rgba(255, 255, 255, 0.02);
    box-sizing: border-box;
    transition: all 0.2s ease;
  }

  .note-list-item-card:hover .note-card-content {
    background-color: rgba(255, 255, 255, 0.03);
    border-color: rgba(255, 255, 255, 0.06);
  }

  /* Active glow border effect */
  .note-card-border-glow {
    position: absolute;
    left: 12px;
    top: 4px;
    bottom: 4px;
    width: 3px;
    background-color: transparent;
    border-radius: 999px;
    transition: all 0.2s ease;
    z-index: 10;
  }

  .note-list-item-card.active .note-card-border-glow {
    background-color: #6366f1;
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.8);
  }

  .note-list-item-card.active .note-card-content {
    background-color: rgba(99, 102, 241, 0.05);
    border-color: rgba(99, 102, 241, 0.2);
  }

  .note-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 8px;
  }

  .note-title {
    font-size: 0.84rem;
    font-weight: 600;
    color: #cbd5e1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    transition: color 0.2s ease;
  }

  .note-list-item-card:hover .note-title {
    color: #f8fafc;
  }

  .note-list-item-card.active .note-title {
    color: #ffffff;
  }

  .note-time {
    font-size: 0.68rem;
    color: #475569;
    white-space: nowrap;
  }

  .note-list-item-card:hover .note-time {
    color: #64748b;
  }

  .note-list-item-card.active .note-time {
    color: #818cf8;
  }

  .tags-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 6px;
    overflow: hidden;
  }

  .tag-pill {
    font-size: 0.68rem;
    color: #6366f1;
    background-color: rgba(99, 102, 241, 0.08);
    border: 1px solid rgba(99, 102, 241, 0.15);
    border-radius: 4px;
    padding: 1px 5px;
    font-weight: 500;
  }

  .tag-pill.overflow {
    color: #94a3b8;
    background-color: rgba(255, 255, 255, 0.04);
    border-color: rgba(255, 255, 255, 0.08);
  }

  .no-tags {
    font-size: 0.68rem;
    color: #334155;
    font-style: italic;
  }
</style>
