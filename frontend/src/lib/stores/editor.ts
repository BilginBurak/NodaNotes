import { writable, get } from 'svelte/store';
import type { Snapshot, TrashEntry, AttachmentInfoDto } from '../types';
import * as ipc from '../services/ipc';
import { activeNote, loadNotes, selectNote } from './notes';

export const snapshotsList = writable<Snapshot[]>([]);
export const attachmentsList = writable<string[]>([]);
export const attachmentsWithMetadataList = writable<AttachmentInfoDto[]>([]);
export const trashList = writable<TrashEntry[]>([]);
export const showSnapshots = writable<boolean>(false);
export const showAttachments = writable<boolean>(false);
export const editorViewMode = writable<'edit' | 'preview' | 'live'>('live');
export const loadingEditorMetadata = writable<boolean>(false);

export async function loadNoteSnapshots(noteId: string) {
  loadingEditorMetadata.set(true);
  try {
    const snaps = await ipc.listSnapshots(noteId);
    // Sort by timestamp descending
    snaps.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
    snapshotsList.set(snaps);
  } catch (e) {
    console.error('Failed to load snapshots:', e);
    snapshotsList.set([]);
  } finally {
    loadingEditorMetadata.set(false);
  }
}

export async function restoreNoteSnapshot(noteId: string, timestamp: string) {
  loadingEditorMetadata.set(true);
  try {
    await ipc.restoreSnapshot(noteId, timestamp);
    // Reload active note content and list
    await selectNote(noteId);
    await loadNoteSnapshots(noteId);
  } catch (e) {
    console.error('Failed to restore snapshot:', e);
    throw e;
  } finally {
    loadingEditorMetadata.set(false);
  }
}

export async function deleteNoteSnapshot(noteId: string, timestamp: string) {
  loadingEditorMetadata.set(true);
  try {
    await ipc.deleteSnapshot(noteId, timestamp);
    await loadNoteSnapshots(noteId);
  } catch (e) {
    console.error('Failed to delete snapshot:', e);
    throw e;
  } finally {
    loadingEditorMetadata.set(false);
  }
}

export async function loadAttachments() {
  try {
    const list = await ipc.listAttachments();
    attachmentsList.set(list);
  } catch (e) {
    console.error('Failed to load attachments:', e);
  }
}

export async function loadAttachmentsWithMetadata() {
  try {
    const list = await ipc.listAttachmentsWithMetadata();
    attachmentsWithMetadataList.set(list);
    // Keep attachmentsList in sync for backwards compatibility
    attachmentsList.set(list.map(a => a.name));
  } catch (e) {
    console.error('Failed to load attachments with metadata:', e);
  }
}

export async function uploadAttachment(sourcePath: string) {
  try {
    const uri = await ipc.addAttachment(sourcePath);
    await loadAttachmentsWithMetadata();
    return uri;
  } catch (e) {
    console.error('Failed to upload attachment:', e);
    throw e;
  }
}

export async function removeAttachment(name: string) {
  try {
    await ipc.deleteAttachment(name);
    await loadAttachmentsWithMetadata();
  } catch (e) {
    console.error('Failed to delete attachment:', e);
    throw e;
  }
}

export async function loadTrash() {
  try {
    const list = await ipc.listTrash();
    trashList.set(list);
  } catch (e) {
    console.error('Failed to load trash:', e);
  }
}

export async function sendNoteToTrash(id: string) {
  try {
    await ipc.trashNote(id);
    await loadNotes();
    await loadTrash();
  } catch (e) {
    console.error('Failed to send note to trash:', e);
    throw e;
  }
}

export async function recoverFromTrash(id: string) {
  try {
    await ipc.restoreFromTrash(id);
    await loadNotes();
    await loadTrash();
  } catch (e) {
    console.error('Failed to restore note from trash:', e);
    throw e;
  }
}

export async function emptyTrashPermanently(id: string) {
  try {
    await ipc.permanentDelete(id);
    await loadTrash();
  } catch (e) {
    console.error('Failed to delete note permanently:', e);
    throw e;
  }
}
