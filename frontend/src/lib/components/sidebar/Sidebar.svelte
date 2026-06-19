<script lang="ts">
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
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
    activeViewMode,
    selectedTag,
    tagsList
  } from '../../stores/notes';
  import FolderTreeItem from './FolderTreeItem.svelte';
  import type { TreeNode, NoteListItemDto } from '../../types';
  import * as ipc from '../../services/ipc';
  import { openContextMenu } from '../../stores/contextMenu';
  import { get } from 'svelte/store';

  let { onOpenSettings } = $props<{
    onOpenSettings: (tab: 'appearance' | 'editor' | 'sync' | 'history' | 'vault' | 'maintenance' | 'templates') => void;
  }>();

  const info = $derived($vaultInfo);
  const trash = $derived($trashList);
  const folders = $derived($foldersList);
  const notes = $derived($notesList);
  const activeFld = $derived($selectedFolder);
  const conflicts = $derived($syncConflicts);
  const activeNt = $derived($activeNote);
  const attachmentsWithMetadata = $derived($attachmentsWithMetadataList);
  const activeViewModeVal = $derived($activeViewMode);
  
  // Tags and selected tag
  const tags = $derived($tagsList);
  const currentTag = $derived($selectedTag);

  // Accordion Toggles
  let workspaceOpen = $state(true);
  let foldersOpen = $state(true);
  let tagsOpen = $state(true);
  let managementOpen = $state(true);

  // Splitter Resizable Height (folders tree height)
  let foldersHeight = $state(220);
  let isResizing = $state(false);
  let middleHeight = $state(0);
  let expandedFolders = $state<Record<string, boolean>>({});

  onMount(() => {
    // Load accordion memory
    workspaceOpen = localStorage.getItem('noda_sidebar_workspace_open') !== 'false';
    foldersOpen = localStorage.getItem('noda_sidebar_folders_open') !== 'false';
    tagsOpen = localStorage.getItem('noda_sidebar_tags_open') !== 'false';
    managementOpen = localStorage.getItem('noda_sidebar_management_open') !== 'false';

    // Load splitter height memory
    const savedHeight = localStorage.getItem('noda_sidebar_folders_height');
    if (savedHeight) {
      foldersHeight = parseInt(savedHeight, 10);
    }
  });

  function toggleWorkspace() {
    workspaceOpen = !workspaceOpen;
    localStorage.setItem('noda_sidebar_workspace_open', String(workspaceOpen));
  }
  function toggleFolders() {
    foldersOpen = !foldersOpen;
    localStorage.setItem('noda_sidebar_folders_open', String(foldersOpen));
  }
  function toggleTags() {
    tagsOpen = !tagsOpen;
    localStorage.setItem('noda_sidebar_tags_open', String(tagsOpen));
  }
  function toggleManagement() {
    managementOpen = !managementOpen;
    localStorage.setItem('noda_sidebar_management_open', String(managementOpen));
  }

  function startResize(e: MouseEvent) {
    e.preventDefault();
    isResizing = true;
    const startY = e.clientY;
    const startHeight = foldersHeight;
    const middleEl = document.querySelector('.sidebar-middle');
    const middleHeight = middleEl ? middleEl.clientHeight : 500;

    function onMouseMove(moveEvent: MouseEvent) {
      const deltaY = moveEvent.clientY - startY;
      // Clamp both folders and tags to at least 80px (with 6px splitter height accounted)
      foldersHeight = Math.max(80, Math.min(middleHeight - 80 - 6, startHeight + deltaY));
    }

    function onMouseUp() {
      isResizing = false;
      localStorage.setItem('noda_sidebar_folders_height', String(foldersHeight));
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }

  function selectTrash() {
    selectedFolder.set('__trash__');
    selectedTag.set(null);
    activeViewMode.set('trash');
    loadTrash();
  }

  function selectConflicts() {
    selectedFolder.set('__conflicts__');
    selectedTag.set(null);
    activeViewMode.set('conflicts');
    loadConflicts();
  }

  function selectAttachments() {
    selectedFolder.set('__attachments__');
    selectedTag.set(null);
    activeViewMode.set('attachments' as any);
    loadAttachmentsWithMetadata();
  }

  function selectAllNotes() {
    selectedFolder.set(null);
    selectedTag.set(null);
    activeViewMode.set('normal');
  }

  function selectDailyNotes() {
    selectedFolder.set('Daily Notes');
    selectedTag.set(null);
    activeViewMode.set('daily');
  }

  function selectEncryptedNotes() {
    selectedFolder.set('__encrypted__');
    selectedTag.set(null);
    activeViewMode.set('encrypted' as any);
  }

  function selectTag(tagName: string) {
    selectedTag.set(tagName);
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
        if (a.name === '.templates') return 1;
        if (b.name === '.templates') return -1;
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

  // Clamp foldersHeight when settings panel expands or window shrinks
  $effect(() => {
    if (foldersOpen && tagsOpen && middleHeight > 0) {
      const maxFoldersHeight = Math.max(80, middleHeight - 80 - 6); // 80px min-height for tags, 6px for splitter
      if (foldersHeight > maxFoldersHeight) {
        foldersHeight = maxFoldersHeight;
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

<aside class="sidebar" oncontextmenu={(e) => handleContextMenu(e, 'root', '')}>
  <!-- Top fixed layout: WORKSPACE -->
  <div class="sidebar-top">
    <div 
      class="nav-section"
      class:drag-over={rootDragOver}
      ondragover={handleRootDragOver}
      ondragleave={handleRootDragLeave}
      ondrop={handleRootDrop}
    >
      <button class="section-label-btn" onclick={toggleWorkspace} aria-expanded={workspaceOpen}>
        <svg class="chevron-icon" class:open={workspaceOpen} viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 12 10 8 6 4" />
        </svg>
        <span>Workspace</span>
      </button>

      {#if workspaceOpen}
        <div class="nav-items" transition:slide={{ duration: 150 }}>
          <button 
            class="nav-item nav-btn" 
            class:active-item={activeFld === null && activeViewModeVal !== 'daily' && !currentTag}
            onclick={selectAllNotes}
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M2 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5z"/>
              <line x1="5" y1="7" x2="11" y2="7"/>
              <line x1="5" y1="10" x2="9" y2="10"/>
            </svg>
            <span>All Notes</span>
          </button>

          <button 
            class="nav-item nav-btn" 
            class:active-item={activeViewModeVal === 'daily'}
            onclick={selectDailyNotes}
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="2" y="2" width="12" height="12" rx="2"/>
              <polyline points="8,4 8,8 10,10"/>
            </svg>
            <span>Daily Notes</span>
          </button>

          <button 
            class="nav-item nav-btn" 
            class:active-item={activeViewModeVal === 'encrypted'}
            onclick={selectEncryptedNotes}
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="3" y="6" width="10" height="8" rx="1.5" ry="1.5"/>
              <path d="M5 6V4.5a3 3 0 0 1 6 0V6"/>
            </svg>
            <span>Encrypted Notes</span>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <!-- Middle Resizable layout: FOLDERS & TAGS -->
  <div class="sidebar-middle" bind:clientHeight={middleHeight}>
    <!-- Folders area -->
    <div 
      class="nav-section folders-section"
      class:drag-over={rootDragOver}
      ondragover={handleRootDragOver}
      ondragleave={handleRootDragLeave}
      ondrop={handleRootDrop}
      style={!foldersOpen ? 'flex: none; height: auto; min-height: 0;' : (!tagsOpen ? 'flex: 1; height: auto; min-height: 80px;' : `height: ${foldersHeight}px; flex-shrink: 0; min-height: 80px;`)}
    >
      <div class="section-header-row">
        <button class="section-label-btn" onclick={toggleFolders} aria-expanded={foldersOpen}>
          <svg class="chevron-icon" class:open={foldersOpen} viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 12 10 8 6 4" />
          </svg>
          <span oncontextmenu={(e) => handleContextMenu(e, 'root', '')}>{info?.name || 'Folders'}</span>
        </button>
        {#if foldersOpen}
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
        {/if}
      </div>
      
      {#if foldersOpen}
        <div class="folder-tree scrollbar-thin" role="tree" transition:slide={{ duration: 150 }}>
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
      {/if}
    </div>

    <!-- Resizable Splitter (yatay splitter) -->
    {#if foldersOpen && tagsOpen}
      <div class="sidebar-splitter" class:resizing={isResizing} onmousedown={startResize} role="separator" aria-label="Resize folders and tags">
        <div class="splitter-line"></div>
      </div>
    {/if}

    <!-- Tags area -->
    <div 
      class="nav-section tags-section"
      style={!tagsOpen ? 'flex: none; height: auto; min-height: 0;' : (!foldersOpen ? 'flex: 1; height: auto; min-height: 80px;' : 'flex: 1; height: auto; min-height: 80px;')}
    >
      <button class="section-label-btn" onclick={toggleTags} aria-expanded={tagsOpen}>
        <svg class="chevron-icon" class:open={tagsOpen} viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 12 10 8 6 4" />
        </svg>
        <span>Tags</span>
      </button>

      {#if tagsOpen}
        <div class="tags-list scrollbar-thin" transition:slide={{ duration: 150 }}>
          {#if tags.length === 0}
            <div class="empty-tags-state">
              No tags found in notes.
            </div>
          {:else}
            {#each tags as tag (tag.name)}
              <button 
                class="nav-item nav-btn tag-item" 
                class:active-item={currentTag === tag.name}
                onclick={() => selectTag(tag.name)}
              >
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M7.293 1.5a1 1 0 0 1 .707.293l6 6a1 1 0 0 1 0 1.414l-4 4a1 1 0 0 1-1.414 0l-6-6A1 1 0 0 1 2.293 6.5V2.5a1 1 0 0 1 1-1h4z"/>
                  <circle cx="5.5" cy="5.5" r="1"/>
                </svg>
                <span class="flex-grow">#{tag.name}</span>
                <span class="badge badge-blue">{tag.count}</span>
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Bottom fixed layout: MANAGEMENT & PREFERENCES -->
  <div class="sidebar-bottom">
    <div class="nav-section">
      <button class="section-label-btn" onclick={toggleManagement} aria-expanded={managementOpen}>
        <svg class="chevron-icon" class:open={managementOpen} viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="6 12 10 8 6 4" />
        </svg>
        <span>Management & Settings</span>
      </button>
      
      {#if managementOpen}
        <div class="nav-items" transition:slide={{ duration: 150 }}>
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

          <!-- Deleted Notes -->
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

          <!-- Sync Conflicts -->
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
      {/if}
    </div>

    <!-- Vault Footer -->
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
    overflow: hidden;
  }

  .sidebar-top {
    padding: 10px 8px 4px;
    flex-shrink: 0;
  }

  .sidebar-middle {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 0 8px;
  }

  .sidebar-bottom {
    flex-shrink: 0;
    padding: 4px 8px 10px;
    border-top: 1px solid var(--border-subtle);
  }

  /* Splitter */
  .sidebar-splitter {
    height: 6px;
    margin: 4px -8px;
    cursor: ns-resize;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    flex-shrink: 0;
  }

  .splitter-line {
    width: 100%;
    height: 1px;
    background-color: var(--border-subtle);
    transition: background-color 0.15s ease, height 0.15s ease;
  }

  .sidebar-splitter:hover .splitter-line,
  .sidebar-splitter.resizing .splitter-line {
    background-color: var(--accent);
    height: 2px;
  }

  /* Folders section */
  .folders-section {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 80px;
  }

  .folder-tree {
    flex: 1;
    overflow-y: auto;
    margin: 4px 0 0 0;
  }

  /* Tags section */
  .tags-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 80px;
  }

  .tags-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-top: 4px;
  }

  .empty-tags-state {
    font-size: 11px;
    color: var(--text-disabled);
    padding: 12px 8px;
    text-align: center;
  }

  /* Section Header / Accordion Labels */
  .section-label-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    font-weight: 700;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 4px 8px;
    width: 100%;
    text-align: left;
    font-family: var(--font-sans);
    border-radius: var(--radius-sm);
    transition: color 0.12s ease, background-color 0.12s ease;
  }

  .section-label-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .chevron-icon {
    width: 10px;
    height: 10px;
    color: var(--text-disabled);
    transition: transform 0.15s ease;
  }

  .chevron-icon.open {
    transform: rotate(90deg);
  }

  .nav-section {
    display: flex;
    flex-direction: column;
    gap: 2px;
    transition: background-color 0.15s ease;
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
