import { EditorView, keymap, highlightActiveLine, drawSelection, dropCursor } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { history, defaultKeymap, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, indentOnInput, syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
import { search, searchKeymap } from '@codemirror/search';

/**
 * macOS native editor teması — Apple'ın Xcode Dark + iA Writer esinlenmiş
 *
 * Gelecekte light ve custom tema desteği eklemek için:
 *   export function getLightEditorExtensions(...) { ... }
 *   veya tema parametresi geçilebilir:
 *   export function getEditorExtensions(onDocChange, theme: 'dark' | 'light' = 'dark')
 */
const macOSDarkTheme = EditorView.theme({
  '&': {
    color: 'rgba(255, 255, 255, 0.88)',
    backgroundColor: '#1c1c1e',
    fontFamily: '"SF Mono", "Fira Code", "JetBrains Mono", Menlo, Monaco, monospace',
    fontSize: '14px',
    height: '100%',
  },
  '.cm-content': {
    caretColor: '#0a84ff',
    /* iA Writer benzeri geniş kenar boşlukları — odak hissi */
    padding: '32px 60px',
    maxWidth: '800px',
    margin: '0 auto',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: '#0a84ff',
    borderLeftWidth: '2px',
  },
  /* macOS mavi seçim rengi */
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'rgba(10, 132, 255, 0.30) !important',
  },
  '.cm-gutters': {
    backgroundColor: '#1c1c1e',
    color: 'rgba(255, 255, 255, 0.20)',
    borderRight: 'none',
    userSelect: 'none',
    paddingLeft: '8px',
    minWidth: '48px',
  },
  '.cm-gutterElement': {
    padding: '0 8px 0 0',
    fontSize: '12px',
  },
  /* Aktif satır — çok hafif vurgu, dikkat dağıtmayan */
  '.cm-activeLine': {
    backgroundColor: 'rgba(255, 255, 255, 0.025)',
  },
  '.cm-activeLineGutter': {
    color: 'rgba(255, 255, 255, 0.55)',
    backgroundColor: 'rgba(255, 255, 255, 0.025)',
  },
  '.cm-matchingBracket': {
    backgroundColor: 'rgba(255, 255, 255, 0.10)',
    color: '#64d2ff',
  },
  '.cm-nonmatchingBracket': {
    backgroundColor: 'rgba(255, 69, 58, 0.20)',
    color: '#ff453a',
  },
  /* Markdown sözdizim stilleri */
  '.cm-heading': {
    color: 'rgba(255, 255, 255, 0.88)',
    fontWeight: '700',
  },
  '.cm-strong': {
    color: 'rgba(255, 255, 255, 0.95)',
    fontWeight: '700',
  },
  '.cm-emphasis': {
    color: 'rgba(255, 255, 255, 0.80)',
    fontStyle: 'italic',
  },
  '.cm-link': {
    color: '#0a84ff',
    textDecoration: 'underline',
  },
  '.cm-url': {
    color: 'rgba(255, 255, 255, 0.35)',
  },
  '.cm-code': {
    color: '#64d2ff',
    fontFamily: '"SF Mono", "Fira Code", Menlo, monospace',
  },
  /* Search highlight */
  '.cm-searchMatch': {
    backgroundColor: 'rgba(255, 214, 10, 0.25)',
    outline: '1px solid rgba(255, 214, 10, 0.5)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: 'rgba(10, 132, 255, 0.40)',
  },
}, { dark: true });

/**
 * Editor extension koleksiyonu.
 * onDocChange: içerik değiştiğinde çağrılacak callback
 */
export function getEditorExtensions(onDocChange: (val: string) => void) {
  return [
    macOSDarkTheme,
    highlightActiveLine(),
    drawSelection(),
    dropCursor(),
    history(),
    bracketMatching(),
    indentOnInput(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    markdown(),
    search({ top: true }),

    // Değişiklik dinleyici — store'a ve otomatik kayıda bildir
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        onDocChange(update.state.doc.toString());
      }
    }),

    // Klavye kısayolları
    keymap.of([
      ...defaultKeymap,
      ...historyKeymap,
      ...searchKeymap,
      indentWithTab,
    ]),

    EditorState.tabSize.of(4),
  ];
}
