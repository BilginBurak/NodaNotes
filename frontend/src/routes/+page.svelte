<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { vaultInfo, openExistingVault, createNewVault, vaultError, loadingVault } from '../lib/stores/vault';
  import Sidebar from '../lib/components/sidebar/Sidebar.svelte';
  import NoteList from '../lib/components/notelist/NoteList.svelte';
  import Editor from '../lib/components/editor/Editor.svelte';
  import Toolbar from '../lib/components/toolbar/Toolbar.svelte';

  $: info = $vaultInfo;
  $: error = $vaultError;
  $: loading = $loadingVault;

  async function handleOpenVault() {
    console.log('handleOpenVault starting...');
    vaultError.set(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Existing Vault Folder'
      });
      console.log('Selected path:', selected);
      
      if (selected && typeof selected === 'string') {
        await openExistingVault(selected);
      } else {
        console.log('No folder selected or selection cancelled');
      }
    } catch (e: any) {
      console.error('Failed to open vault:', e);
      vaultError.set(`Dialog error: ${e.message || e.toString()}`);
    }
  }

  async function handleCreateVault() {
    console.log('handleCreateVault starting...');
    vaultError.set(null);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Choose Folder for New Vault'
      });
      console.log('Selected path:', selected);
      
      if (selected && typeof selected === 'string') {
        await createNewVault(selected);
      } else {
        console.log('No folder selected or selection cancelled');
      }
    } catch (e: any) {
      console.error('Failed to create vault:', e);
      vaultError.set(`Dialog error: ${e.message || e.toString()}`);
    }
  }
</script>

