<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { checkActiveVault, vaultInfo } from '../lib/stores/vault';
  import { loadNotes } from '../lib/stores/notes';
  import { syncStatus, syncConflicts } from '../lib/stores/sync';
  import { listenToVaultUpdated, listenToSyncStatus, listenToSyncConflict } from '../lib/services/events';
  import '../lib/styles/app.css'; // Let's create a beautiful global styles file!

  let unlistenUpdated: (() => void) | null = null;
  let unlistenSync: (() => void) | null = null;
  let unlistenConflicts: (() => void) | null = null;

  $: info = $vaultInfo;

  // React to vault changes: if vault is opened, load notes immediately!
  $: if (info) {
    loadNotes();
  }

  onMount(async () => {
    try {
      // 1. Listen for background file updates (Watcher)
      unlistenUpdated = await listenToVaultUpdated((payload) => {
        console.log('Vault updated in background:', payload);
        loadNotes();
      });

      // 2. Listen for background sync status updates
      unlistenSync = await listenToSyncStatus((status: any) => {
        syncStatus.update(current => {
          if (typeof status === 'string') {
            if (status === 'Idle') return { status: 'Idle', last_sync_time: current.last_sync_time };
            if (status === 'Syncing') return { status: 'Syncing', last_sync_time: current.last_sync_time };
          } else if (typeof status === 'object' && status !== null && status.Error) {
            return { status: 'Error', error_message: status.Error, last_sync_time: current.last_sync_time };
          }
          // fallback
          return { status: 'Idle', last_sync_time: current.last_sync_time };
        });
      });

      // 3. Listen for sync conflicts
      unlistenConflicts = await listenToSyncConflict((conflict) => {
        syncConflicts.update(c => {
          // Prevent duplicates
          if (c.some(item => item.remote_path === conflict.remote_path)) return c;
          return [...c, conflict];
        });
      });

      // 4. Auto-restore last open vault
      await checkActiveVault();
    } catch (e) {
      console.error('Failed to initialize Tauri event listeners:', e);
    }
  });

  onDestroy(() => {
    if (unlistenUpdated) unlistenUpdated();
    if (unlistenSync) unlistenSync();
    if (unlistenConflicts) unlistenConflicts();
  });
</script>

<slot />
