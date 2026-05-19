import AppKit
import SwiftUI

// MARK: - RawMarkdownEditor (Live Preview)
// Obsidian-style: markers collapsed by default, revealed only on the cursor's line.
struct RawMarkdownEditor: NSViewRepresentable {

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
        tv.isAutomaticTextCompletionEnabled = false
        tv.font = LiveTheme.bodyFont
        tv.textColor = LiveTheme.body
        tv.backgroundColor = NSColor.textBackgroundColor
        tv.textContainerInset = NSSize(width: 48, height: 24)
        tv.string = note.content

        context.coordinator.textView   = tv
        context.coordinator.currentNoteID = note.id

        // Immediately render collapsed — no raw flicker on open
        LiveHighlighter.applyFull(storage: tv.textStorage!, activeLine: nil)
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let tv = scrollView.documentView as? NSTextView else { return }
        let idChanged = context.coordinator.currentNoteID != note.id
        if idChanged || (tv.string != note.content && !context.coordinator.isEditing) {
            context.coordinator.isInternalUpdate = true
            context.coordinator.currentNoteID = note.id
            context.coordinator.isEditing = false
            context.coordinator.lastActiveLine = nil
            tv.string = note.content
            LiveHighlighter.applyFull(storage: tv.textStorage!, activeLine: nil)
            context.coordinator.isInternalUpdate = false
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator(isDirty: $isDirty, onSave: onSave) }

    // MARK: - Coordinator
    final class Coordinator: NSObject, NSTextViewDelegate {
        @Binding var isDirty: Bool
        var onSave: (String) -> Void
        var isEditing = false
        var isInternalUpdate = false
        var currentNoteID: UUID?
        var lastActiveLine: NSRange?
        weak var textView: NSTextView?
        private var debounce: Task<Void, Never>?

        init(isDirty: Binding<Bool>, onSave: @escaping (String) -> Void) {
            _isDirty = isDirty; self.onSave = onSave
        }

        func textDidChange(_ notification: Notification) {
            guard !isInternalUpdate, let tv = notification.object as? NSTextView else { return }
            isEditing = true; isDirty = true
            let active = lineRange(in: tv)
            LiveHighlighter.applyFull(storage: tv.textStorage!, activeLine: active)
            lastActiveLine = active
            debounce?.cancel()
            let content = tv.string
            debounce = Task { @MainActor [weak self] in
                try? await Task.sleep(for: .seconds(2))
                guard !Task.isCancelled else { return }
                self?.onSave(content); self?.isEditing = false
            }
        }

        // KEY: incremental per-line update on cursor move
        func textViewDidChangeSelection(_ notification: Notification) {
            guard !isInternalUpdate, let tv = notification.object as? NSTextView else { return }
            let newLine = lineRange(in: tv)
            guard newLine != lastActiveLine else { return }

            let storage = tv.textStorage!
            // Collapse previous line, reveal new line — only those two lines touched
            if let old = lastActiveLine {
                LiveHighlighter.applyLineRange(storage: storage,
                                               text: tv.string as NSString,
                                               lineRange: old,
                                               isActive: false)
            }
            if let new = newLine {
                LiveHighlighter.applyLineRange(storage: storage,
                                               text: tv.string as NSString,
                                               lineRange: new,
                                               isActive: true)
            }
            lastActiveLine = newLine
        }

        func textView(_ textView: NSTextView, doCommandBy sel: Selector) -> Bool { false }

        func textDidEndEditing(_ notification: Notification) {
            guard let tv = notification.object as? NSTextView else { return }
            debounce?.cancel(); onSave(tv.string); isEditing = false
        }

        private func lineRange(in tv: NSTextView) -> NSRange? {
            let s = tv.selectedRange()
            guard s.location != NSNotFound, !tv.string.isEmpty else { return nil }
            let len = (tv.string as NSString).length
            guard s.location <= len else { return nil }
            return (tv.string as NSString).lineRange(for: s)
        }
    }
}

// MARK: - LiveTheme

enum LiveTheme {
    static let bodyFont     = NSFont.systemFont(ofSize: 15, weight: .regular)
    static let monoFont     = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
    static let body         = NSColor.labelColor
    static let lineHeight: CGFloat = 26

    static func headingFont(_ level: Int) -> NSFont {
        switch level {
        case 1: .boldSystemFont(ofSize: 28)
        case 2: .boldSystemFont(ofSize: 22)
        case 3: .boldSystemFont(ofSize: 18)
        case 4: .boldSystemFont(ofSize: 17)
        case 5: .boldSystemFont(ofSize: 16)
        default: .boldSystemFont(ofSize: 15)
        }
    }

    // Tiny invisible font used to zero-out marker width
    static let ghostFont = NSFont.systemFont(ofSize: 0.01)

    static var bodyPara: NSParagraphStyle {
        let s = NSMutableParagraphStyle()
        s.minimumLineHeight = lineHeight
        s.maximumLineHeight = lineHeight
        return s
    }
    static var headingPara: NSParagraphStyle {
        let s = NSMutableParagraphStyle()
        s.minimumLineHeight = lineHeight + 8
        s.maximumLineHeight = lineHeight + 8
        s.paragraphSpacingBefore = 8
        return s
    }
}

