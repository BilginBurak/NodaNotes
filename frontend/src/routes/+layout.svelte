<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { checkActiveVault, vaultInfo } from '../lib/stores/vault';
  import { loadNotes } from '../lib/stores/notes';
  import { syncStatus, syncConflicts, lastSyncReport, showSyncReport, syncProgress } from '../lib/stores/sync';
  import { listenToVaultUpdated, listenToSyncStatus, listenToSyncConflict, listenToSyncFinished, listenToSyncProgress } from '../lib/services/events';
  import * as ipc from '../lib/services/ipc';

  import '../lib/styles/app.css'; // Let's create a beautiful global styles file!

  let unlistenUpdated: (() => void) | null = null;
  let unlistenSync: (() => void) | null = null;
  let unlistenConflicts: (() => void) | null = null;
  let unlistenSyncFinished: (() => void) | null = null;
  let unlistenSyncProgress: (() => void) | null = null;
  let globalClickListener: ((e: MouseEvent) => void) | null = null;


  $: info = $vaultInfo;

  // React to vault changes: if vault is opened, load notes immediately!
  $: if (info) {
    loadNotes();
  }

  onMount(async () => {
    try {
      // 0. Intercept global clicks on links to open in the default browser
      globalClickListener = (e: MouseEvent) => {
        let target = e.target as HTMLElement | null;
        while (target && target !== document.body) {
          if (target.tagName === 'A' && target.hasAttribute('href')) {
            const href = target.getAttribute('href');
            if (href && (href.startsWith('http://') || href.startsWith('https://') || href.startsWith('mailto:') || href.startsWith('tel:'))) {
              e.preventDefault();
              ipc.openExternalUrl(href).catch(err => {
                console.error('Failed to open external url:', err);
              });
              break;
            }
          }
          target = target.parentElement;
        }
      };
      document.addEventListener('click', globalClickListener);

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
            // Automatically trigger sync report dialog to show the error
            showSyncReport.set(true);
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
          if (c.some(item => item.archived_path === conflict.archived_path)) return c;
          return [...c, conflict];
        });
      });

      // 5. Listen for background sync finished reports
      unlistenSyncFinished = await listenToSyncFinished((report) => {
        const enriched = {
          ...report,
          total: report.uploads + report.downloads + report.deletes_local + report.deletes_remote + report.conflicts,
          completed_at: new Date().toISOString(),
        };
        lastSyncReport.set(enriched);
        showSyncReport.set(true);
      });

      // 6. Listen for sync progress events
      unlistenSyncProgress = await listenToSyncProgress((progress) => {
        syncProgress.set(progress);
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
    if (unlistenSyncFinished) unlistenSyncFinished();
    if (unlistenSyncProgress) unlistenSyncProgress();
    if (globalClickListener) {
      document.removeEventListener('click', globalClickListener);
    }
  });
</script>

<slot />