{#if !info}
  <!-- Gorgeous Launcher screen with blurry background circles -->
  <div class="launcher-shell">
    <div class="bg-glow purple"></div>
    <div class="bg-glow indigo"></div>

    <div class="launcher-card border-glow">
      <div class="launcher-header">
        <div class="logo-animation">
          <div class="logo-ring">
            <div class="logo-dot"></div>
          </div>
        </div>
        <h1>Noda Notes</h1>
        <p class="subtitle">State-of-the-Art Private Markdown Vault</p>
      </div>

      {#if error}
        <div class="error-banner">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-5 h-5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z" />
          </svg>
          <div class="error-text">
            <h4>Vault Operation Failed</h4>
            <p>{error}</p>
          </div>
        </div>
      {/if}

      <div class="launcher-actions">
        <button class="action-card hover-glow open" onclick={handleOpenVault} disabled={loading}>
          <div class="card-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" class="w-7 h-7">
              <path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A9 9 0 0 1 12 3v0a9 9 0 0 1 9 9v.75m-18 0a2.25 2.25 0 0 0 2.25 2.25h13.5A2.25 2.25 0 0 0 21 12.75m-18 0V12a9 9 0 0 1 9-9v0a9 9 0 0 1 9 9v.75m-18 0a2.25 2.25 0 0 1 2.25-2.25h13.5A2.25 2.25 0 0 1 21 12.75m-18 0v1.5a2.25 2.25 0 0 0 2.25 2.25h13.5a2.25 2.25 0 0 0 2.25-2.25v-1.5m-18 0V12a9 9 0 0 0 9 9v0a9 9 0 0 0 9-9v-.75" />
            </svg>
          </div>
          <div class="card-details">
            <h3>Open Existing Vault</h3>
            <p>Select a folder already formatted with a `.noda` index.</p>
          </div>
        </button>

        <button class="action-card hover-glow create" onclick={handleCreateVault} disabled={loading}>
          <div class="card-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" class="w-7 h-7">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
            </svg>
          </div>
          <div class="card-details">
            <h3>Create New Vault</h3>
            <p>Select an empty directory to initialize a secure local database.</p>
          </div>
        </button>
      </div>

      {#if loading}
        <div class="launcher-loading">
          <div class="spinner"></div>
          <span>Mounting vault structures...</span>
        </div>
      {/if}

      <div class="launcher-footer">
        <span>Version 2.0.0 (Tauri + Rust)</span>
        <span class="dot">•</span>
        <span>End-to-End Encryption Capable</span>
      </div>
    </div>
  </div>
{:else}
  <!-- Gorgeous Three-Panel Dashboard Workspace -->
  <div class="app-container">
    <Toolbar />
    <div class="app-workspace">
      <Sidebar />
      <NoteList />
      <div class="main-content">
        <Editor />
      </div>
    </div>
  </div>
{/if}

<style>
  /* Launcher Styling */
  .launcher-shell {
    position: fixed;
    inset: 0;
    background-color: #030712;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  /* Blurry decorative backgrounds */
  .bg-glow {
    position: absolute;
    width: 350px;
    height: 350px;
    border-radius: 50%;
    filter: blur(140px);
    opacity: 0.15;
    z-index: 1;
    pointer-events: none;
  }

  .bg-glow.purple {
    background-color: #8b5cf6;
    top: 20%;
    left: 25%;
    animation: floatPurple 8s ease-in-out infinite alternate;
  }

  .bg-glow.indigo {
    background-color: #4f46e5;
    bottom: 20%;
    right: 25%;
    animation: floatIndigo 8s ease-in-out infinite alternate;
  }

  @keyframes floatPurple {
    from { transform: translate(0, 0) scale(1); }
    to { transform: translate(40px, 30px) scale(1.1); }
  }

  @keyframes floatIndigo {
    from { transform: translate(0, 0) scale(1); }
    to { transform: translate(-40px, -30px) scale(1.15); }
  }

  .launcher-card {
    position: relative;
    width: 480px;
    background-color: rgba(13, 17, 26, 0.7);
    backdrop-filter: blur(20px);
    border-radius: 16px;
    padding: 40px;
    z-index: 10;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .border-glow {
    border: 1px solid rgba(255, 255, 255, 0.05);
    box-shadow: 0 0 40px rgba(99, 102, 241, 0.05), inset 0 1px 0 rgba(255, 255, 255, 0.05);
  }

  .launcher-header {
    text-align: center;
    margin-bottom: 30px;
  }

  .logo-animation {
    display: inline-flex;
    margin-bottom: 16px;
  }

  .logo-ring {
    width: 32px;
    height: 32px;
    border: 3px solid #6366f1;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 0 20px rgba(99, 102, 241, 0.4);
    animation: pulseRing 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  .logo-dot {
    width: 8px;
    height: 8px;
    background-color: #818cf8;
    border-radius: 50%;
  }

  @keyframes pulseRing {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.08); opacity: 0.9; }
  }

  .launcher-header h1 {
    font-family: 'Outfit', sans-serif;
    font-size: 2rem;
    font-weight: 700;
    color: #ffffff;
    margin: 0 0 6px 0;
    letter-spacing: -0.5px;
  }

  .subtitle {
    font-size: 0.84rem;
    color: #64748b;
    margin: 0;
  }

  /* Error Banner */
  .error-banner {
    width: 100%;
    background-color: rgba(244, 63, 94, 0.06);
    border: 1px solid rgba(244, 63, 94, 0.15);
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 24px;
    box-sizing: border-box;
    color: #f43f5e;
  }

  .error-text h4 {
    margin: 0 0 2px 0;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .error-text p {
    margin: 0;
    font-size: 0.74rem;
    opacity: 0.85;
    line-height: 1.3;
  }

  .launcher-actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 100%;
  }

  .action-card {
    display: flex;
    align-items: center;
    width: 100%;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.04);
    border-radius: 10px;
    padding: 16px 20px;
    gap: 16px;
    cursor: pointer;
    text-align: left;
    transition: all 0.2s ease;
    box-sizing: border-box;
  }

  .action-card:hover:not(:disabled) {
    background-color: rgba(255, 255, 255, 0.04);
    border-color: rgba(99, 102, 241, 0.25);
  }

  .action-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .card-icon {
    width: 44px;
    height: 44px;
    border-radius: 8px;
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.04);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: #94a3b8;
    transition: all 0.2s ease;
  }

  .action-card:hover .card-icon {
    background-color: rgba(99, 102, 241, 0.1);
    border-color: rgba(99, 102, 241, 0.2);
    color: #818cf8;
  }

  .action-card.open:hover .card-icon {
    box-shadow: 0 0 10px rgba(99, 102, 241, 0.2);
  }

  .action-card.create:hover .card-icon {
    background-color: rgba(16, 185, 129, 0.1);
    border-color: rgba(16, 185, 129, 0.2);
    color: #10b981;
    box-shadow: 0 0 10px rgba(16, 185, 129, 0.2);
  }

  .card-details h3 {
    margin: 0 0 2px 0;
    font-size: 0.88rem;
    font-weight: 600;
    color: #cbd5e1;
    transition: color 0.2s ease;
  }

  .action-card:hover .card-details h3 {
    color: #ffffff;
  }

  .card-details p {
    margin: 0;
    font-size: 0.74rem;
    color: #475569;
    line-height: 1.3;
  }

  .launcher-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #818cf8;
    font-size: 0.76rem;
    margin-top: 20px;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.05);
    border-top-color: #6366f1;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .launcher-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #334155;
    font-size: 0.65rem;
    margin-top: 30px;
    font-weight: 500;
  }

  .launcher-footer .dot {
    opacity: 0.5;
  }

  /* App workspace panel styling */
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background-color: #0a0d14;
  }

  .app-workspace {
    display: flex;
    flex: 1;
    height: calc(100vh - 48px);
    overflow: hidden;
  }

  .main-content {
    display: flex;
    flex: 1;
    height: 100%;
    overflow: hidden;
    position: relative;
  }
</style>
