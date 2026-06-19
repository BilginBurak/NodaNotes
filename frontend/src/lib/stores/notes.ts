import { writable, get } from 'svelte/store';
import type { NoteListItemDto, NoteDto, TagWithCountDto } from '../types';
import * as ipc from '../services/ipc';
import { loadNoteSnapshots, showSnapshots } from './editor';

export const notesList      = writable<NoteListItemDto[]>([]);
export const activeNote     = writable<NoteDto | null>(null);
export const loadingNote    = writable<boolean>(false);
export const activeNoteDirty = writable<boolean>(false);
export const activeNoteSessionModified = writable<boolean>(false);
export const activeNoteLocked = writable<boolean>(false);
export const selectedFolder  = writable<string | null>(null);
export const selectedTag     = writable<string | null>(null);
export const tagsList        = writable<TagWithCountDto[]>([]);
export const focusEditorAtEnd = writable<boolean>(false);
export const notesError     = writable<string | null>(null);
/** Son başarılı kayıt zamanı — status bar için */
export const lastSavedAt    = writable<Date | null>(null);
/** Özel görünüm modu: normal, trash ya da conflicts */
export const activeViewMode = writable<'normal' | 'trash' | 'conflicts' | 'attachments' | 'daily'>('normal');
/** Silinen not olarak görüntüleniyor mu */
export const viewingTrashNote = writable<boolean>(false);
/** Çakışma notu olarak görüntüleniyor mu (archivedPath bilgisi ile) */
export const viewingConflictNote = writable<{ noteId: string; archivedPath: string } | null>(null);

export async function loadNotes() {
  notesError.set(null);
  try {
    const list = await ipc.listNotes();
    list.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
    notesList.set(list);
    await loadFolders();
    await loadTags();
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load notes');
  }
}

export async function loadTags() {
  try {
    const list = await ipc.listTagsWithCounts();
    tagsList.set(list);
  } catch (e) {
    console.error('Failed to load tags:', e);
  }
}

export async function triggerDailyNote() {
  notesError.set(null);
  try {
    const dailyNote = await ipc.triggerDailyNote();
    await loadNotes();
    activeNote.set(dailyNote);
    activeNoteDirty.set(false);
    activeNoteSessionModified.set(false);
    
    selectedFolder.set('Daily Notes');
    selectedTag.set(null);
    activeViewMode.set('normal');
    
    focusEditorAtEnd.set(true);
    
    return dailyNote;
  } catch (e: any) {
    notesError.set(e.message || 'Günlük not oluşturulamadı');
    throw e;
  }
}

export async function selectNote(id: string) {
  const currentActive = get(activeNote);
  const wasModified = get(activeNoteSessionModified);
  if (currentActive && wasModified) {
    await saveActiveNote(true, 'Blur');
  }

  loadingNote.set(true);
  notesError.set(null);
  activeNoteLocked.set(false);
  try {
    const note = await ipc.getNote(id);
    activeNote.set(note);
    activeNoteDirty.set(false);
    activeNoteSessionModified.set(false);
    viewingTrashNote.set(false);
    viewingConflictNote.set(null);
    
    // Explicitly load snapshots if the panel is open
    if (get(showSnapshots)) {
      await loadNoteSnapshots(id);
    }
  } catch (e: any) {
    if (e.code === 'VAULT_LOCKED') {
      activeNoteLocked.set(true);
      const noteListItem = get(notesList).find(n => n.id === id);
      if (noteListItem) {
        activeNote.set({
          id: noteListItem.id,
          parent_id: noteListItem.parent_id,
          title: noteListItem.title,
          body: '',
          color: noteListItem.color,
          pinned: noteListItem.pinned,
          tags: noteListItem.tags,
          inline_tags: noteListItem.inline_tags,
          created_at: '',
          updated_at: noteListItem.updated_at,
          file_path: noteListItem.file_path,
          is_encrypted: true
        });
      }
    } else {
      notesError.set(e.message || 'Failed to load note content');
      activeNote.set(null);
    }
  } finally {
    loadingNote.set(false);
  }
}

export async function selectTrashNote(id: string) {
  loadingNote.set(true);
  notesError.set(null);
  try {
    const note = await ipc.getTrashNote(id);
    activeNote.set(note);
    activeNoteDirty.set(false);
    activeNoteSessionModified.set(false);
    viewingTrashNote.set(true);
    viewingConflictNote.set(null);
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load trash note');
    activeNote.set(null);
  } finally {
    loadingNote.set(false);
  }
}

