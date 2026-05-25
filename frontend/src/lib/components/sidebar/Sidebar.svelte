<script lang="ts">
  import { onMount } from 'svelte';
  import { vaultInfo } from '../../stores/vault';
  import { trashList, loadTrash, recoverFromTrash, emptyTrashPermanently, attachmentsWithMetadataList, loadAttachmentsWithMetadata } from '../../stores/editor';
  import { syncConflicts, loadConflicts } from '../../stores/sync';
  import { 
    notesList, 
    foldersList, 
    selectedFolder, 
    activeNote,
    moveNote, 
    moveFolder,
    loadNotes,
    draggedItem,
    createFolderAndStartRename,
    renamingFolder,
    activeViewMode
  } from '../../stores/notes';
  import FolderTreeItem from './FolderTreeItem.svelte';
  import type { TreeNode, NoteListItemDto } from '../../types';
  import * as ipc from '../../services/ipc';
  import { openContextMenu } from '../../stores/contextMenu';
  import { get } from 'svelte/store';

  let { onOpenSettings } = $props<{
    onOpenSettings: (tab: 'appearance' | 'editor' | 'sync' | 'history' | 'vault') => void;
  }>();

  const info = $derived($vaultInfo);
  const trash = $derived($trashList);
  const folders = $derived($foldersList);
  const notes = $derived($notesList);
  const activeFld = $derived($selectedFolder);
  const conflicts = $derived($syncConflicts);
  const activeNt = $derived($activeNote);
  const attachmentsWithMetadata = $derived($attachmentsWithMetadataList);

  let expandedFolders = $state<Record<string, boolean>>({});

  function selectTrash() {
    selectedFolder.set('__trash__');
    activeViewMode.set('trash');
    loadTrash();
  }

  function selectConflicts() {
    selectedFolder.set('__conflicts__');
    activeViewMode.set('conflicts');
    loadConflicts();
  }

  function selectAttachments() {
    selectedFolder.set('__attachments__');
    activeViewMode.set('attachments' as any);
    loadAttachmentsWithMetadata();
  }

  function selectAllNotes() {
    selectedFolder.set(null);
    activeViewMode.set('normal');
  }

  // Reactive Tree Generation
  const tree = $derived(buildTree(folders, notes));

  function buildTree(foldersList: string[], notesList: NoteListItemDto[]): TreeNode {
    const root: TreeNode = {
      name: 'Vault Root',
      type: 'folder',
      relPath: '',
      children: []
    };

    // 1. Add all folders
    for (const path of foldersList) {
      if (!path.trim()) continue;
      const parts = path.split('/');
      let current = root;
      let accumulatedPath = '';
      
      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        accumulatedPath = accumulatedPath ? `${accumulatedPath}/${part}` : part;
        
        let child = current.children.find(c => c.type === 'folder' && c.name === part);
        if (!child) {
          child = {
            name: part,
            type: 'folder',
            relPath: accumulatedPath,
            children: []
          };
          current.children.push(child);
        }
        current = child;
      }
    }

    // 2. Add all active notes to their corresponding folders
    for (const note of notesList) {
      const filePath = note.file_path;
      const lastSlash = filePath.lastIndexOf('/');
      const dirPath = lastSlash !== -1 ? filePath.substring(0, lastSlash) : '';
      
      let current = root;
      if (dirPath) {
        const parts = dirPath.split('/');
        let accumulatedPath = '';
        for (const part of parts) {
          accumulatedPath = accumulatedPath ? `${accumulatedPath}/${part}` : part;
          let child = current.children.find(c => c.type === 'folder' && c.name === part);
          if (!child) {
            child = {
              name: part,
              type: 'folder',
              relPath: accumulatedPath,
              children: []
            };
            current.children.push(child);
          }
          current = child;
        }
      }
      
      current.children.push({
        name: note.title || 'Untitled',
        type: 'note',
        relPath: filePath,
        id: note.id,
        children: []
      });
    }

    function sortTree(node: TreeNode) {
      node.children.sort((a, b) => {
        if (a.type !== b.type) {
          return a.type === 'folder' ? -1 : 1;
        }
        return a.name.localeCompare(b.name);
      });
      for (const child of node.children) {
        sortTree(child);
      }
    }
    
    sortTree(root);
    return root;
  }

  // Drag & Drop Root State
  let rootDragOver = $state(false);

  function closeVault() {
    vaultInfo.set(null);
  }

  async function handleRestoreTrash(id: string) {
    try {
      await recoverFromTrash(id);
    } catch (e) {
      console.error('Failed to restore note:', e);
    }
  }

  async function handlePermanentDelete(id: string) {
    if (confirm('Permanently delete this note? This cannot be undone.')) {
      try {
        await emptyTrashPermanently(id);
      } catch (e) {
        console.error('Failed to delete permanently:', e);
      }
    }
  }

  // ── Drag & Drop Root ──────────────────────────────────────
  function handleRootDragOver(e: DragEvent) {
    e.preventDefault();
    rootDragOver = true;
  }

  function handleRootDragLeave() {
    rootDragOver = false;
  }

  async function handleRootDrop(e: DragEvent) {
    e.preventDefault();
    rootDragOver = false;
    
    const item = get(draggedItem);
    if (!item) return;

    try {
      if (item.type === 'note') {
        await moveNote(item.id, '');
      } else if (item.type === 'folder') {
        const folderName = item.relPath.split('/').pop() || '';
        await moveFolder(item.relPath, folderName);
      }
      draggedItem.set(null);
    } catch (err) {
      console.error('Drop to root failed:', err);
    }
  }

  // ── Context Menu Helpers ─────────────────────────────────
  function handleContextMenu(e: MouseEvent, type: 'folder' | 'note' | 'root', relPath: string, noteId = '') {
    openContextMenu(e, type, relPath, noteId);
  }

  $effect(() => {
    const renaming = $renamingFolder;
    if (renaming) {
      const parts = renaming.split('/');
      let accumulated = '';
      for (let i = 0; i < parts.length - 1; i++) {
        accumulated = accumulated ? `${accumulated}/${parts[i]}` : parts[i];
        expandedFolders[accumulated] = true;
      }
    }
  });

  $effect(() => {
    const active = activeNt;
    if (active && active.file_path) {
      const parts = active.file_path.split('/');
      let accumulated = '';
      for (let i = 0; i < parts.length - 1; i++) {
        accumulated = accumulated ? `${accumulated}/${parts[i]}` : parts[i];
        expandedFolders[accumulated] = true;
      }
    }
  });

  onMount(() => {
    loadTrash();
    loadConflicts();
    loadAttachmentsWithMetadata();

    const handleGlobalDragEnd = () => {
      rootDragOver = false;
    };

    window.addEventListener('dragend', handleGlobalDragEnd);
    window.addEventListener('drop', handleGlobalDragEnd, true);

    return () => {
      window.removeEventListener('dragend', handleGlobalDragEnd);
      window.removeEventListener('drop', handleGlobalDragEnd, true);
    };
  });
