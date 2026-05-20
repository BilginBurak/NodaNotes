<script lang="ts">
  import { marked } from 'marked';

  export let content: string = '';

  // Simple customization of marked for safety / styling
  $: html = marked.parse(content) as string;
</script>

<div class="markdown-preview scrollbar-thin">
  {@html html}
</div>

<style>
  .markdown-preview {
    padding: 24px 30px;
    height: 100%;
    overflow-y: auto;
    color: #cbd5e1;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    font-size: 15px;
    line-height: 1.6;
    background-color: #0d1117;
  }

  :global(.markdown-preview h1) {
    font-size: 1.8rem;
    font-weight: 700;
    color: #f8fafc;
    margin-top: 1.5em;
    margin-bottom: 0.5em;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 0.3em;
  }

  :global(.markdown-preview h2) {
    font-size: 1.4rem;
    font-weight: 600;
    color: #f8fafc;
    margin-top: 1.4em;
    margin-bottom: 0.5em;
  }

  :global(.markdown-preview h3) {
    font-size: 1.15rem;
    font-weight: 600;
    color: #f8fafc;
    margin-top: 1.3em;
    margin-bottom: 0.5em;
  }

  :global(.markdown-preview p) {
    margin-top: 0;
    margin-bottom: 1em;
  }

  :global(.markdown-preview strong) {
    color: #f8fafc;
    font-weight: 600;
  }

  :global(.markdown-preview em) {
    color: #e2e8f0;
    font-style: italic;
  }

  :global(.markdown-preview a) {
    color: #38bdf8;
    text-decoration: none;
    transition: color 0.15s ease;
  }

  :global(.markdown-preview a:hover) {
    color: #7dd3fc;
    text-decoration: underline;
  }

  :global(.markdown-preview code) {
    background-color: rgba(255, 255, 255, 0.06);
    padding: 0.2em 0.4em;
    border-radius: 4px;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.9em;
    color: #f472b6;
  }

  :global(.markdown-preview pre) {
    background-color: #07090e;
    padding: 16px;
    border-radius: 8px;
    overflow-x: auto;
    border: 1px solid rgba(255, 255, 255, 0.05);
    margin: 1.2em 0;
  }

  :global(.markdown-preview pre code) {
    background-color: transparent;
    padding: 0;
    border-radius: 0;
    color: #e2e8f0;
    font-size: 13.5px;
  }

  :global(.markdown-preview ul, .markdown-preview ol) {
    padding-left: 20px;
    margin-bottom: 1em;
  }

  :global(.markdown-preview li) {
    margin-bottom: 0.4em;
  }

  :global(.markdown-preview blockquote) {
    margin: 1.2em 0;
    padding: 0 16px;
    color: #94a3b8;
    border-left: 4px solid #6366f1;
    background-color: rgba(99, 102, 241, 0.03);
    border-top-right-radius: 4px;
    border-bottom-right-radius: 4px;
  }

  :global(.markdown-preview img) {
    max-width: 100%;
    border-radius: 8px;
    margin: 1.5em 0;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  }

  /* Sleek modern scrollbar */
  .scrollbar-thin::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }

  .scrollbar-thin::-webkit-scrollbar-track {
    background: transparent;
  }

  .scrollbar-thin::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
  }

  .scrollbar-thin::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.2);
  }
</style>