// MARK: - LiveHighlighter

enum LiveHighlighter {

    nonisolated(unsafe) static let headingRx    = try! NSRegularExpression(pattern: #"^(#{1,6}) (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let boldRx       = try! NSRegularExpression(pattern: #"\*\*(.+?)\*\*"#)
    nonisolated(unsafe) static let italicRx     = try! NSRegularExpression(pattern: #"(?<!\*)\*(?!\*)(.+?)\*(?!\*)"#)
    nonisolated(unsafe) static let codeBlockRx  = try! NSRegularExpression(pattern: #"```[\s\S]*?```"#, options: .dotMatchesLineSeparators)
    nonisolated(unsafe) static let inlineCodeRx = try! NSRegularExpression(pattern: #"`([^`\n]+)`"#)
    nonisolated(unsafe) static let linkRx       = try! NSRegularExpression(pattern: #"\[([^\]]+)\]\(([^\)]+)\)"#)
    nonisolated(unsafe) static let bqRx         = try! NSRegularExpression(pattern: #"^> .+"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let taskRx       = try! NSRegularExpression(pattern: #"^- \[([ xX])\] .+"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let strikeRx     = try! NSRegularExpression(pattern: #"~~(.+?)~~"#)

    // MARK: Full pass (on load / text change)
    static func applyFull(storage: NSTextStorage, activeLine: NSRange?) {
        let text = storage.string
        guard !text.isEmpty else { return }
        let full = NSRange(location: 0, length: storage.length)

        storage.beginEditing()
        // Reset
        storage.setAttributes([
            .font: LiveTheme.bodyFont,
            .foregroundColor: LiveTheme.body,
            .paragraphStyle: LiveTheme.bodyPara
        ], range: full)
        storage.removeAttribute(.backgroundColor, range: full)
        storage.removeAttribute(.underlineStyle, range: full)
        storage.removeAttribute(.strikethroughStyle, range: full)
        storage.removeAttribute(.kern, range: full)

        applyRules(storage: storage, text: text as NSString, scanRange: full, activeLine: activeLine)
        storage.endEditing()
    }

    // MARK: Single-line incremental pass
    static func applyLineRange(storage: NSTextStorage, text: NSString, lineRange: NSRange, isActive: Bool) {
        guard lineRange.location + lineRange.length <= storage.length else { return }
        storage.beginEditing()
        // Reset this line only
        storage.setAttributes([
            .font: LiveTheme.bodyFont,
            .foregroundColor: LiveTheme.body,
            .paragraphStyle: LiveTheme.bodyPara
        ], range: lineRange)
        storage.removeAttribute(.backgroundColor, range: lineRange)
        storage.removeAttribute(.underlineStyle, range: lineRange)
        storage.removeAttribute(.kern, range: lineRange)
        storage.removeAttribute(.strikethroughStyle, range: lineRange)

        applyRules(storage: storage, text: text, scanRange: lineRange, activeLine: isActive ? lineRange : nil)
        storage.endEditing()
    }

    // MARK: Core rules engine
    private static func applyRules(storage: NSTextStorage, text: NSString,
                                    scanRange: NSRange, activeLine: NSRange?) {
        let str = text as String

        // FIX: nil activeLine → collapse ALL markers (reading-mode look)
        func onActiveLine(_ range: NSRange) -> Bool {
            guard let al = activeLine else { return false }
            return NSIntersectionRange(al, range).length > 0
        }

        func collapseMarker(_ r: NSRange, inMatch match: NSRange) {
            guard r.location + r.length <= storage.length, r.length > 0 else { return }
            if onActiveLine(match) {
                // Active line: show marker dimmed
                storage.addAttribute(.font, value: LiveTheme.bodyFont, range: r)
                storage.addAttribute(.foregroundColor, value: NSColor.tertiaryLabelColor, range: r)
                storage.removeAttribute(.kern, range: r)
            } else {
                // Inactive: truly zero-width
                storage.addAttribute(.font, value: LiveTheme.ghostFont, range: r)
                storage.addAttribute(.foregroundColor, value: NSColor.clear, range: r)
                let kern = -LiveTheme.bodyFont.pointSize * 0.9
                storage.addAttribute(.kern, value: kern, range: r)
            }
        }

        // Code blocks first (protect inner content)
        var protected: [NSRange] = []
        codeBlockRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let r = m?.range else { return }
            storage.addAttribute(.font, value: LiveTheme.monoFont, range: r)
            storage.addAttribute(.foregroundColor, value: NSColor.systemOrange, range: r)
            storage.addAttribute(.backgroundColor, value: NSColor(white: 0.5, alpha: 0.07), range: r)
            protected.append(r)
        }

        func isProtected(_ r: NSRange) -> Bool {
            protected.contains { NSIntersectionRange($0, r).length > 0 }
        }

        // Headings
        headingRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 3 else { return }
            let full = m.range; let markerR = m.range(at: 1); let titleR = m.range(at: 2)
            guard titleR.location != NSNotFound, !isProtected(full) else { return }
            let level = markerR.length
            storage.addAttribute(.font, value: LiveTheme.headingFont(level), range: titleR)
            storage.addAttribute(.paragraphStyle, value: LiveTheme.headingPara, range: full)
            let markerAndSpace = NSRange(location: markerR.location, length: markerR.length + 1)
            collapseMarker(markerAndSpace, inMatch: full)
        }

        // Bold
        boldRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let full = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(full) else { return }
            storage.addAttribute(.font, value: NSFont.boldSystemFont(ofSize: LiveTheme.bodyFont.pointSize), range: contentR)
            collapseMarker(NSRange(location: full.location, length: 2), inMatch: full)
            collapseMarker(NSRange(location: full.location + full.length - 2, length: 2), inMatch: full)
        }

        // Italic
        italicRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let full = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(full) else { return }
            let italic = NSFontManager.shared.convert(LiveTheme.bodyFont, toHaveTrait: .italicFontMask)
            storage.addAttribute(.font, value: italic, range: contentR)
            collapseMarker(NSRange(location: full.location, length: 1), inMatch: full)
            collapseMarker(NSRange(location: full.location + full.length - 1, length: 1), inMatch: full)
        }

        // Strikethrough
        strikeRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let full = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(full) else { return }
            storage.addAttribute(.strikethroughStyle, value: NSUnderlineStyle.single.rawValue, range: contentR)
            storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: contentR)
            collapseMarker(NSRange(location: full.location, length: 2), inMatch: full)
            collapseMarker(NSRange(location: full.location + full.length - 2, length: 2), inMatch: full)
        }

        // Inline code
        inlineCodeRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let full = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(full) else { return }
            storage.addAttribute(.font, value: LiveTheme.monoFont, range: contentR)
            storage.addAttribute(.foregroundColor, value: NSColor.systemOrange, range: contentR)
            storage.addAttribute(.backgroundColor, value: NSColor(white: 0.5, alpha: 0.08), range: contentR)
            collapseMarker(NSRange(location: full.location, length: 1), inMatch: full)
            collapseMarker(NSRange(location: full.location + full.length - 1, length: 1), inMatch: full)
        }