</script>

<aside class="sidebar" style="position: relative; z-index: 100; overflow: visible;" oncontextmenu={(e) => handleContextMenu(e, 'root', '')}>
  <!-- Content Area -->
  <div class="sidebar-body scrollbar-thin">
    <!-- Workspace Section -->
    <div 
      class="nav-section"
      class:drag-over={rootDragOver}
      ondragover={handleRootDragOver}
      ondragleave={handleRootDragLeave}
      ondrop={handleRootDrop}
    >
      <span 
        class="section-label"
        oncontextmenu={(e) => handleContextMenu(e, 'root', '')}
      >
        Workspace
      </span>
      <div class="nav-items">
        <button 
          class="nav-item nav-btn" 
          class:active-item={activeFld === null}
          onclick={selectAllNotes}
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M2 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5z"/>
            <line x1="5" y1="7" x2="11" y2="7"/>
            <line x1="5" y1="10" x2="9" y2="10"/>
          </svg>
          <span>All Notes</span>
        </button>
      </div>
    </div>

    <!-- Folders Section -->
    <div 
      class="nav-section folders-section"
      class:drag-over={rootDragOver}
      ondragover={handleRootDragOver}
      ondragleave={handleRootDragLeave}
      ondrop={handleRootDrop}
    >
      <div class="section-header-row">
        <span class="section-label" oncontextmenu={(e) => handleContextMenu(e, 'root', '')}>{info?.name || 'Folders'}</span>
        <button 
          class="add-folder-btn" 
          onclick={() => createFolderAndStartRename(activeFld)} 
          title="Create subfolder in active folder"
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="8" y1="3" x2="8" y2="13" />
            <line x1="3" y1="8" x2="13" y2="8" />
          </svg>
        </button>
      </div>
      
      <div class="folder-tree scrollbar-thin" role="tree">
        {#if tree.children.length === 0}
          <div class="empty-tree-state" oncontextmenu={(e) => handleContextMenu(e, 'root', '')}>
            Right-click or click + to create a folder.
          </div>
        {:else}
          {#each tree.children as child (child.type + '-' + child.relPath)}
            <FolderTreeItem 
              node={child} 
              depth={0} 
              {expandedFolders} 
            />
          {/each}
        {/if}
      </div>
    </div>

    <!-- Management & Preferences Section -->
    <div class="nav-section">
      <span class="section-label">Management & Settings</span>
      <div class="nav-items">
        <!-- Settings Toggle -->
        <button class="nav-item nav-btn" onclick={() => onOpenSettings('appearance')}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="8" cy="8" r="2.5"/>
            <path d="M8 2v1.5M8 12.5V14M2 8H3.5M12.5 8H14M4.05 4.05l1.06 1.06M10.88 10.88l1.06 1.06M4.05 11.95l1.06-1.06M10.88 5.12l1.06-1.06"/>
          </svg>
          <span>Settings</span>
        </button>

        <!-- Attachments Link -->
        <button 
          class="nav-item nav-btn"
          class:active-item={activeFld === '__attachments__'}
          onclick={selectAttachments}
          aria-label="View attachments"
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M2 13V3a1 1 0 0 1 1-1h6l4 4v7a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>
            <polyline points="8 2 8 6 12 6"/>
          </svg>
          <span class="flex-grow">Attachments</span>
          {#if attachmentsWithMetadata.length > 0}
            <span class="badge badge-blue">{attachmentsWithMetadata.length}</span>
          {/if}
        </button>

        <!-- Deleted Notes — navigates to __trash__ folder mode -->
        <button 
          class="nav-item nav-btn"
          class:active-item={activeFld === '__trash__'}
          onclick={selectTrash}
          aria-label="View deleted notes"
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="2,4 14,4"/>
            <path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1M6 7v5M10 7v5"/>
            <path d="M3 4l1 9a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1l1-9"/>
          </svg>
          <span class="flex-grow">Deleted Notes</span>
          {#if trash.length > 0}
            <span class="badge badge-red">{trash.length}</span>
          {/if}
        </button>

        <!-- Sync Conflicts — navigates to __conflicts__ folder mode -->
        <button 
          class="nav-item nav-btn"
          class:active-item={activeFld === '__conflicts__'}
          onclick={selectConflicts}
          aria-label="View sync conflicts"
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M8 1.5L14 12.5H2L8 1.5z"/>
            <line x1="8" y1="6" x2="8" y2="9"/>
            <circle cx="8" cy="11" r="0.5" fill="currentColor"/>
          </svg>
          <span class="flex-grow">Sync Conflicts</span>
          {#if conflicts.length > 0}
            <span class="badge badge-orange">{conflicts.length}</span>
          {/if}
        </button>

      </div>
    </div>

  </div>

  <!-- Footer -->
  <div class="sidebar-footer">
    {#if info}
      <div class="vault-row">
        <div class="vault-meta">
          <span class="vault-label">Vault</span>
          <span class="vault-path" title={info.path}>{info.path}</span>
        </div>
        <button class="eject-btn" onclick={closeVault} title="Close vault">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="4,10 8,6 12,10"/>
            <line x1="3" y1="13" x2="13" y2="13"/>
          </svg>
        </button>
      </div>
    {/if}
  </div>

</aside>

<style>
  /* ── Sidebar Container ── */
  .sidebar {
    width: 220px;
    height: 100%;
    background-color: var(--sidebar-vibrancy);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    position: relative;
    z-index: 100;
    overflow: visible;
  }

  /* ── Body ── */
  .sidebar-body {
    flex: 1;
    overflow-y: auto;
    padding: 10px 8px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  /* Navigation Sections */
  .nav-section {
    display: flex;
    flex-direction: column;
    gap: 2px;
    transition: background-color 0.15s ease;
  }

  .section-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 0 8px;
    margin-bottom: 2px;
    cursor: default;
    user-select: none;
  }

  .nav-items {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  /* macOS Finder style items */
  .nav-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    user-select: none;
    transition: all 0.12s ease;
  }

  .nav-item svg {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
  }

  .nav-item.active-item {
    color: var(--accent);
    background-color: var(--accent-muted);
  }

  .nav-btn {
    background: transparent;
    border: none;
    width: 100%;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
  }

  .nav-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }



  .flex-grow {
    flex: 1;
  }

  .badge {
    font-size: 10px;
    font-weight: 600;
    min-width: 16px;
    height: 16px;
    border-radius: var(--radius-pill);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 4px;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .badge-red {
    background-color: var(--color-red-muted);
    color: var(--color-red);
  }

  .badge-blue {
    background-color: var(--accent-muted);
    color: var(--accent);
  }

  .badge-orange {
    background-color: var(--color-orange-muted);
    color: var(--color-orange);
  }



  /* ── Folders Section ── */
  .folders-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 160px;
  }

  .section-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 8px;
  }

  .add-folder-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-tertiary);
    padding: 3px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
  }

  .add-folder-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .add-folder-btn svg {
    width: 12px;
    height: 12px;
  }

  .folder-tree {
    flex: 1;
    overflow-y: auto;
    margin: 4px -8px 0 -8px;
  }

  .empty-tree-state {
    font-size: 11px;
    color: var(--text-disabled);
    padding: 12px 8px;
    margin: 8px;
    text-align: center;
    line-height: 1.4;
    border: 1px dashed var(--border-subtle);
    border-radius: var(--radius-md);
    cursor: default;
    user-select: none;
  }

  .nav-section.drag-over {
    background-color: var(--accent-muted);
    border-radius: var(--radius-lg);
  }

  /* ── Footer ── */
  .sidebar-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .vault-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .vault-meta {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
    gap: 1px;
  }

  .vault-label {
    font-size: 10px;
    color: var(--text-disabled);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .vault-path {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  .eject-btn {
    background: transparent;
    border: none;
    color: var(--text-disabled);
    cursor: pointer;
    padding: 5px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
    flex-shrink: 0;
  }

  .eject-btn svg {
    width: 14px;
    height: 14px;
    display: block;
  }

  .eject-btn:hover {
    color: var(--color-red);
    background-color: var(--color-red-muted);
  }


</style>
