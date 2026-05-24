<script lang="ts">
  import { 
    selectedFolder, 
    activeNote, 
    selectNote, 
    moveNote, 
    moveFolder, 
    draggedItem,
    renamingFolder,
    renameFolder,
    renamingNote,
    renameNoteById,
    activeViewMode
  } from '../../stores/notes';
  import { openContextMenu } from '../../stores/contextMenu';
  import type { TreeNode } from '../../types';
  import FolderTreeItem from './FolderTreeItem.svelte';

  let {
    node,
    depth = 0,
    expandedFolders
  } = $props<{
    node: TreeNode;
    depth?: number;
    expandedFolders: Record<string, boolean>;
  }>();

  let dragOverActive = $state(false);

  const isOpen = $derived(expandedFolders[node.relPath] || false);
  const isSelected = $derived(node.type === 'folder' 
    ? $selectedFolder === node.relPath 
    : $activeNote?.id === node.id);

  function handleRowClick(e: MouseEvent) {
    e.stopPropagation();
    if (node.type === 'folder') {
      selectedFolder.set(node.relPath);
      activeViewMode.set('normal');
    } else if (node.type === 'note' && node.id) {
      selectNote(node.id);
      activeViewMode.set('normal');
      const lastSlash = node.relPath.lastIndexOf('/');
      const parentFolder = lastSlash !== -1 ? node.relPath.substring(0, lastSlash) : '';
      selectedFolder.set(parentFolder);
    }
  }

  function toggleExpand(e: MouseEvent) {
    e.stopPropagation();
    expandedFolders[node.relPath] = !isOpen;
  }

  // ── Drag & Drop ───────────────────────────────────────────
  function handleDragStart(e: DragEvent) {
    e.stopPropagation();
    draggedItem.set({
      type: node.type as 'note' | 'folder',
      id: node.id || '',
      relPath: node.relPath
    });
    if (e.dataTransfer) {
      e.dataTransfer.setData('text/plain', JSON.stringify({
        type: node.type,
        id: node.id,
        relPath: node.relPath
      }));
      e.dataTransfer.effectAllowed = 'move';
    }
  }

  function handleDragOver(e: DragEvent) {
    if (node.type !== 'folder') return;
    e.preventDefault();
    e.stopPropagation();
    dragOverActive = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.stopPropagation();
    dragOverActive = false;
  }

  async function handleDrop(e: DragEvent) {
    if (node.type !== 'folder') return;
    e.preventDefault();
    e.stopPropagation();
    dragOverActive = false;

    // Use global draggedItem store for extreme reliability in WKWebView
    let data = $draggedItem;

    if (!data && e.dataTransfer) {
      const rawData = e.dataTransfer.getData('text/plain');
      if (rawData) {
        try {
          data = JSON.parse(rawData);
        } catch (err) {
          console.error('Failed to parse fallback dataTransfer rawData:', err);
        }
      }
    }

    if (!data) return;
    const draggedPath = data.relPath;
    if (!draggedPath) return;

    try {
      if (data.type === 'note') {
        await moveNote(data.id, node.relPath);
      } else if (data.type === 'folder') {
        if (draggedPath === node.relPath || node.relPath.startsWith(draggedPath + '/')) {
          alert('Cannot move a folder into itself or its subfolder.');
          return;
        }
        const folderName = draggedPath.split('/').pop() || '';
        const newPath = node.relPath ? `${node.relPath}/${folderName}` : folderName;
        await moveFolder(draggedPath, newPath);
      }
      draggedItem.set(null);
    } catch (err) {
      console.error('Drop failed:', err);
    }
  }

  const isRenaming = $derived(
    (node.type === 'folder' && $renamingFolder === node.relPath) ||
    (node.type === 'note' && node.id && $renamingNote === node.id)
  );
  let renameValue = $state('');

  $effect(() => {
    if (isRenaming) {
      renameValue = node.name;
    }
  });

  let isSubmitting = false;

  async function commitRename() {
    if (isSubmitting) return;
    isSubmitting = true;
    const val = renameValue.trim();
    if (!val || val === node.name) {
      renamingFolder.set(null);
      renamingNote.set(null);
      isSubmitting = false;
      return;
    }
    try {
      if (node.type === 'folder') {
        if (val.includes('/') || val.includes('\\')) {
          alert('Folder name cannot contain slashes.');
          renamingFolder.set(null);
          isSubmitting = false;
          return;
        }
        await renameFolder(node.relPath, val);
      } else if (node.type === 'note' && node.id) {
        await renameNoteById(node.id, val);
      }
    } catch (err: any) {
      alert(err.message || 'Failed to rename');
    } finally {
      renamingFolder.set(null);
      renamingNote.set(null);
      isSubmitting = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Enter') {
      commitRename();
    } else if (e.key === 'Escape') {
      renamingFolder.set(null);
      renamingNote.set(null);
    }
  }

  function autoFocus(el: HTMLInputElement) {
    setTimeout(() => {
      el.focus();
      el.select();
    }, 10);
  }
