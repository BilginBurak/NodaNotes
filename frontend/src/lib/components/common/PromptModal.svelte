<script lang="ts">
  import { onMount } from 'svelte';
  import { promptStore, closePrompt } from '../../stores/prompt';

  const state = $derived($promptStore);

  let inputEl = $state<HTMLInputElement | null>(null);
  let value = $state('');
  let validationError = $state<string | null>(null);

  // Focus input and set value when prompt is opened
  $effect(() => {
    if (state.show) {
      value = state.value;
      validationError = null;
      setTimeout(() => {
        if (inputEl) {
          inputEl.focus();
          inputEl.select();
        }
      }, 50);
    }
  });

  async function handleSave() {
    if (!state.onSubmit) return;
    
    // Custom Validation
    if (state.validation) {
      const err = state.validation(value);
      if (err) {
        validationError = err;
        return;
      }
    }

    if (!value.trim()) {
      validationError = 'This field cannot be empty.';
      return;
    }

    try {
      await state.onSubmit(value);
      closePrompt();
    } catch (e: any) {
      validationError = e.message || 'An error occurred';
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSave();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closePrompt();
    }
  }
</script>

{#if state.show}
  <div 
    class="prompt-overlay" 
    onclick={closePrompt}
    role="presentation"
  >
    <!-- Modal Card -->
    <div 
      class="prompt-card" 
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="prompt-title"
    >
      <h3 id="prompt-title" class="prompt-title">{state.title}</h3>
      
      <div class="prompt-body">
        <input
          bind:this={inputEl}
          bind:value={value}
          class="prompt-input"
          class:has-error={!!validationError}
          placeholder={state.placeholder}
          onkeydown={handleKeyDown}
          autocomplete="off"
          spellcheck="false"
        />

        {#if validationError}
          <div class="prompt-error" role="alert">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
              <circle cx="8" cy="8" r="6.5"/>
              <line x1="8" y1="5" x2="8" y2="8.5"/>
              <line x1="8" y1="11" x2="8" y2="11" stroke-width="2.5"/>
            </svg>
            <span>{validationError}</span>
          </div>
        {/if}
      </div>

      <div class="prompt-footer">
        <button class="prompt-btn cancel" onclick={closePrompt}>Cancel</button>
        <button class="prompt-btn save" onclick={handleSave}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .prompt-overlay {
    position: fixed;
    inset: 0;
    z-index: 999998;
    background-color: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .prompt-card {
    width: 320px;
    background-color: var(--modal-bg);
    border: 1px solid var(--border-normal);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    animation: promptScaleIn 0.25s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .prompt-title {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.2px;
  }

  .prompt-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .prompt-input {
    width: 100%;
    background-color: var(--bg-control);
    border: 1.5px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 8px 12px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-sans);
    outline: none;
    box-sizing: border-box;
    transition: all 0.15s ease;
    user-select: text;
  }

  .prompt-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-muted);
    background-color: var(--bg-elevated);
  }

  .prompt-input.has-error {
    border-color: var(--color-red);
    box-shadow: 0 0 0 3px var(--color-red-muted);
  }

  .prompt-error {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--color-red);
    font-size: 11px;
    font-weight: 500;
    margin-top: 2px;
  }

  .prompt-error svg {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
  }

  .prompt-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  .prompt-btn {
    font-family: var(--font-sans);
    font-size: 12px;
    font-weight: 600;
    padding: 7px 14px;
    border-radius: var(--radius-md);
    border: none;
    cursor: pointer;
    transition: all 0.12s ease;
    min-width: 68px;
  }

  .prompt-btn.cancel {
    background-color: var(--bg-control);
    border: 1.5px solid var(--border-subtle);
    color: var(--text-secondary);
  }

  .prompt-btn.cancel:hover {
    background-color: var(--bg-control-hover);
    color: var(--text-primary);
  }

  .prompt-btn.save {
    background-color: var(--accent);
    color: white;
  }

  .prompt-btn.save:hover {
    background-color: var(--accent-hover);
  }

  .prompt-btn:active {
    transform: scale(0.97);
  }

  @keyframes promptScaleIn {
    from {
      opacity: 0;
      transform: scale(0.93);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
