import AppKit
import SwiftUI

// MARK: - SourceEditor
// Plain text editor — shows ALL markdown syntax as-is, basic color coding only.
struct SourceEditor: NSViewRepresentable {

    let note: Note
    @Binding var isDirty: Bool
    var onSave: (String) -> Void

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSTextView.scrollableTextView()
        guard let tv = scrollView.documentView as? NSTextView else { return scrollView }

        tv.delegate = context.coordinator
        tv.isEditable = true
        tv.isRichText = false
        tv.allowsUndo = true
        tv.isAutomaticQuoteSubstitutionEnabled = false
        tv.isAutomaticDashSubstitutionEnabled  = false
        tv.isAutomaticSpellingCorrectionEnabled = false
        tv.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        tv.textColor = NSColor.labelColor
        tv.backgroundColor = NSColor.textBackgroundColor
        tv.textContainerInset = NSSize(width: 24, height: 16)
        tv.string = note.content

        context.coordinator.textView   = tv
        context.coordinator.currentID = note.id
        SourceHighlighter.apply(to: tv.textStorage!)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let tv = scrollView.documentView as? NSTextView else { return }
        if context.coordinator.currentID != note.id ||
           (tv.string != note.content && !context.coordinator.isEditing) {
            context.coordinator.isInternalUpdate = true
            context.coordinator.currentID = note.id
            context.coordinator.isEditing = false
            tv.string = note.content
            SourceHighlighter.apply(to: tv.textStorage!)
            context.coordinator.isInternalUpdate = false
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator(isDirty: $isDirty, onSave: onSave) }

    final class Coordinator: NSObject, NSTextViewDelegate {
        @Binding var isDirty: Bool
        var onSave: (String) -> Void
        var isEditing = false
        var isInternalUpdate = false
        var currentID: UUID?
        weak var textView: NSTextView?
        private var debounce: Task<Void, Never>?

        init(isDirty: Binding<Bool>, onSave: @escaping (String) -> Void) {
            _isDirty = isDirty; self.onSave = onSave
        }

        func textDidChange(_ notification: Notification) {
            guard !isInternalUpdate, let tv = notification.object as? NSTextView else { return }
            isEditing = true; isDirty = true
            SourceHighlighter.apply(to: tv.textStorage!)
            debounce?.cancel()
            let content = tv.string
            debounce = Task { @MainActor [weak self] in
                try? await Task.sleep(for: .seconds(2))
                guard !Task.isCancelled else { return }
                self?.onSave(content); self?.isEditing = false
            }
        }
        func textDidEndEditing(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            debounce?.cancel(); onSave(tv.string); isEditing = false
        }
    }
}

// MARK: - SourceHighlighter (code-editor style, nothing hidden)
enum SourceHighlighter {
    nonisolated(unsafe) static let headingRx = try! NSRegularExpression(pattern: #"^#{1,6} .+"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let boldRx    = try! NSRegularExpression(pattern: #"\*\*.+?\*\*"#)
    nonisolated(unsafe) static let italicRx  = try! NSRegularExpression(pattern: #"(?<!\*)\*(?!\*).+?\*(?!\*)"#)
    nonisolated(unsafe) static let codeRx    = try! NSRegularExpression(pattern: #"`[^`]+`"#)
    nonisolated(unsafe) static let linkRx    = try! NSRegularExpression(pattern: #"\[.+?\]\(.+?\)"#)
    nonisolated(unsafe) static let bqRx      = try! NSRegularExpression(pattern: #"^> .+"#, options: .anchorsMatchLines)

    static func apply(to storage: NSTextStorage) {
        let text = storage.string
        guard !text.isEmpty else { return }
        let full = NSRange(location: 0, length: storage.length)
        let mono = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)

        storage.beginEditing()
        storage.setAttributes([.font: mono, .foregroundColor: NSColor.labelColor], range: full)

        headingRx.enumerateMatches(in: text, range: full) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.systemPurple, range: r)
            storage.addAttribute(.font, value: NSFont.monospacedSystemFont(ofSize: 13, weight: .bold), range: r)
        }
        boldRx.enumerateMatches(in: text, range: full) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.systemBlue, range: r)
        }
        italicRx.enumerateMatches(in: text, range: full) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.systemTeal, range: r)
        }
        codeRx.enumerateMatches(in: text, range: full) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.systemOrange, range: r)
        }
        linkRx.enumerateMatches(in: text, range: full) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.linkColor, range: r)
        }
        bqRx.enumerateMatches(in: text, range: full) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: r)
        }
        storage.endEditing()
    }
}