</script>

<div
  class="tree-item-row"
  class:selected={isSelected}
  class:drag-over={dragOverActive}
  style="padding-left: {depth * 12 + 8}px"
  draggable="true"
  ondragstart={handleDragStart}
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  onclick={handleRowClick}
  oncontextmenu={(e) => {
    e.preventDefault();
    e.stopPropagation();
    openContextMenu(e, node.type as 'folder' | 'note', node.relPath, node.id || '', node.name);
  }}
  role="treeitem"
  aria-selected={isSelected}
>
  {#if node.type === 'folder'}
    <!-- Folder Row -->
    <button 
      class="chevron-btn" 
      class:open={isOpen} 
      onclick={toggleExpand}
      aria-label={isOpen ? 'Collapse folder' : 'Expand folder'}
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="6 12 10 8 6 4" />
      </svg>
    </button>
    
    <span class="icon folder-icon">
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M1.5 3.5a1 1 0 0 1 1-1h3.586a1 1 0 0 1 .707.293L8.5 4.5H13.5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-8z" />
      </svg>
    </span>

    {#if isRenaming}
      <input
        type="text"
        class="rename-input"
        bind:value={renameValue}
        use:autoFocus
        onclick={(e) => e.stopPropagation()}
        onkeydown={handleKeyDown}
        onblur={commitRename}
      />
    {:else}
      <span class="label text-truncate">{node.name}</span>
    {/if}
  {:else}
    <!-- Note Row -->
    <span class="icon note-icon" style="margin-left: 16px;">
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M2 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5z" />
        <line x1="5" y1="7" x2="11" y2="7" />
        <line x1="5" y1="10" x2="9" y2="10" />
      </svg>
    </span>
    
    {#if isRenaming}
      <input
        type="text"
        class="rename-input"
        bind:value={renameValue}
        use:autoFocus
        onclick={(e) => e.stopPropagation()}
        onkeydown={handleKeyDown}
        onblur={commitRename}
      />
    {:else}
      <span class="label text-truncate">{node.name}</span>
    {/if}
  {/if}
</div>

{#if node.type === 'folder' && isOpen && node.children.length > 0}
  <div class="tree-children" role="group">
    {#each node.children as child (child.type + '-' + child.relPath)}
      <FolderTreeItem 
        node={child} 
        depth={depth + 1} 
        {expandedFolders} 
      />
    {/each}
  </div>
{/if}

<style>
  .tree-item-row {
    display: flex;
    align-items: center;
    padding: 4px 6px;
    margin: 1px 0;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    user-select: none;
    cursor: pointer;
    transition: all 0.12s ease;
    border: 2px dashed transparent;
    -webkit-user-drag: element;
  }

  .tree-item-row:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .tree-item-row.selected {
    color: var(--accent);
    background-color: var(--accent-muted);
  }

  .tree-item-row.drag-over {
    border-color: var(--accent);
    background-color: var(--accent-muted);
  }

  .chevron-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
    transition: transform 0.15s ease, color 0.12s ease;
    border-radius: var(--radius-sm);
  }

  .chevron-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .chevron-btn svg {
    width: 11px;
    height: 11px;
  }

  .chevron-btn.open {
    transform: rotate(90deg);
  }

  .icon {
    display: flex;
    align-items: center;
    margin-right: 6px;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .icon svg {
    width: 14px;
    height: 14px;
  }

  .folder-icon {
    color: #ff9f0a; /* macOS orange folder style */
  }

  .note-icon {
    color: var(--accent); /* blue notes style */
  }

  .label {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .text-truncate {
    display: inline-block;
    max-width: 100%;
  }

  .tree-children {
    display: flex;
    flex-direction: column;
  }

  .rename-input {
    font-family: var(--font-sans);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    background-color: var(--bg-input, rgba(255, 255, 255, 0.05));
    border: 1.5px solid var(--accent);
    border-radius: var(--radius-sm, 4px);
    padding: 1px 4px;
    margin: -2px -5px;
    width: calc(100% - 10px);
    box-sizing: border-box;
    outline: none;
    box-shadow: 0 0 0 2px var(--accent-muted);
  }
</style>
