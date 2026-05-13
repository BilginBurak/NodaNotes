import AppKit
import SwiftUI

// MARK: - RawMarkdownEditor

struct RawMarkdownEditor: NSViewRepresentable {

    let note: Note
    @Binding var isDirty: Bool
    var onSave: (String) -> Void

    // MARK: - NSViewRepresentable

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSTextView.scrollableTextView()
        guard let textView = scrollView.documentView as? NSTextView else { return scrollView }

        textView.delegate = context.coordinator
        textView.isEditable = true
        textView.isRichText = false
        textView.allowsUndo = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = false
        textView.font = editorFont
        textView.textContainerInset = NSSize(width: 8, height: 12)
        textView.string = note.content

        // Register for ⌘S
        context.coordinator.textView = textView

        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView else { return }
        // Only update if content changed externally (not from user typing)
        if textView.string != note.content && !context.coordinator.isEditing {
            let selected = textView.selectedRanges
            textView.string = note.content
            textView.selectedRanges = selected
            MarkdownHighlighter.highlight(textView.textStorage!, font: editorFont)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(isDirty: $isDirty, onSave: onSave)
    }

    // MARK: - Font

    private var editorFont: NSFont {
        NSFont(name: "SF Mono", size: 14)
            ?? NSFont(name: "Menlo", size: 14)
            ?? NSFont.monospacedSystemFont(ofSize: 14, weight: .regular)
    }

    // MARK: - Coordinator

    final class Coordinator: NSObject, NSTextViewDelegate {

        @Binding var isDirty: Bool
        var onSave: (String) -> Void
        var isEditing = false
        weak var textView: NSTextView?
        private var debounceTask: Task<Void, Never>?

        init(isDirty: Binding<Bool>, onSave: @escaping (String) -> Void) {
            _isDirty = isDirty
            self.onSave = onSave
        }

        // MARK: - NSTextViewDelegate

        func textDidChange(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            isEditing = true
            isDirty = true

            // Syntax highlight incrementally
            MarkdownHighlighter.highlight(tv.textStorage!, font: editorFont)

            // 2-second debounce autosave
            debounceTask?.cancel()
            let content = tv.string
            debounceTask = Task { @MainActor [weak self] in
                try? await Task.sleep(for: .seconds(2))
                guard !Task.isCancelled else { return }
                self?.onSave(content)
                self?.isEditing = false
            }
        }

        func textView(_ textView: NSTextView, doCommandBy commandSelector: Selector) -> Bool {
            // ⌘S manual save
            if commandSelector == #selector(NSResponder.insertNewline(_:)) { return false }
            return false
        }

        private var editorFont: NSFont {
            NSFont(name: "SF Mono", size: 14)
                ?? NSFont(name: "Menlo", size: 14)
                ?? NSFont.monospacedSystemFont(ofSize: 14, weight: .regular)
        }
    }
}

// MARK: - MarkdownHighlighter

enum MarkdownHighlighter {

    nonisolated(unsafe) static let headingPattern    = try! NSRegularExpression(pattern: #"^#{1,6} .+"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let boldPattern       = try! NSRegularExpression(pattern: #"\*\*[^*]+\*\*"#)
    nonisolated(unsafe) static let italicPattern     = try! NSRegularExpression(pattern: #"(?<!\*)\*(?!\*)[^*]+\*(?!\*)"#)
    nonisolated(unsafe) static let codePattern       = try! NSRegularExpression(pattern: #"`[^`]+`"#)
    nonisolated(unsafe) static let linkPattern       = try! NSRegularExpression(pattern: #"\[.+?\]\(.+?\)"#)
    nonisolated(unsafe) static let blockquotePattern = try! NSRegularExpression(pattern: #"^> .+"#, options: .anchorsMatchLines)

    static func highlight(_ storage: NSTextStorage, font: NSFont) {
        let text = storage.string
        guard !text.isEmpty else { return }
        
        let fullRange = NSRange(location: 0, length: storage.length)

        storage.beginEditing()

        // Reset to default
        storage.setAttributes([
            .font: font,
            .foregroundColor: NSColor.labelColor
        ], range: fullRange)
        storage.removeAttribute(.backgroundColor, range: fullRange)

        let patterns: [(NSRegularExpression, (NSRange) -> Void)] = [
            (headingPattern, { range in
                let size = font.pointSize + 2
                storage.addAttribute(.font, value: NSFont.boldSystemFont(ofSize: size), range: range)
            }),
            (boldPattern, { range in
                storage.addAttribute(.font, value: NSFont.boldSystemFont(ofSize: font.pointSize), range: range)
            }),
            (italicPattern, { range in
                let italic = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask)
                storage.addAttribute(.font, value: italic, range: range)
            }),
            (codePattern, { range in
                storage.addAttribute(.backgroundColor, value: NSColor.quaternaryLabelColor, range: range)
            }),
            (linkPattern, { range in
                storage.addAttribute(.foregroundColor, value: NSColor.linkColor, range: range)
            }),
            (blockquotePattern, { range in
                storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: range)
                let italic = NSFontManager.shared.convert(font, toHaveTrait: .italicFontMask)
                storage.addAttribute(.font, value: italic, range: range)
            })
        ]

        for (pattern, apply) in patterns {
            pattern.enumerateMatches(in: text, range: fullRange) { match, _, _ in
                if let match { apply(match.range) }
            }
        }

        storage.endEditing()
    }
}
