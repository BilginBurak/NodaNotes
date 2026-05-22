import { writable } from 'svelte/store';

export interface ContextMenuState {
  show: boolean;
  x: number;
  y: number;
  type: 'folder' | 'note' | 'root';
  relPath: string;
  noteId: string;
  noteTitle: string;
}

const initialState: ContextMenuState = {
  show: false,
  x: 0,
  y: 0,
  type: 'root',
  relPath: '',
  noteId: '',
  noteTitle: ''
};

export const contextMenuStore = writable<ContextMenuState>(initialState);

export function openContextMenu(
  e: MouseEvent,
  type: 'folder' | 'note' | 'root',
  relPath: string,
  noteId = '',
  noteTitle = ''
) {
  e.preventDefault();
  e.stopPropagation();

  // Prevent opening outside viewport boundaries
  const menuWidth = 180;
  const menuHeight = 160;
  const x = Math.min(e.clientX, window.innerWidth - menuWidth - 8);
  const y = Math.min(e.clientY, window.innerHeight - menuHeight - 8);

  contextMenuStore.set({
    show: true,
    x,
    y,
    type,
    relPath,
    noteId,
    noteTitle
  });
}

export function closeContextMenu() {
  contextMenuStore.set(initialState);
}
