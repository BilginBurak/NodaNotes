import SwiftUI
import AppKit

// MARK: - MarkdownToolbar
// Floating formatting toolbar for Live Preview and Source modes.

struct MarkdownToolbar: View {

    let editorMode: EditorMode
    @State private var hoveredAction: String? = nil

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 2) {
                // Text formatting group
                group {
                    toolbarButton(icon: "bold",          label: "Bold",          shortcut: "⌘B") { wrap("**", "**") }
                    toolbarButton(icon: "italic",        label: "Italic",        shortcut: "⌘I") { wrap("*",  "*")  }
                    toolbarButton(icon: "strikethrough", label: "Strikethrough", shortcut: "")   { wrap("~~", "~~") }
                    toolbarButton(icon: "chevron.left.forwardslash.chevron.right",
                                                         label: "Inline Code",   shortcut: "⌘`") { wrap("`",  "`")  }
                }

                divider

                // Heading group
                group {
                    toolbarButton(icon: "textformat.size.larger", label: "H1", shortcut: "") { insertLinePrefix("# ") }
                    toolbarButton(icon: "textformat.size",        label: "H2", shortcut: "") { insertLinePrefix("## ") }
                    toolbarButton(icon: "textformat.size.smaller",label: "H3", shortcut: "") { insertLinePrefix("### ") }
                }

                divider

                // List group
                group {
                    toolbarButton(icon: "list.bullet",   label: "Bullet List",   shortcut: "") { insertLinePrefix("- ") }
                    toolbarButton(icon: "list.number",   label: "Numbered List", shortcut: "") { insertLinePrefix("1. ") }
                    toolbarButton(icon: "checklist",     label: "Task List",     shortcut: "") { insertLinePrefix("- [ ] ") }
                    toolbarButton(icon: "text.quote",    label: "Blockquote",    shortcut: "") { insertLinePrefix("> ") }
                }

                divider

                // Insert group
                group {
                    toolbarButton(icon: "link",          label: "Link",          shortcut: "⌘K") { insertLink() }
                    toolbarButton(icon: "minus",         label: "Divider",       shortcut: "")   { insertText("\n---\n") }
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
        }
        .frame(height: 34)
        .background(Color(NSColor.windowBackgroundColor))
    }

    // MARK: - Helpers

    private func toolbarButton(icon: String, label: String, shortcut: String,
                                action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .medium))
                .frame(width: 28, height: 26)
                .background(
                    hoveredAction == icon
                        ? Color(NSColor.controlBackgroundColor)
                        : Color.clear
                )
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(shortcut.isEmpty ? label : "\(label)  \(shortcut)")
        .onHover { inside in hoveredAction = inside ? icon : nil }
    }

    @ViewBuilder
    private func group<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        HStack(spacing: 0) { content() }
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(Color(NSColor.controlBackgroundColor))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .strokeBorder(Color(NSColor.separatorColor).opacity(0.5), lineWidth: 0.5)
            )
    }

    private var divider: some View {
        Rectangle()
            .fill(Color(NSColor.separatorColor).opacity(0.4))
            .frame(width: 1, height: 20)
            .padding(.horizontal, 4)
    }

    // MARK: - Text Insertion via NSTextView

    private func currentTextView() -> NSTextView? {
        NSApplication.shared.keyWindow?
            .contentView?
            .findSubview(ofType: NSTextView.self)
    }

    private func wrap(_ open: String, _ close: String) {
        guard let tv = currentTextView() else { return }
        let sel = tv.selectedRange()
        let str = (tv.string as NSString)
        if sel.length > 0 {
            let selected = str.substring(with: sel)
            let replacement = "\(open)\(selected)\(close)"
            tv.insertText(replacement, replacementRange: sel)
        } else {
            tv.insertText("\(open)\(close)", replacementRange: sel)
            // Move cursor between markers
            let newPos = NSRange(location: sel.location + open.count, length: 0)
            tv.setSelectedRange(newPos)
        }
    }

    private func insertLinePrefix(_ prefix: String) {
        guard let tv = currentTextView() else { return }
        let sel = tv.selectedRange()
        let ns  = tv.string as NSString
        let lineRange = ns.lineRange(for: sel)
        let lineText  = ns.substring(with: lineRange)
        if lineText.hasPrefix(prefix) {
            // Toggle off
            let stripped = String(lineText.dropFirst(prefix.count))
            tv.insertText(stripped, replacementRange: lineRange)
        } else {
            tv.insertText(prefix + lineText, replacementRange: lineRange)
        }
    }

    private func insertText(_ text: String) {
        guard let tv = currentTextView() else { return }
        tv.insertText(text, replacementRange: tv.selectedRange())
    }

    private func insertLink() {
        guard let tv = currentTextView() else { return }
        let sel = tv.selectedRange()
        let selected = sel.length > 0 ? (tv.string as NSString).substring(with: sel) : "Link Text"
        let replacement = "[\(selected)](url)"
        tv.insertText(replacement, replacementRange: sel)
    }
}

// MARK: - NSView extension to find subview

extension NSView {
    func findSubview<T: NSView>(ofType type: T.Type) -> T? {
        if let match = self as? T { return match }
        for sub in subviews {
            if let found = sub.findSubview(ofType: type) { return found }
        }
        return nil
    }
}
