import { EditorView, keymap, highlightActiveLine, drawSelection, dropCursor } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { history, defaultKeymap, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, indentOnInput, syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
import { search, searchKeymap } from '@codemirror/search';

/**
 * Modern premium dark theme configuration for CodeMirror 6 editor
 */
const customTheme = EditorView.theme({
  '&': {
    color: '#e2e8f0',
    backgroundColor: '#0a0d14',
    fontFamily: '"SF Mono", "Fira Code", Menlo, Monaco, Consolas, monospace',
    fontSize: '14px',
    height: '100%',
  },
  '.cm-content': {
    caretColor: '#6366f1',
    padding: '24px 20px',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: '#6366f1',
    borderLeftWidth: '2px',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'rgba(99, 102, 241, 0.25) !important',
  },
  '.cm-gutters': {
    backgroundColor: '#0a0d14',
    color: '#475569',
    borderRight: 'none',
    userSelect: 'none',
    paddingLeft: '12px',
  },
  '.cm-gutterElement': {
    padding: '0 8px 0 0',
  },
  '.cm-activeLine': {
    backgroundColor: 'rgba(255, 255, 255, 0.015)',
  },
  '.cm-activeLineGutter': {
    color: '#f8fafc',
    backgroundColor: 'rgba(255, 255, 255, 0.015)',
  },
  '.cm-matchingBracket': {
    backgroundColor: 'rgba(255, 255, 255, 0.1)',
    color: '#38bdf8',
  },
  '.cm-nonmatchingBracket': {
    backgroundColor: 'rgba(239, 68, 68, 0.2)',
    color: '#ef4444',
  },
  // Markdown style modifications
  '.cm-heading': {
    color: '#818cf8',
    fontWeight: 'bold',
  },
  '.cm-strong': {
    color: '#f8fafc',
    fontWeight: 'bold',
  },
  '.cm-emphasis': {
    color: '#cbd5e1',
    fontStyle: 'italic',
  },
  '.cm-link': {
    color: '#38bdf8',
    textDecoration: 'underline',
  },
  '.cm-url': {
    color: '#64748b',
  },
}, { dark: true });

/**
 * Export default collection of editor extensions
 */
export function getEditorExtensions(onDocChange: (val: string) => void) {
  return [
    customTheme,
    highlightActiveLine(),
    drawSelection(),
    dropCursor(),
    history(),
    bracketMatching(),
    indentOnInput(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    markdown(),
    search({ top: true }),
    
    // Listen to changes to notify stores/auto-save
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        onDocChange(update.state.doc.toString());
      }
    }),

    // Keybindings
    keymap.of([
      ...defaultKeymap,
      ...historyKeymap,
      ...searchKeymap,
      indentWithTab
    ]),

    EditorState.tabSize.of(4),
  ];
}
