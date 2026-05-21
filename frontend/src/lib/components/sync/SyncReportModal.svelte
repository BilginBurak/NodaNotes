<script lang="ts">
  import { lastSyncReport, showSyncReport, dismissSyncReport, triggerSyncNow } from '../../stores/sync';

  $: report = $lastSyncReport;
  $: visible = $showSyncReport;

  let showDetail = false;

  function handleBadgeClick() {
    showDetail = true;
  }

  function closeDetail() {
    showDetail = false;
  }

  function handleDismiss() {
    dismissSyncReport();
    showDetail = false;
  }

  async function handleSyncAgain() {
    closeDetail();
    try {
      await triggerSyncNow();
    } catch (e) {
      console.error('Re-sync failed:', e);
    }
  }

  function formatTime(isoStr?: string): string {
    if (!isoStr) return '—';
    return new Date(isoStr).toLocaleTimeString(undefined, {
      hour: '2-digit', minute: '2-digit', second: '2-digit'
    });
  }

  // Raporun "ilgi çekici" olup olmadığını belirle (0'dan fazla işlem varsa)
  $: hasActivity = report ? (report.uploads + report.downloads + report.deletes_local + report.deletes_remote + report.conflicts) > 0 : false;
  $: hasErrors   = report ? report.conflicts > 0 : false;
</script>

