import { EditorView, keymap, highlightActiveLine, drawSelection, dropCursor, Decoration, ViewPlugin, type DecorationSet, type ViewUpdate, WidgetType } from '@codemirror/view';
import { EditorState, RangeSetBuilder, Compartment } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { history, defaultKeymap, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, indentOnInput, syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
import { search, searchKeymap } from '@codemirror/search';
import { syntaxTree } from '@codemirror/language';

class CheckboxWidget extends WidgetType {
  constructor(readonly checked: boolean, readonly pos: number) {
    super();
  }

  eq(other: CheckboxWidget) {
    return this.checked === other.checked && this.pos === other.pos;
  }

  toDOM(view: EditorView) {
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = this.checked;
    input.className = 'cm-task-checkbox';

    input.addEventListener('click', (e) => {
      e.preventDefault();
      const from = this.pos;
      const to = this.pos + 3; // "[ ]" or "[x]" length is 3
      const current = view.state.sliceDoc(from, to);
      let replacement = '';
      if (current === '[ ]') {
        replacement = '[x]';
      } else if (current === '[x]' || current === '[X]') {
        replacement = '[ ]';
      } else {
        return;
      }

      view.dispatch({
        changes: { from, to, insert: replacement }
      });
    });

    return input;
  }

  ignoreEvent() {
    return false;
  }
}

export const livePreviewCompartment = new Compartment();
export const editorModeCompartment = new Compartment();

// Live Preview (WYSIWYG) dekorasyon eklentisi
const livePreviewPlugin = ViewPlugin.fromClass(class {
  decorations: DecorationSet;

  constructor(view: EditorView) {
    this.decorations = this.getDecorations(view);
  }

  update(update: ViewUpdate) {
    if (update.docChanged || update.selectionSet || update.viewportChanged) {
      this.decorations = this.getDecorations(update.view);
    }
  }

  getDecorations(view: EditorView): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    const state = view.state;

    // Aktif cursor satırlarını bul (destek: çoklu cursor)
    const cursorLines = new Set<number>();
    for (const range of state.selection.ranges) {
      const line = state.doc.lineAt(range.head).number;
      cursorLines.add(line);
    }

    const decos: { from: number; to: number; deco: Decoration; isLine?: boolean }[] = [];

    // Viewport satırlarında task checkbox'ları ara
    for (const { from, to } of view.visibleRanges) {
      let pos = from;
      while (pos < to) {
        const line = state.doc.lineAt(pos);
        const text = line.text;
        const match = text.match(/^(\s*[-*+]\s+)(\[([ xX])\])/);
        if (match) {
          const lineNum = line.number;
          if (!cursorLines.has(lineNum)) {
            const startMarkOffset = match[1].length;
            const bracketStart = line.from + startMarkOffset;
            const bracketEnd = bracketStart + match[2].length;
            const isChecked = match[3].toLowerCase() === 'x';

            decos.push({
              from: bracketStart,
              to: bracketEnd,
              deco: Decoration.replace({
                widget: new CheckboxWidget(isChecked, bracketStart)
              })
            });

            decos.push({
              from: line.from,
              to: bracketStart,
              deco: Decoration.mark({ class: 'cm-hidden-syntax' })
            });
          }
        }
        pos = line.to + 1;
      }
    }

    const isCursorInNode = (from: number, to: number) => {
      const startLine = state.doc.lineAt(from).number;
      const endLine = state.doc.lineAt(to).number;
      for (const cline of cursorLines) {
        if (cline >= startLine && cline <= endLine) return true;
      }
      return false;
    };

    for (const { from, to } of view.visibleRanges) {
      syntaxTree(state).iterate({
        from, to,
        enter(node) {
          const type = node.name;

          // Eğer cursor bu düğümün satırındaysa markdown sembollerini gizleme (raw edit)
          if (isCursorInNode(node.from, node.to)) {
            return;
          }

          // 1. Başlıklar
          if (type === 'HeaderMark') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-hidden-syntax' }) });
          } else if (type === 'ATXHeading1') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-h1' }) });
          } else if (type === 'ATXHeading2') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-h2' }) });
          } else if (type === 'ATXHeading3') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-h3' }) });
          } else if (type === 'ATXHeading4' || type === 'ATXHeading5' || type === 'ATXHeading6') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-h4' }) });
          }

          // 2. Kalın / İtalik
          else if (type === 'EmphasisMark') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-hidden-syntax' }) });
          } else if (type === 'StrongEmphasis') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-strong' }) });
          } else if (type === 'Emphasis') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-em' }) });
          }

          // 3. Bağlantılar (Links)
          else if (type === 'LinkMark' || type === 'URL') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-hidden-syntax' }) });
          } else if (type === 'Link') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-link-text' }) });
          }

          // 4. Inline Code
          else if (type === 'CodeMark') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-hidden-syntax' }) });
          } else if (type === 'InlineCode') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-code' }) });
          }

          // 5. BlockQuotes
          else if (type === 'QuoteMark') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-hidden-syntax' }) });
          } else if (type === 'BlockQuote') {
            const startLine = state.doc.lineAt(node.from).number;
            const endLine = state.doc.lineAt(node.to).number;
            for (let l = startLine; l <= endLine; l++) {
              if (!cursorLines.has(l)) {
                const lineInfo = state.doc.line(l);
                decos.push({
                  from: lineInfo.from,
                  to: lineInfo.from,
                  deco: Decoration.line({ class: 'cm-live-blockquote-line' }),
                  isLine: true
                });
              }
            }
          }

          // 6. Horizontal Rules
          else if (type === 'HorizontalRule') {
            decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-hidden-syntax' }) });
            const lineInfo = state.doc.lineAt(node.from);
            if (!cursorLines.has(lineInfo.number)) {
              decos.push({
                from: lineInfo.from,
                to: lineInfo.from,
                deco: Decoration.line({ class: 'cm-live-hr-line' }),
                isLine: true
              });
            }
          }

          // 7. List marks (Bullet & Number lists)
          else if (type === 'ListMark') {
            const text = state.sliceDoc(node.from, node.to).trim();
            if (text === '-' || text === '*' || text === '+') {
              decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-list-mark' }) });
            } else if (/^\d+\.?$/.test(text)) {
              decos.push({ from: node.from, to: node.to, deco: Decoration.mark({ class: 'cm-live-ordered-mark' }) });
            }
          }
        }
      });
    }

    // Sıralama (CodeMirror RangeSetBuilder gereksinimleri):
    // 1. from ASC
    // 2. Aynı from ise, Line dekorasyonu önce gelir
    // 3. Mark dekorasyonları nesting için to DESC (büyük aralık önce)
    decos.sort((a, b) => {
      if (a.from !== b.from) return a.from - b.from;
      const aIsLine = a.isLine ? 1 : 0;
      const bIsLine = b.isLine ? 1 : 0;
      if (aIsLine !== bIsLine) {
        return bIsLine - aIsLine; // Line önce
      }
      return b.to - a.to; // Nesting: Büyük önce
    });

    // Tekilleştirme
    const uniqueDecos: typeof decos = [];
    let lastKey = '';
    for (const item of decos) {
      const key = `${item.from}-${item.to}-${item.isLine ? 'L' : 'M'}`;
      if (key !== lastKey) {
        uniqueDecos.push(item);
        lastKey = key;
      }
    }

    for (const item of uniqueDecos) {
      try {
        builder.add(item.from, item.to, item.deco);
      } catch (e) {
        console.error('Failed to add decoration:', e, item);
      }
    }

    return builder.finish();
  }
}, {
  decorations: v => v.decorations
});

