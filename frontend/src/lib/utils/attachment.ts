/**
 * Resolves attachment URL.
 * In Tauri desktop context, `noda://attachments/...` is natively resolved via a custom protocol handler.
 * In a standard browser context (e.g. Safari), we rewrite it to `/attachments/...` with the daemon auth token.
 */
export function resolveAttachmentUrl(url: string): string {
  if (!url || !url.startsWith('noda://attachments/')) {
    return url;
  }

  const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
  if (isTauri) {
    return url;
  }

  // Pure browser context
  const name = url.replace('noda://attachments/', '');
  const token = typeof window !== 'undefined' ? (window as any).__NODA_TOKEN__ || '' : '';
  return `/attachments/${encodeURIComponent(name)}?token=${encodeURIComponent(token)}`;
}

/**
 * Rewrites any occurrences of noda://attachments/ inside raw HTML attributes (src/href).
 */
export function rewriteHtmlAttachments(html: string): string {
  if (!html) return html;

  const isTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__;
  if (isTauri) {
    return html;
  }

  const token = typeof window !== 'undefined' ? (window as any).__NODA_TOKEN__ || '' : '';

  return html.replace(
    /(src|href)="noda:\/\/attachments\/([^"]+)"/g,
    (_, attr, filename) => {
      // Decode if it's already encoded, then encode properly
      let decoded = filename;
      try {
        decoded = decodeURIComponent(filename);
      } catch {
        // ignore
      }
      return `${attr}="/attachments/${encodeURIComponent(decoded)}?token=${encodeURIComponent(token)}"`;
    }
  );
}