{#if visible && report}
  <!-- Sağ alt toast bildirimi -->
  <div
    class="sync-toast"
    class:has-activity={hasActivity}
    class:has-errors={hasErrors}
    role="status"
    aria-live="polite"
    aria-label="Sync completed"
  >
    <div class="toast-icon" aria-hidden="true">
      {#if hasErrors}
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M8 2L14 13H2L8 2z"/>
          <line x1="8" y1="7" x2="8" y2="9.5"/>
          <line x1="8" y1="11.5" x2="8" y2="11.5" stroke-width="2.4"/>
        </svg>
      {:else}
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="2,8 6,12 14,4"/>
        </svg>
      {/if}
    </div>

    <div class="toast-content">
      <div class="toast-title">
        Sync {hasErrors ? 'completed with conflicts' : 'complete'}
      </div>
      <div class="toast-summary">
        {#if report.uploads > 0}↑{report.uploads}{/if}
        {#if report.downloads > 0}{report.uploads > 0 ? ' · ' : ''}↓{report.downloads}{/if}
        {#if report.conflicts > 0} · {report.conflicts}⚠{/if}
        {#if !hasActivity}No changes{/if}
        <span class="toast-time">· {formatTime(report.completed_at)}</span>
      </div>
    </div>

    <div class="toast-actions">
      <button class="toast-btn-detail" onclick={handleBadgeClick} title="View full report">
        Details
      </button>
      <button class="toast-btn-close" onclick={handleDismiss} title="Dismiss" aria-label="Dismiss sync notification">
        <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
          <line x1="2" y1="2" x2="10" y2="10"/>
          <line x1="10" y1="2" x2="2" y2="10"/>
        </svg>
      </button>
    </div>
  </div>

  <!-- Detay modal -->
  {#if showDetail}
    <div class="modal-backdrop" onclick={closeDetail} role="presentation"></div>
    <div class="sync-detail-modal" role="dialog" aria-modal="true" aria-label="Sync Report">
      <div class="modal-header">
        <div class="modal-title-group">
          <div class="modal-icon" class:error-icon={hasErrors} aria-hidden="true">
            {#if hasErrors}
              <svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M9 2L16 15H2L9 2z"/>
                <line x1="9" y1="8" x2="9" y2="11"/>
                <line x1="9" y1="13.5" x2="9" y2="13.5" stroke-width="2.4"/>
              </svg>
            {:else}
              <svg viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="9" cy="9" r="7"/>
                <polyline points="5,9 7.5,11.5 13,6.5"/>
              </svg>
            {/if}
          </div>
          <div>
            <h3>Sync Report</h3>
            <span class="modal-subtitle">{formatTime(report.completed_at)}</span>
          </div>
        </div>
        <button class="modal-close" onclick={closeDetail} aria-label="Close">
          <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
            <line x1="2" y1="2" x2="12" y2="12"/>
            <line x1="12" y1="2" x2="2" y2="12"/>
          </svg>
        </button>
      </div>

      <div class="modal-body">
        <!-- Özet kartları -->
        <div class="stat-grid">
          <div class="stat-card" class:active={report.uploads > 0}>
            <div class="stat-icon upload-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <line x1="8" y1="12" x2="8" y2="4"/>
                <polyline points="5,7 8,4 11,7"/>
                <path d="M3 13h10"/>
              </svg>
            </div>
            <div class="stat-value">{report.uploads}</div>
            <div class="stat-label">Uploaded</div>
          </div>

          <div class="stat-card" class:active={report.downloads > 0}>
            <div class="stat-icon download-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <line x1="8" y1="4" x2="8" y2="12"/>
                <polyline points="5,9 8,12 11,9"/>
                <path d="M3 13h10"/>
              </svg>
            </div>
            <div class="stat-value">{report.downloads}</div>
            <div class="stat-label">Downloaded</div>
          </div>

          <div class="stat-card" class:active={report.deletes_remote > 0}>
            <div class="stat-icon delete-remote-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3,4 13,4"/>
                <path d="M6 4V3a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v1M4 4l.8 9a1 1 0 0 0 1 .9h4.4a1 1 0 0 0 1-.9L12 4"/>
              </svg>
            </div>
            <div class="stat-value">{report.deletes_remote}</div>
            <div class="stat-label">Remote deletes</div>
          </div>

          <div class="stat-card" class:active={report.deletes_local > 0}>
            <div class="stat-icon delete-local-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="3" width="12" height="10" rx="1.5"/>
                <line x1="5" y1="7" x2="11" y2="7"/>
              </svg>
            </div>
            <div class="stat-value">{report.deletes_local}</div>
            <div class="stat-label">Local deletes</div>
          </div>

          <div class="stat-card" class:warning={report.conflicts > 0}>
            <div class="stat-icon conflict-icon" aria-hidden="true">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M8 1.5L14.5 13H1.5L8 1.5z"/>
                <line x1="8" y1="6" x2="8" y2="9"/>
                <line x1="8" y1="11" x2="8" y2="11" stroke-width="2.4"/>
              </svg>
            </div>
            <div class="stat-value">{report.conflicts}</div>
            <div class="stat-label">Conflicts</div>
          </div>
        </div>

        <!-- Toplam özet -->
        <div class="total-row">
          <span class="total-label">Total operations</span>
          <span class="total-value">{report.total ?? 0}</span>
        </div>

        <!-- Detaylı Dosya Listeleri -->
        <div class="file-details scrollbar-thin">
          {#if report.uploaded_files?.length > 0}
            <div class="file-group">
              <h4 class="color-upload">Uploaded</h4>
              <ul>
                {#each report.uploaded_files as file}<li>{file}</li>{/each}
              </ul>
            </div>
          {/if}
          
          {#if report.downloaded_files?.length > 0}
            <div class="file-group">
              <h4 class="color-download">Downloaded</h4>
              <ul>
                {#each report.downloaded_files as file}<li>{file}</li>{/each}
              </ul>
            </div>
          {/if}

          {#if report.deleted_local_files?.length > 0}
            <div class="file-group">
              <h4 class="color-delete">Local Deletes</h4>
              <ul>
                {#each report.deleted_local_files as file}<li>{file}</li>{/each}
              </ul>
            </div>
          {/if}

          {#if report.deleted_remote_files?.length > 0}
            <div class="file-group">
              <h4 class="color-delete">Remote Deletes</h4>
              <ul>
                {#each report.deleted_remote_files as file}<li>{file}</li>{/each}
              </ul>
            </div>
          {/if}
          
          {#if report.conflict_files?.length > 0}
            <div class="file-group">
              <h4 class="color-conflict">Conflicts</h4>
              <ul>
                {#each report.conflict_files as file}<li>{file}</li>{/each}
              </ul>
            </div>
          {/if}
        </div>

        {#if report.conflicts > 0}
          <div class="conflict-info">
            <svg viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M7 1.5L13 12.5H1L7 1.5z"/>
              <line x1="7" y1="5.5" x2="7" y2="8.5"/>
              <line x1="7" y1="10.5" x2="7" y2="10.5" stroke-width="2.2"/>
            </svg>
            <p>
              {report.conflicts} conflict{report.conflicts > 1 ? 's were' : ' was'} detected.
              Your local version was kept and the remote copy was archived in <code>.noda/conflicts/</code>.
            </p>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn btn-ghost" onclick={handleSyncAgain}>
          Sync Again
        </button>
        <button class="btn btn-secondary" onclick={handleDismiss}>
          Dismiss
        </button>
      </div>
    </div>
  {/if}
{/if}

<style>
  /* ── Toast bildirimi ── */
  .sync-toast {
    position: fixed;
    bottom: 24px;
    right: 20px;
    display: flex;
    align-items: center;
    gap: 10px;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-lg);
    padding: 10px 12px;
    box-shadow: var(--shadow-lg);
    z-index: 20000;
    animation: slideUp 0.22s cubic-bezier(0.16, 1, 0.3, 1);
    max-width: 320px;
  }

  @keyframes slideUp {
    from { opacity: 0; transform: translateY(12px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .sync-toast.has-activity {
    border-color: var(--accent-border);
  }

  .sync-toast.has-errors {
    border-color: rgba(255, 159, 10, 0.40);
  }

  .toast-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background-color: var(--color-green-muted);
    color: var(--color-green);
    flex-shrink: 0;
  }

  .toast-icon svg { width: 14px; height: 14px; display: block; }

  .sync-toast.has-errors .toast-icon {
    background-color: var(--color-orange-muted);
    color: var(--color-orange);
  }

  .toast-content { flex: 1; overflow: hidden; }

  .toast-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .toast-summary {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
  }

  .toast-time { opacity: 0.7; }

  .toast-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .toast-btn-detail {
    background: transparent;
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-sm);
    color: var(--accent);
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    padding: 3px 8px;
    cursor: pointer;
    transition: all 0.12s ease;
    white-space: nowrap;
  }

  .toast-btn-detail:hover {
    background-color: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .toast-btn-close {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
  }

  .toast-btn-close svg { width: 10px; height: 10px; display: block; }
  .toast-btn-close:hover { color: var(--text-primary); background-color: var(--bg-hover); }

  /* ── Detay Modal ── */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: var(--overlay-bg);
    backdrop-filter: blur(4px);
    z-index: 20001;
  }

  .sync-detail-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 440px;
    max-width: 92vw;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-xl);
    z-index: 20002;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-xl);
    animation: zoomIn 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    overflow: hidden;
  }

  /* Modal header */
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .modal-title-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .modal-icon {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background-color: var(--color-green-muted);
    color: var(--color-green);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .modal-icon svg { width: 16px; height: 16px; display: block; }
  .modal-icon.error-icon { background-color: var(--color-orange-muted); color: var(--color-orange); }

  .modal-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.2px;
    line-height: 1;
  }

  .modal-subtitle {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .modal-close {
    background: transparent;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
  }

  .modal-close svg { width: 13px; height: 13px; display: block; }
  .modal-close:hover { color: var(--text-primary); background-color: var(--bg-hover); }

  /* Modal body */
  .modal-body {
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* Stat grid */
  .stat-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
  }

  .stat-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 6px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    transition: all 0.12s ease;
  }

  .stat-card.active { border-color: var(--accent-border); background-color: var(--accent-muted); }
  .stat-card.warning { border-color: rgba(255,159,10,0.35); background-color: var(--color-orange-muted); }

  .stat-icon {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-tertiary);
  }

  .stat-icon svg { width: 14px; height: 14px; display: block; }

  .stat-card.active .stat-icon { color: var(--accent); }
  .stat-card.warning .stat-icon { color: var(--color-orange); }

  .upload-icon   { color: var(--accent); }
  .download-icon { color: var(--color-green); }

  .stat-value {
    font-size: 18px;
    font-weight: 700;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .stat-card.active .stat-value { color: var(--accent); }
  .stat-card.warning .stat-value { color: var(--color-orange); }

  .stat-label {
    font-size: 9px;
    color: var(--text-disabled);
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    font-weight: 600;
    white-space: nowrap;
  }

  /* Toplam satır */
  .total-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
  }

  .total-label { font-size: 12px; color: var(--text-secondary); }
  .total-value { font-size: 16px; font-weight: 700; color: var(--text-primary); font-variant-numeric: tabular-nums; }

  /* Conflict bilgisi */
  .conflict-info {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    background-color: var(--color-orange-muted);
    border: 1px solid rgba(255,159,10,0.25);
    border-radius: var(--radius-md);
    color: var(--color-orange);
  }

  .conflict-info svg { width: 14px; height: 14px; flex-shrink: 0; margin-top: 1px; }
  .conflict-info p { font-size: 11px; margin: 0; line-height: 1.4; color: rgba(255,159,10,0.9); }

  .conflict-info code {
    font-family: var(--font-mono);
    font-size: 10px;
    background-color: rgba(255,159,10,0.12);
    border-radius: 3px;
    padding: 1px 4px;
  }

  /* Detaylı Dosya Listeleri */
  .file-details {
    max-height: 200px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-right: 4px;
  }

  .file-group h4 {
    margin: 0 0 4px 0;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .file-group ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .file-group li {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    background-color: var(--bg-control);
    padding: 4px 8px;
    border-radius: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .color-upload { color: var(--accent); }
  .color-download { color: var(--color-green); }
  .color-delete { color: var(--text-tertiary); }
  .color-conflict { color: var(--color-orange); }

  /* Scrollbar for file list */
  .scrollbar-thin::-webkit-scrollbar { width: 5px; }
  .scrollbar-thin::-webkit-scrollbar-track { background: transparent; }
  .scrollbar-thin::-webkit-scrollbar-thumb {
    background: var(--scrollbar-thumb);
    border-radius: var(--radius-pill);
  }
  .scrollbar-thin::-webkit-scrollbar-thumb:hover { background: var(--scrollbar-thumb-hover); }

  /* Footer */
  .modal-footer {
    padding: 12px 18px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    background-color: var(--bg-window);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: var(--radius-md);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    font-family: var(--font-sans);
    transition: all 0.12s ease;
    border: 1px solid transparent;
  }

  .btn-secondary {
    background-color: var(--bg-control);
    color: var(--text-secondary);
    border-color: var(--border-normal);
  }

  .btn-secondary:hover {
    background-color: var(--bg-control-hover);
    color: var(--text-primary);
  }

  .btn-ghost {
    background-color: transparent;
    color: var(--accent);
    border-color: var(--accent-border);
  }

  .btn-ghost:hover {
    background-color: var(--accent-muted);
  }
</style>
