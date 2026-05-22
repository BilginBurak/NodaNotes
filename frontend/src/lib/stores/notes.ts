import { writable, get } from 'svelte/store';
import type { NoteListItemDto, NoteDto } from '../types';
import * as ipc from '../services/ipc';
import { loadNoteSnapshots, showSnapshots } from './editor';

export const notesList      = writable<NoteListItemDto[]>([]);
export const activeNote     = writable<NoteDto | null>(null);
export const loadingNote    = writable<boolean>(false);
export const activeNoteDirty = writable<boolean>(false);
export const notesError     = writable<string | null>(null);
/** Son başarılı kayıt zamanı — status bar için */
export const lastSavedAt    = writable<Date | null>(null);

export async function loadNotes() {
  notesError.set(null);
  try {
    const list = await ipc.listNotes();
    list.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
    notesList.set(list);
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load notes');
  }
}

export async function selectNote(id: string) {
  const currentActive = get(activeNote);
  const isDirty = get(activeNoteDirty);
  if (currentActive && isDirty) {
    await saveActiveNote();
  }

  loadingNote.set(true);
  notesError.set(null);
  try {
    const note = await ipc.getNote(id);
    activeNote.set(note);
    activeNoteDirty.set(false);
    
    // Explicitly load snapshots if the panel is open
    if (get(showSnapshots)) {
      await loadNoteSnapshots(id);
    }
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load note content');
    activeNote.set(null);
  } finally {
    loadingNote.set(false);
  }
}

export async function saveActiveNote() {
  const currentActive = get(activeNote);
  const isDirty = get(activeNoteDirty);
  if (!currentActive || !isDirty) return;

  try {
    const updated = await ipc.updateNote(
      currentActive.id,
      currentActive.title,
      currentActive.body,
      currentActive.parent_id ?? null,
      currentActive.color ?? null,
      currentActive.pinned ?? false,
      currentActive.tags ?? [],
    );
    activeNote.set(updated);
    activeNoteDirty.set(false);
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

export async function createNewNote() {
  notesError.set(null);
  try {
    // Tauri backend artık title ve diğer alanları bekliyor
    const newNote = await ipc.createNote('Untitled', '', null, null, false, []);
    await loadNotes();
    activeNote.set(newNote);
    activeNoteDirty.set(false);
    return newNote;
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
}

export function updateActiveNoteTags(tags: string[]) {
  activeNote.update((note) => {
    if (!note) return null;
    return { ...note, tags };
  });
  activeNoteDirty.set(true);
}