        // Links
        linkRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 3 else { return }
            let full = m.range; let labelR = m.range(at: 1); let urlR = m.range(at: 2)
            guard labelR.location != NSNotFound, urlR.location != NSNotFound, !isProtected(full) else { return }
            storage.addAttribute(.foregroundColor, value: NSColor.linkColor, range: labelR)
            storage.addAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, range: labelR)
            // Collapse [, ] and (url)
            let openB  = NSRange(location: full.location, length: 1)
            let closeB = NSRange(location: labelR.location + labelR.length, length: 1)
            let paren  = NSRange(location: closeB.location + 1,
                                 length: full.location + full.length - (closeB.location + 1))
            for r in [openB, closeB, paren] { collapseMarker(r, inMatch: full) }
        }

        // Blockquotes
        bqRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let r = m?.range, !isProtected(r) else { return }
            let italic = NSFontManager.shared.convert(LiveTheme.bodyFont, toHaveTrait: .italicFontMask)
            storage.addAttribute(.font, value: italic, range: r)
            storage.addAttribute(.foregroundColor, value: NSColor.systemBlue.withAlphaComponent(0.8), range: r)
            collapseMarker(NSRange(location: r.location, length: min(2, r.length)), inMatch: r)
        }

        // Task lists
        taskRx.enumerateMatches(in: str, range: scanRange) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let full = m.range; let checkboxR = m.range(at: 1)
            guard checkboxR.location != NSNotFound else { return }
            let checkStr = (str as NSString).substring(with: checkboxR)
            let isDone = checkStr.lowercased() == "x"
            // Replace "- [x] " or "- [ ] " prefix with checkbox symbol
            let prefixRange = NSRange(location: full.location, length: checkboxR.location + checkboxR.length + 2 - full.location)
            let symbol = isDone ? "☑ " : "☐ "
            if onActiveLine(full) {
                storage.addAttribute(.foregroundColor, value: NSColor.tertiaryLabelColor, range: prefixRange)
            } else {
                storage.addAttribute(.font, value: LiveTheme.ghostFont, range: prefixRange)
                storage.addAttribute(.foregroundColor, value: NSColor.clear, range: prefixRange)
                // Insert checkbox as attachment-like via kern trick isn't ideal; use replacement
                // We'll just show the raw prefix dimmed on inactive lines for stability
                storage.addAttribute(.font, value: LiveTheme.bodyFont, range: prefixRange)
                storage.addAttribute(.foregroundColor, value: NSColor.tertiaryLabelColor, range: prefixRange)
            }
            if isDone {
                let contentStart = NSRange(location: prefixRange.location + prefixRange.length,
                                           length: full.location + full.length - (prefixRange.location + prefixRange.length))
                if contentStart.length > 0 {
                    storage.addAttribute(.strikethroughStyle, value: NSUnderlineStyle.single.rawValue, range: contentStart)
                    storage.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: contentStart)
                }
            }
        }
    }
}