export async function selectConflictNote(noteId: string, archivedPath: string) {
  loadingNote.set(true);
  notesError.set(null);
  try {
    const note = await ipc.getConflictNote(archivedPath);
    activeNote.set(note);
    activeNoteDirty.set(false);
    activeNoteSessionModified.set(false);
    viewingTrashNote.set(false);
    viewingConflictNote.set({ noteId, archivedPath });
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load conflict note');
    activeNote.set(null);
  } finally {
    loadingNote.set(false);
  }
}

export async function saveActiveNote(triggerSnapshot: boolean = false, reason: string | null = null) {
  const currentActive = get(activeNote);
  const isDirty = get(activeNoteDirty);
  const wasModified = get(activeNoteSessionModified);

  if (!currentActive) return;
  if (!isDirty && !triggerSnapshot) return;
  if (triggerSnapshot && !wasModified) return;

  try {
    const updated = await ipc.updateNote(
      currentActive.id,
      currentActive.title,
      currentActive.body,
      currentActive.parent_id ?? null,
      currentActive.color ?? null,
      currentActive.pinned ?? false,
      currentActive.tags ?? [],
      triggerSnapshot,
      reason
    );
    activeNote.set(updated);
    activeNoteDirty.set(false);
    if (triggerSnapshot || reason === 'Manual' || reason === 'Blur') {
      activeNoteSessionModified.set(false);
    }
    lastSavedAt.set(new Date());
    await loadNotes();

    // Explicitly load snapshots after save if the panel is open
    if (get(showSnapshots)) {
      await loadNoteSnapshots(updated.id);
    }
  } catch (e: any) {
    notesError.set(e.message || 'Failed to save note');
    throw e;
  }
}

export async function createNewNote(targetDir: string | null = null) {
  notesError.set(null);
  try {
    const newNote = await ipc.createNote('Untitled', '', null, null, false, []);
    if (targetDir) {
      await ipc.moveNote(newNote.id, targetDir);
    }
    await loadNotes();
    const noteWithCorrectPath = await ipc.getNote(newNote.id);
    activeNote.set(noteWithCorrectPath);
    activeNoteDirty.set(false);
    activeNoteSessionModified.set(false);
    return noteWithCorrectPath;
  } catch (e: any) {
    notesError.set(e.message || 'Failed to create note');
    throw e;
  }
}

export async function renameActiveNote(newTitle: string) {
  const currentActive = get(activeNote);
  if (!currentActive) return;

  try {
    const updated = await ipc.renameNote(currentActive.id, newTitle);
    activeNote.set(updated);
    await loadNotes();
  } catch (e: any) {
    notesError.set(e.message || 'Failed to rename note');
    throw e;
  }
}

export async function renameNoteById(id: string, newTitle: string) {
  try {
    const updated = await ipc.renameNote(id, newTitle);
    // Aktif not ise güncelle
    const currentActive = get(activeNote);
    if (currentActive && currentActive.id === id) {
      activeNote.set(updated);
    }
    await loadNotes();
  } catch (e: any) {
    notesError.set(e.message || 'Failed to rename note');
    throw e;
  }
}

export async function deleteActiveNote() {
  const currentActive = get(activeNote);
  if (!currentActive) return;

  try {
    await ipc.deleteNote(currentActive.id);
    activeNote.set(null);
    activeNoteDirty.set(false);
    activeNoteSessionModified.set(false);
    await loadNotes();
  } catch (e: any) {
    notesError.set(e.message || 'Failed to delete note');
    throw e;
  }
}

export function updateActiveNoteBody(body: string) {
  activeNote.update((note) => {
    if (!note) return null;
    return { ...note, body };
  });
  activeNoteDirty.set(true);
  activeNoteSessionModified.set(true);
}

export function updateActiveNoteFrontmatter(frontmatter: Record<string, any>) {
  activeNote.update((note) => {
    if (!note) return null;
    const oldFm = note.frontmatter || {
      id: note.id,
      title: note.title,
      created: note.created_at,
      updated: note.updated_at,
      tags: note.tags
    };
    return {
      ...note,
      frontmatter: {
        ...oldFm,
        ...frontmatter
      }
    };
  });
  activeNoteDirty.set(true);
  activeNoteSessionModified.set(true);
}

