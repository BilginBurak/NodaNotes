<script lang="ts">
  import { marked } from 'marked';
  import { activeNote } from '../../stores/notes';
  import * as ipc from '../../services/ipc';
  import { rewriteHtmlAttachments } from '../../utils/attachment';

  export let content: string = '';

  $: html = rewriteHtmlAttachments(marked.parse(content) as string);

  function handlePreviewClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' && (target as HTMLInputElement).type === 'checkbox') {
      // Previews are disabled by default, we intercept click/doubleclick
      e.preventDefault();
      e.stopPropagation();
      
      const parentLi = target.closest('li');
      const lineContent = parentLi ? parentLi.textContent?.trim() || '' : '';
      const note = $activeNote;
      if (note && lineContent) {
        ipc.toggleTaskStatus(note.id, lineContent).then((updated) => {
          activeNote.set(updated);
        }).catch((err) => {
          console.error("Failed to toggle task status:", err);
        });
      }
    }
  }
</script>

<div class="markdown-preview scrollbar-thin" onclick={handlePreviewClick} ondblclick={handlePreviewClick} role="presentation">
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html}
</div>

<style>
  .markdown-preview {
    padding: 32px 60px;
    height: 100%;
    overflow-y: auto;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 15px;
    line-height: 1.7;
    background-color: var(--bg-editor);
    max-width: 900px;
    margin: 0 auto;
    box-sizing: border-box;
    user-select: text;
  }

  :global(.markdown-preview input[type="checkbox"]) {
    pointer-events: auto !important;
    cursor: pointer;
  }

  :global(.markdown-preview h1) {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 0.5em 0;
    letter-spacing: -0.3px;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 0.4em;
    line-height: 1.2;
  }

  :global(.markdown-preview h2) {
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 1.4em 0 0.4em 0;
    letter-spacing: -0.2px;
  }

  :global(.markdown-preview h3) {
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 1.2em 0 0.4em 0;
  }

  :global(.markdown-preview h4, .markdown-preview h5, .markdown-preview h6) {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 1em 0 0.3em 0;
  }

  :global(.markdown-preview p) {
    margin: 0 0 1em 0;
    color: var(--text-secondary);
  }

  :global(.markdown-preview strong) {
    font-weight: 700;
    color: var(--text-primary);
  }

  :global(.markdown-preview em) {
    color: var(--text-secondary);
    font-style: italic;
  }

  :global(.markdown-preview a) {
    color: var(--accent);
    text-decoration: none;
    transition: color 0.15s ease;
  }

  :global(.markdown-preview a:hover) {
    color: var(--accent-hover);
    text-decoration: underline;
  }

  :global(.markdown-preview code) {
    background-color: var(--bg-control);
    border: 1px solid var(--border-subtle);
    padding: 0.15em 0.4em;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.88em;
    color: var(--accent-hover);
  }

  :global(.markdown-preview pre) {
    background-color: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    padding: 16px 20px;
    border-radius: var(--radius-md);
    overflow-x: auto;
    margin: 1.2em 0;
  }

  :global(.markdown-preview pre code) {
    background: none;
    border: none;
    padding: 0;
    font-size: 13px;
    color: var(--text-primary);
  }

  :global(.markdown-preview ul, .markdown-preview ol) {
    padding-left: 22px;
    margin-bottom: 1em;
    color: var(--text-secondary);
  }

  :global(.markdown-preview li) {
    margin-bottom: 0.3em;
  }

  :global(.markdown-preview blockquote) {
    margin: 1.2em 0;
    padding: 10px 16px;
    color: var(--text-secondary);
    border-left: 3px solid var(--accent);
    background-color: var(--accent-muted);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
  }

  :global(.markdown-preview blockquote p) {
    margin: 0;
  }

  :global(.markdown-preview img) {
    max-width: 100%;
    border-radius: var(--radius-md);
    margin: 1.2em 0;
    box-shadow: var(--shadow-md);
  }

  :global(.markdown-preview hr) {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: 2em 0;
  }

  :global(.markdown-preview table) {
    width: 100%;
    border-collapse: collapse;
    margin: 1.2em 0;
    font-size: 0.9em;
  }

  :global(.markdown-preview th) {
    padding: 8px 12px;
    border-bottom: 2px solid var(--border-normal);
    text-align: left;
    font-weight: 600;
    color: var(--text-primary);
  }

  :global(.markdown-preview td) {
    padding: 7px 12px;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-secondary);
  }

  /* Scrollbar */
  .scrollbar-thin::-webkit-scrollbar { width: 5px; height: 5px; }
  .scrollbar-thin::-webkit-scrollbar-track { background: transparent; }
  .scrollbar-thin::-webkit-scrollbar-thumb {
    background: var(--scrollbar-thumb);
    border-radius: var(--radius-pill);
  }
  .scrollbar-thin::-webkit-scrollbar-thumb:hover { background: var(--scrollbar-thumb-hover); }
</style>
