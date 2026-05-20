import { writable, get } from 'svelte/store';
import type { NoteListItemDto, NoteDto } from '../types';
import * as ipc from '../services/ipc';

export const notesList = writable<NoteListItemDto[]>([]);
export const activeNote = writable<NoteDto | null>(null);
export const loadingNote = writable<boolean>(false);
export const activeNoteDirty = writable<boolean>(false);
export const notesError = writable<string | null>(null);

export async function loadNotes() {
  notesError.set(null);
  try {
    const list = await ipc.listNotes();
    // Sort notes: most recently updated first
    list.sort((a, b) => new Date(b.updated).getTime() - new Date(a.updated).getTime());
    notesList.set(list);
  } catch (e: any) {
    notesError.set(e.message || 'Failed to load notes');
  }
}

export async function selectNote(id: string) {
  // If active note is dirty, auto-save first
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
      currentActive.frontmatter
    );
    activeNote.set(updated);
    activeNoteDirty.set(false);
    await loadNotes();
  } catch (e: any) {
    notesError.set(e.message || 'Failed to save note');
    throw e;
  }
}

export async function createNewNote() {
  notesError.set(null);
  try {
    const newNote = await ipc.createNote();
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
    return {
      ...note,
      frontmatter: {
        ...note.frontmatter,
        ...frontmatter
      }
    };
  });
  activeNoteDirty.set(true);
}
