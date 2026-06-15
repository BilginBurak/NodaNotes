<script lang="ts">
  import { onMount } from 'svelte';
  import type { AttachmentInfoDto } from '../../types';
  import { resolveAttachmentUrl } from '../../utils/attachment';

  let {
    isOpen = $bindable(false),
    attachment,
    onClose,
    onInsert,
    onDelete
  } = $props<{
    isOpen: boolean;
    attachment: AttachmentInfoDto | null;
    onClose: () => void;
    onInsert: (name: string) => void;
    onDelete: (name: string) => void;
  }>();

  // Determine file formats
  const isImage = $derived(attachment ? /\.(png|jpg|jpeg|gif|webp|svg)$/i.test(attachment.name) : false);
  const isPdf = $derived(attachment ? /\.pdf$/i.test(attachment.name) : false);
  const isTextLike = $derived(attachment ? /\.(txt|json|md|js|ts|css|html|xml|toml|yaml|yml|rs|go|py|sh|bat)$/i.test(attachment.name) : false);

  let textContent = $state('');
  let loadingText = $state(false);
  let textError = $state<string | null>(null);

  $effect(() => {
    if (isOpen && attachment && isTextLike) {
      loadingText = true;
      textError = null;
      textContent = '';
      
      const attachmentUrl = resolveAttachmentUrl(`noda://attachments/${attachment.name}`);
      fetch(attachmentUrl)
        .then(res => {
          if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
          return res.text();
        })
        .then(text => {
          textContent = text;
          loadingText = false;
        })
        .catch(err => {
          console.error('Failed to fetch attachment text content:', err);
          textError = err.message || 'Failed to load file content.';
          loadingText = false;
        });
    }
  });

  // Format bytes
  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Format date
  function formatDate(timestamp: number): string {
    if (!timestamp) return 'Unknown date';
    return new Date(timestamp).toLocaleString(undefined, {
      year: 'numeric', month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      e.preventDefault();
      onClose();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

{#if isOpen && attachment}
  <!-- Overlay Backdrop with backdrop-filter -->
  <div class="quicklook-overlay" onclick={onClose} role="dialog" aria-modal="true">
    
    <!-- Modal Container -->
    <div class="quicklook-container" onclick={(e) => e.stopPropagation()}>
      
      <!-- macOS-style Header/Toolbar -->
      <div class="quicklook-header">
        <div class="header-left">
          <!-- macOS Traffic Lights close button -->
          <button class="mac-close-btn" onclick={onClose} aria-label="Close" title="Close">
            <svg viewBox="0 0 12 12" fill="currentColor">
              <path d="M9 3L3 9M3 3L9 9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            </svg>
          </button>
          <div class="file-meta">
            <span class="file-title" title={attachment.name}>{attachment.name}</span>
            <span class="file-specs">{formatBytes(attachment.size)} · {formatDate(attachment.modified_at)}</span>
          </div>
        </div>

        <div class="header-right">
          <!-- Insert into Note button -->
          <button class="action-btn insert-btn" onclick={() => { onInsert(attachment.name); onClose(); }} title="Insert into active note">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="8" y1="3" x2="8" y2="13"/>
              <line x1="3" y1="8" x2="13" y2="8"/>
            </svg>
            <span>Insert into Note</span>
          </button>

          <!-- Delete button -->
          <button class="action-btn delete-btn" onclick={() => { onDelete(attachment.name); onClose(); }} title="Permanently delete attachment">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <polyline points="2 3 14 3"/>
              <path d="M4 3V2a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v1M5 6v7M11 6v7"/>
              <path d="M3 3l1 11a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1l1-11"/>
            </svg>
            <span>Delete</span>
          </button>
        </div>
      </div>

      <!-- Main Viewer Area -->
      <div class="quicklook-content">
        {#if isImage}
          <div class="image-viewer">
            <img src={resolveAttachmentUrl(`noda://attachments/${attachment.name}`)} alt={attachment.name} />
          </div>
        {:else if isPdf}
          <div class="pdf-viewer">
            <iframe src={resolveAttachmentUrl(`noda://attachments/${attachment.name}`)} title={attachment.name} class="pdf-iframe"></iframe>
          </div>
        {:else if isTextLike}
          <div class="text-viewer scrollbar-thin">
            {#if loadingText}
              <div class="loading-state">
                <div class="loading-spinner"></div>
                <span>Loading preview...</span>
              </div>
            {:else if textError}
              <div class="error-state">
                <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                  <circle cx="8" cy="8" r="6.5"/>
                  <line x1="8" y1="5" x2="8" y2="8.5"/>
                  <line x1="8" y1="11" x2="8" y2="11" stroke-width="2.4"/>
                </svg>
                <span>{textError}</span>
              </div>
            {:else}
              <pre><code>{textContent}</code></pre>
            {/if}
          </div>
        {:else}
          <div class="document-viewer">
            <div class="doc-badge-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                <polyline points="14 2 14 8 20 8"/>
                <line x1="16" y1="13" x2="8" y2="13"/>
                <line x1="16" y1="17" x2="8" y2="17"/>
                <polyline points="10 9 9 9 8 9"/>
              </svg>
            </div>
            <h3>Non-Previewable Attachment</h3>
            <p>This file type cannot be previewed. You can still insert it as a download link into your markdown notes or delete it permanently.</p>
          </div>
        {/if}
      </div>

    </div>
  </div>
{/if}

<style>
  /* ── Overlay ── */
  .quicklook-overlay {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    animation: fadeIn 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* ── Container ── */
  .quicklook-container {
    width: 760px;
    height: 560px;
    max-width: 90vw;
    max-height: 85vh;
    background-color: var(--toolbar-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-xl, 14px);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: scaleIn 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  /* ── Header ── */
  .quicklook-header {
    height: 52px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    background-color: rgba(255, 255, 255, 0.02);
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    overflow: hidden;
  }

  /* macOS Close traffic light style */
  .mac-close-btn {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background-color: #ff5f56;
    border: 1px solid #e0443e;
    color: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    padding: 0;
  }

  .mac-close-btn svg {
    opacity: 0;
    width: 8px;
    height: 8px;
    color: #4c0002;
    transition: opacity 0.15s ease;
  }

  .mac-close-btn:hover {
    background-color: #ff3b30;
  }

  .mac-close-btn:hover svg {
    opacity: 1;
  }

  .file-meta {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .file-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 320px;
  }

  .file-specs {
    font-size: 10px;
    color: var(--text-tertiary);
    margin-top: 1px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Action Buttons */
  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: var(--radius-md, 6px);
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .action-btn svg {
    width: 12px;
    height: 12px;
  }

  .insert-btn {
    background-color: var(--accent);
    border: none;
    color: #ffffff;
  }

  .insert-btn:hover {
    background-color: var(--accent-hover);
  }

  .insert-btn:active {
    transform: scale(0.97);
  }

  .delete-btn {
    background-color: transparent;
    border: 1px solid var(--border-normal);
    color: var(--color-red, #ff453a);
  }

  .delete-btn:hover {
    background-color: var(--color-red-muted, rgba(255, 69, 58, 0.15));
    border-color: rgba(255, 69, 58, 0.3);
  }

  /* ── Content View ── */
  .quicklook-content {
    flex: 1;
    overflow: hidden;
    background-color: rgba(0, 0, 0, 0.05);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    box-sizing: border-box;
  }

  .image-viewer {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .image-viewer img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: var(--radius-md, 6px);
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.3);
  }

  .document-viewer {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 40px;
    max-width: 480px;
  }

  .doc-badge-icon {
    width: 64px;
    height: 64px;
    color: var(--text-disabled);
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg, 10px);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 20px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  .doc-badge-icon svg {
    width: 32px;
    height: 32px;
  }

  .document-viewer h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .document-viewer p {
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-tertiary);
    margin: 8px 0 0;
  }

  /* ── PDF Viewer ── */
  .pdf-viewer {
    width: 100%;
    height: 100%;
    overflow: hidden;
    border-radius: var(--radius-md, 6px);
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.3);
    background-color: var(--bg-control);
  }

  .pdf-iframe {
    width: 100%;
    height: 100%;
    border: none;
    background-color: #ffffff;
  }

  /* ── Text/Code Viewer ── */
  .text-viewer {
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md, 6px);
    padding: 16px;
    box-sizing: border-box;
    overflow-y: auto;
    font-family: var(--font-mono, Menlo, Monaco, Consolas, 'Courier New', monospace);
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-primary);
    text-align: left;
  }

  .text-viewer pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .text-viewer code {
    font-family: inherit;
    color: inherit;
  }

  /* ── Status States ── */
  .loading-state, .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-normal);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .error-state {
    color: var(--color-red, #ff453a);
  }

  .error-state svg {
    width: 24px;
    height: 24px;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* ── Keyframes ── */
  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes scaleIn {
    from { transform: scale(0.96); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }
</style>