/**
 * macOS native editor teması — Apple'ın Xcode Dark + iA Writer esinlenmiş premium WYSIWYG düzeni
 */
const macOSDarkTheme = EditorView.theme({
  '&': {
    color: 'var(--text-primary)',
    backgroundColor: 'var(--bg-editor)',
    fontSize: '15px',
    height: '100%',
  },
  '&.cm-mode-live .cm-content': {
    fontFamily: 'var(--font-sans) !important',
  },
  '&.cm-mode-live .cm-line': {
    fontFamily: 'var(--font-sans) !important',
  },
  '&.cm-mode-edit .cm-content': {
    fontFamily: 'var(--font-mono) !important',
  },
  '&.cm-mode-edit .cm-line': {
    fontFamily: 'var(--font-mono) !important',
  },
  '.cm-content': {
    caretColor: 'var(--accent)',
    padding: '32px 60px',
    maxWidth: '900px',
    margin: '0 auto',
    lineHeight: '1.7',
  },
  '.cm-line': {
    padding: '4px 0',
  },
  '.cm-cursor, .cm-dropCursor': {
    borderLeftColor: 'var(--accent)',
    borderLeftWidth: '2px',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'rgba(10, 132, 255, 0.25) !important',
  },
  '.cm-gutters': {
    backgroundColor: 'var(--bg-editor)',
    color: 'var(--text-disabled)',
    borderRight: 'none',
    userSelect: 'none',
    paddingLeft: '8px',
    minWidth: '48px',
    opacity: 0.35,
  },
  '.cm-gutterElement': {
    padding: '0 8px 0 0',
    fontSize: '12px',
  },
  '.cm-activeLine': {
    backgroundColor: 'rgba(255, 255, 255, 0.02)',
  },
  '.cm-activeLineGutter': {
    color: 'var(--text-secondary)',
    backgroundColor: 'rgba(255, 255, 255, 0.02)',
  },
  '.cm-matchingBracket': {
    backgroundColor: 'rgba(255, 255, 255, 0.10)',
    color: '#64d2ff',
  },
  '.cm-nonmatchingBracket': {
    backgroundColor: 'rgba(255, 69, 58, 0.20)',
    color: '#ff453a',
  },
  '.cm-heading': {
    color: 'var(--text-primary)',
    fontWeight: '700',
    fontFamily: 'var(--font-sans)',
  },
  '.cm-strong': {
    color: 'var(--text-primary)',
    fontWeight: '700',
  },
  '.cm-emphasis': {
    color: 'var(--text-secondary)',
    fontStyle: 'italic',
  },
  '.cm-link': {
    color: 'var(--accent)',
    textDecoration: 'underline',
  },
  '.cm-url': {
    color: 'var(--text-disabled)',
  },
  '.cm-code': {
    color: 'var(--accent-hover)',
    fontFamily: 'var(--font-mono)',
  },
  '.cm-searchMatch': {
    backgroundColor: 'rgba(255, 214, 10, 0.25)',
    outline: '1px solid rgba(255, 214, 10, 0.5)',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: 'rgba(10, 132, 255, 0.40)',
  },

  /* Live Preview (WYSIWYG) CSS sınıfları - Reading View kalitesinde */
  '.cm-hidden-syntax': {
    display: 'none !important',
  },
  '.cm-live-h1': {
    fontSize: '1.75rem',
    fontWeight: '700',
    color: 'var(--text-primary)',
    display: 'inline-block',
    letterSpacing: '-0.3px',
    borderBottom: '1px solid var(--border-subtle)',
    paddingBottom: '0.4em',
    lineHeight: '1.2',
    width: '100%',
    margin: '0.8em 0 0.4em 0',
  },
  '.cm-live-h2': {
    fontSize: '1.3rem',
    fontWeight: '700',
    color: 'var(--text-primary)',
    display: 'inline-block',
    letterSpacing: '-0.2px',
    lineHeight: '1.3',
    margin: '1.2em 0 0.3em 0',
    width: '100%',
  },
  '.cm-live-h3': {
    fontSize: '1.1rem',
    fontWeight: '600',
    color: 'var(--text-primary)',
    display: 'inline-block',
    lineHeight: '1.4',
    margin: '1em 0 0.3em 0',
    width: '100%',
  },
  '.cm-live-h4': {
    fontSize: '1rem',
    fontWeight: '600',
    color: 'var(--text-secondary)',
    display: 'inline-block',
    lineHeight: '1.4',
    margin: '0.8em 0 0.3em 0',
    width: '100%',
  },
  '.cm-live-strong': {
    fontWeight: '700',
    color: 'var(--text-primary)',
  },
  '.cm-live-em': {
    fontStyle: 'italic',
    color: 'var(--text-secondary)',
  },
  '.cm-live-link-text': {
    color: 'var(--accent)',
    textDecoration: 'none',
    transition: 'color 0.15s ease',
  },
  '.cm-live-link-text:hover': {
    color: 'var(--accent-hover)',
    textDecoration: 'underline',
  },
  '.cm-live-code': {
    backgroundColor: 'var(--bg-control)',
    border: '1px solid var(--border-subtle)',
    padding: '0.15em 0.4em',
    borderRadius: '4px',
    fontFamily: 'var(--font-mono)',
    fontSize: '0.88em',
    color: 'var(--accent-hover)',
  },
  '.cm-live-blockquote-line': {
    borderLeft: '3px solid var(--accent) !important',
    backgroundColor: 'var(--accent-muted) !important',
    paddingLeft: '16px !important',
    borderRadius: '0 var(--radius-sm) var(--radius-sm) 0',
    margin: '4px 0 !important',
  },
  '.cm-live-hr-line': {
    borderBottom: '1px solid var(--border-subtle) !important',
    height: '0',
    margin: '1.5em 0 !important',
    display: 'block',
  },
  '.cm-live-list-mark': {
    color: 'transparent !important',
    position: 'relative',
    display: 'inline-block',
    width: '1em',
  },
  '.cm-live-list-mark::before': {
    content: '"•"',
    position: 'absolute',
    left: '0',
    top: '50%',
    transform: 'translateY(-50%)',
    color: 'var(--accent)',
    fontWeight: 'bold',
    fontSize: '1.3em',
  },
  '.cm-live-ordered-mark': {
    color: 'var(--accent) !important',
    fontWeight: '600',
    marginRight: '4px',
  },
  '.cm-task-checkbox': {
    appearance: 'none',
    width: '14px',
    height: '14px',
    border: '1px solid var(--border-strong)',
    borderRadius: '3px',
    backgroundColor: 'var(--bg-control)',
    display: 'inline-block',
    verticalAlign: 'middle',
    position: 'relative',
    cursor: 'pointer',
    marginRight: '6px',
    marginTop: '-2px',
    transition: 'all 0.1s ease',
  },
  '.cm-task-checkbox:checked': {
    backgroundColor: 'var(--accent)',
    borderColor: 'var(--accent)',
  },
  '.cm-task-checkbox:checked::after': {
    content: '"✓"',
    position: 'absolute',
    color: 'white',
    fontSize: '10px',
    fontWeight: 'bold',
    left: '50%',
    top: '50%',
    transform: 'translate(-50%, -50%)',
  },
  '.cm-task-checkbox:hover': {
    borderColor: 'var(--accent)',
    boxShadow: '0 0 0 2px var(--accent-muted)',
  }
}, { dark: true });

export function getEditorExtensions(onDocChange: (val: string) => void, viewMode: 'edit' | 'live' | 'preview') {
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

    // Live Preview dynamic compartment
    livePreviewCompartment.of(viewMode === 'live' ? [livePreviewPlugin] : []),
    editorModeCompartment.of(EditorView.editorAttributes.of({
      class: viewMode === 'live' ? 'cm-mode-live' : 'cm-mode-edit'
    })),
    EditorView.lineWrapping,

    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        onDocChange(update.state.doc.toString());
      }
    }),

    keymap.of([
      ...defaultKeymap,
      ...historyKeymap,
      ...searchKeymap,
      indentWithTab,
    ]),

    EditorState.tabSize.of(4),
  ];
}
export { livePreviewPlugin };

