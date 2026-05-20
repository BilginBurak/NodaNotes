import { writable } from 'svelte/store';
import * as ipc from '../services/ipc';

export const searchQuery = writable<string>('');
export const searchResults = writable<ipc.SearchResult[]>([]);
export const searching = writable<boolean>(false);
export const searchError = writable<string | null>(null);

export async function executeSearch(query: string) {
  searchQuery.set(query);
  if (!query.trim()) {
    searchResults.set([]);
    searchError.set(null);
    return;
  }

  searching.set(true);
  searchError.set(null);
  try {
    const results = await ipc.searchNotes(query);
    searchResults.set(results);
  } catch (e: any) {
    searchError.set(e.message || 'Failed to search notes');
    searchResults.set([]);
  } finally {
    searching.set(false);
  }
}
