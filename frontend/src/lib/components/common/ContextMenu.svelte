<script lang="ts">
  import { onMount } from 'svelte';
  import { contextMenuStore, closeContextMenu } from '../../stores/contextMenu';
  import { openPrompt } from '../../stores/prompt';
  import { 
    createNewNote, 
    createFolder, 
    createFolderAndStartRename,
    deleteFolder, 
    moveFolder, 
    renameNoteById, 
    loadNotes, 
    activeNote, 
    activeNoteDirty,
    renamingFolder,
    renamingNote
  } from '../../stores/notes';
  import { sendNoteToTrash } from '../../stores/editor';
  import * as ipc from '../../services/ipc';

  const menu = $derived($contextMenuStore);

  async function triggerNewNote(folderPath: string) {
    closeContextMenu();
    try {
      await createNewNote(folderPath || null);
    } catch (e) {
      console.error('Failed to create note:', e);
    }
  }

  async function triggerNewFolder(parentFolder: string) {
    closeContextMenu();
    try {
      await createFolderAndStartRename(parentFolder || null);
    } catch (e) {
      console.error('Failed to create folder:', e);
    }
  }

  function triggerRenameFolder(folderPath: string) {
    closeContextMenu();
    renamingFolder.set(folderPath);
  }

  function triggerRenameNote(noteId: string, currentTitle: string) {
    closeContextMenu();
    renamingNote.set(noteId);
  }

  function triggerDeleteFolder(folderPath: string) {
    closeContextMenu();
    if (confirm(`Are you sure you want to delete folder "${folderPath}" and all its contents? This will trash all notes inside it.`)) {
      deleteFolder(folderPath).catch(err => {
        console.error('Failed to delete folder:', err);
      });
    }
  }

  async function triggerTrashNote(noteId: string) {
    closeContextMenu();
    try {
      await sendNoteToTrash(noteId);
      if ($activeNote && $activeNote.id === noteId) {
        activeNote.set(null);
        activeNoteDirty.set(false);
      }
      await loadNotes();
    } catch (e) {
      console.error('Failed to trash note:', e);
    }
  }

  function handleCopyId(noteId: string) {
    closeContextMenu();
    navigator.clipboard.writeText(noteId).catch(() => {});
  }

  onMount(() => {
    const handleOutsideClick = () => {
      if (menu.show) {
        closeContextMenu();
      }
    };
    window.addEventListener('click', handleOutsideClick);
    window.addEventListener('contextmenu', handleOutsideClick);
    return () => {
      window.removeEventListener('click', handleOutsideClick);
      window.removeEventListener('contextmenu', handleOutsideClick);
    };
  });
</script>

{#if menu.show}
  <!-- Stacking overlay to dismiss the menu -->
  <div 
    class="global-context-overlay" 
    onclick={closeContextMenu}
    oncontextmenu={(e) => { e.preventDefault(); closeContextMenu(); }}
    role="presentation"
  ></div>

  <div 
    class="global-context-menu" 
    style="top: {menu.y}px; left: {menu.x}px;"
    role="menu"
    aria-label="Context options"
  >
    {#if menu.type === 'folder'}
      <button class="ctx-item" role="menuitem" onclick={() => triggerNewNote(menu.relPath)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M2 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5z" />
          <line x1="8" y1="6" x2="8" y2="12" />
          <line x1="5" y1="9" x2="11" y2="9" />
        </svg>
        New Note Here
      </button>
      <button class="ctx-item" role="menuitem" onclick={() => triggerNewFolder(menu.relPath)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1.5 3.5a1 1 0 0 1 1-1h3.586a1 1 0 0 1 .707.293L8.5 4.5H13.5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-8z" />
          <line x1="8" y1="6" x2="8" y2="11" />
          <line x1="5.5" y1="8.5" x2="10.5" y2="8.5" />
        </svg>
        New Folder Here
      </button>
      <button class="ctx-item" role="menuitem" onclick={() => triggerRenameFolder(menu.relPath)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 2H9c-1.1 0-2 .9-2 2v8c0 1.1.9 2 2 2h2c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z" />
          <path d="M4.5 12h-2V4h2" />
        </svg>
        Rename Folder
      </button>
      <div class="ctx-sep" role="separator"></div>
      <button class="ctx-item ctx-danger" role="menuitem" onclick={() => triggerDeleteFolder(menu.relPath)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="2,4 14,4"/>
          <path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1"/>
          <path d="M3 4l1 9a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1l1-9"/>
        </svg>
        Delete Folder
      </button>
    {:else if menu.type === 'note'}
      <button class="ctx-item" role="menuitem" onclick={() => triggerRenameNote(menu.noteId, menu.noteTitle)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7l-3-3z" />
          <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
        Rename Note
      </button>
      <button class="ctx-item" role="menuitem" onclick={() => handleCopyId(menu.noteId)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <rect x="5" y="5" width="8" height="8" rx="1.5"/>
          <path d="M9 5V3a1 1 0 0 0-1-1H3a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h2"/>
        </svg>
        Copy Note ID
      </button>
      <div class="ctx-sep" role="separator"></div>
      <button class="ctx-item ctx-danger" role="menuitem" onclick={() => triggerTrashNote(menu.noteId)}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="2,4 14,4"/>
          <path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1"/>
          <path d="M3 4l1 9a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1l1-9"/>
        </svg>
        Move to Trash
      </button>
    {:else}
      <button class="ctx-item" role="menuitem" onclick={() => triggerNewNote('')}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M2 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5z" />
          <line x1="8" y1="6" x2="8" y2="12" />
          <line x1="5" y1="9" x2="11" y2="9" />
        </svg>
        New Note at Root
      </button>
      <button class="ctx-item" role="menuitem" onclick={() => triggerNewFolder('')}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1.5 3.5a1 1 0 0 1 1-1h3.586a1 1 0 0 1 .707.293L8.5 4.5H13.5a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-8z" />
          <line x1="8" y1="6" x2="8" y2="11" />
          <line x1="5.5" y1="8.5" x2="10.5" y2="8.5" />
        </svg>
        New Folder at Root
      </button>
    {/if}
  </div>
{/if}

<style>
  .global-context-overlay {
    position: fixed;
    inset: 0;
    z-index: 99998;
    background: transparent;
  }

  .global-context-menu {
    position: fixed;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    z-index: 99999;
    min-width: 180px;
    padding: 5px;
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    animation: scaleIn 0.12s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .ctx-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    transition: all 0.1s ease;
    user-select: none;
  }

  .ctx-item svg {
    width: 13px;
    height: 13px;
    flex-shrink: 0;
    display: block;
  }

  .ctx-item:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .ctx-danger { 
    color: var(--color-red); 
  }
  
  .ctx-danger:hover { 
    background-color: var(--color-red-muted); 
    color: var(--color-red); 
  }

  .ctx-sep {
    height: 1px;
    background-color: var(--border-subtle);
    margin: 4px 6px;
  }

  @keyframes scaleIn {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