export function updateActiveNoteTags(tags: string[]) {
  activeNote.update((note) => {
    if (!note) return null;
    return { ...note, tags };
  });
  activeNoteDirty.set(true);
  activeNoteSessionModified.set(true);
}

// ── Folder Stores & Functions ───────────────────────────────
export const draggedItem = writable<{ type: 'note' | 'folder'; id: string; relPath: string } | null>(null);
export const renamingFolder = writable<string | null>(null);
export const renamingNote = writable<string | null>(null);
export const foldersList = writable<string[]>([]);

export async function loadFolders() {
  try {
    const folders = await ipc.listFolders();
    // Sort directories alphabetically
    folders.sort((a, b) => a.localeCompare(b));
    foldersList.set(folders);
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load folders');
  }
}

export async function createFolder(relPath: string) {
  notesError.set(null);
  try {
    await ipc.createFolder(relPath);
    await loadFolders();
  } catch (e: any) {
    notesError.set(e.message || 'Failed to create folder');
    throw e;
  }
}

export async function createFolderAndStartRename(parentPath: string | null = null) {
  notesError.set(null);
  try {
    const list = get(foldersList);
    const normalizedParent = parentPath ? parentPath.trim().replace(/\/+$/, '') : '';
    const parentPrefix = normalizedParent ? `${normalizedParent}/` : '';
    
    let baseName = 'New Folder';
    let candidate = parentPrefix + baseName;
    
    if (list.includes(candidate)) {
      let counter = 2;
      while (list.includes(`${parentPrefix}${baseName} ${counter}`)) {
        counter++;
      }
      candidate = `${parentPrefix}${baseName} ${counter}`;
    }
    
    await ipc.createFolder(candidate);
    await loadFolders();
    
    // Set the renaming folder so the tree row instantly transitions into renaming state
    renamingFolder.set(candidate);
  } catch (e: any) {
    notesError.set(e.message || 'Failed to create folder');
    throw e;
  }
}

export async function renameFolder(srcDir: string, newName: string) {
  notesError.set(null);
  try {
    await ipc.renameFolder(srcDir, newName);
    await loadFolders();
    await loadNotes();

    // If active note was in renamed folder, fetch updated path
    const active = get(activeNote);
    if (active && (active.file_path.startsWith(srcDir + '/') || active.file_path === srcDir)) {
      const updated = await ipc.getNote(active.id);
      activeNote.set(updated);
    }
  } catch (e: any) {
    notesError.set(e.message || 'Failed to rename folder');
    throw e;
  }
}

export async function deleteFolder(relPath: string) {
  notesError.set(null);
  try {
    await ipc.deleteFolder(relPath);
    await loadFolders();
    await loadNotes();
    
    // If active note was in deleted folder, close active note
    const active = get(activeNote);
    if (active && (active.file_path.startsWith(relPath + '/') || active.file_path === relPath)) {
      activeNote.set(null);
      activeNoteDirty.set(false);
      activeNoteSessionModified.set(false);
    }
  } catch (e: any) {
    notesError.set(e.message || 'Failed to delete folder');
    throw e;
  }
}

export async function moveNote(id: string, targetDir: string) {
  notesError.set(null);
  try {
    await ipc.moveNote(id, targetDir);
    await loadNotes();
    
    // If active note is the moved note, fetch updated path
    const active = get(activeNote);
    if (active && active.id === id) {
      const updated = await ipc.getNote(id);
      activeNote.set(updated);
    }
  } catch (e: any) {
    notesError.set(e.message || 'Failed to move note');
    throw e;
  }
}

export async function moveFolder(srcDir: string, targetDir: string) {
  notesError.set(null);
  try {
    await ipc.moveFolder(srcDir, targetDir);
    await loadFolders();
    await loadNotes();
    
    // If active note was inside the moved folder hierarchy, fetch updated path
    const active = get(activeNote);
    if (active && (active.file_path.startsWith(srcDir + '/') || active.file_path === srcDir)) {
      const updated = await ipc.getNote(active.id);
      activeNote.set(updated);
    }
  } catch (e: any) {
    notesError.set(e.message || 'Failed to move folder');
    throw e;
  }
}
