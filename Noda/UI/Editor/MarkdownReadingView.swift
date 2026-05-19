import AppKit
import SwiftUI

// MARK: - MarkdownReadingView
// Full GFM render — markers completely hidden, reading-mode only.
struct MarkdownReadingView: NSViewRepresentable {

    let note: Note

    func makeNSView(context: Context) -> NSScrollView {
        let sv = NSTextView.scrollableTextView()
        guard let tv = sv.documentView as? NSTextView else { return sv }
        configure(tv)
        tv.textStorage?.setAttributedString(GFMRenderer.render(note.content))
        return sv
    }

    func updateNSView(_ sv: NSScrollView, context: Context) {
        guard let tv = sv.documentView as? NSTextView else { return }
        tv.textStorage?.setAttributedString(GFMRenderer.render(note.content))
    }

    private func configure(_ tv: NSTextView) {
        tv.isEditable = false
        tv.isSelectable = true
        tv.isRichText = true
        tv.backgroundColor = NSColor.textBackgroundColor
        tv.textContainerInset = NSSize(width: 64, height: 32)
    }
}

// MARK: - GFMRenderer

enum GFMRenderer {

    nonisolated(unsafe) static let headingRx    = try! NSRegularExpression(pattern: #"^(#{1,6}) (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let boldRx       = try! NSRegularExpression(pattern: #"\*\*(.+?)\*\*"#)
    nonisolated(unsafe) static let italicRx     = try! NSRegularExpression(pattern: #"(?<!\*)\*(?!\*)(.+?)\*(?!\*)"#)
    nonisolated(unsafe) static let strikeRx     = try! NSRegularExpression(pattern: #"~~(.+?)~~"#)
    nonisolated(unsafe) static let codeBlockRx  = try! NSRegularExpression(pattern: #"```(?:\w*\n)?([\s\S]*?)```"#, options: .dotMatchesLineSeparators)
    nonisolated(unsafe) static let inlineCodeRx = try! NSRegularExpression(pattern: #"`([^`\n]+)`"#)
    nonisolated(unsafe) static let linkRx       = try! NSRegularExpression(pattern: #"\[([^\]]+)\]\(([^\)]+)\)"#)
    nonisolated(unsafe) static let bqRx         = try! NSRegularExpression(pattern: #"^> (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let hrRx         = try! NSRegularExpression(pattern: #"^(-{3,}|\*{3,})$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let taskDoneRx   = try! NSRegularExpression(pattern: #"^- \[[xX]\] (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let taskOpenRx   = try! NSRegularExpression(pattern: #"^- \[ \] (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let bulletRx     = try! NSRegularExpression(pattern: #"^[ \t]*[-*+] (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let numberedRx   = try! NSRegularExpression(pattern: #"^[ \t]*(\d+)\. (.+)$"#, options: .anchorsMatchLines)
    nonisolated(unsafe) static let tableRx      = try! NSRegularExpression(pattern: #"^\|.+\|[ \t]*$"#, options: .anchorsMatchLines)

    static let bodyFont    = NSFont.systemFont(ofSize: 15, weight: .regular)
    static let monoFont    = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
    static let italicFont  = NSFontManager.shared.convert(NSFont.systemFont(ofSize: 15), toHaveTrait: .italicFontMask)
    static let ghostFont   = NSFont.systemFont(ofSize: 0.01)
    static let codeColor   = NSColor.systemOrange
    static let codeBg      = NSColor(white: 0.5, alpha: 0.08)
    static let bqColor     = NSColor.systemBlue.withAlphaComponent(0.85)

    static func headingFont(_ level: Int) -> NSFont {
        switch level {
        case 1: .boldSystemFont(ofSize: 28)
        case 2: .boldSystemFont(ofSize: 22)
        case 3: .boldSystemFont(ofSize: 18)
        case 4: .boldSystemFont(ofSize: 17)
        default: .boldSystemFont(ofSize: 15)
        }
    }

    static func headingPara(_ level: Int) -> NSParagraphStyle {
        let s = NSMutableParagraphStyle()
        s.paragraphSpacingBefore = level == 1 ? 24 : 14
        s.paragraphSpacing = 6
        s.lineSpacing = 4
        return s
    }

    static var bodyPara: NSParagraphStyle {
        let s = NSMutableParagraphStyle()
        s.lineSpacing = 5
        s.paragraphSpacing = 4
        return s
    }

    static var codePara: NSParagraphStyle {
        let s = NSMutableParagraphStyle()
        s.lineSpacing = 3
        s.paragraphSpacingBefore = 8
        s.paragraphSpacing = 8
        return s
    }

    // MARK: - Render

    static func render(_ markdown: String) -> NSAttributedString {
        let result = NSMutableAttributedString(
            string: markdown,
            attributes: [
                .font: bodyFont,
                .foregroundColor: NSColor.labelColor,
                .paragraphStyle: bodyPara
            ]
        )
        let ns = markdown as NSString
        let full = NSRange(location: 0, length: result.length)

        var protectedRanges: [NSRange] = []

        func protect(_ r: NSRange) { protectedRanges.append(r) }
        func isProtected(_ r: NSRange) -> Bool {
            protectedRanges.contains { NSIntersectionRange($0, r).length > 0 }
        }
        func ghost(_ r: NSRange) {
            guard r.length > 0, r.location + r.length <= result.length else { return }
            result.addAttribute(.font, value: ghostFont, range: r)
            result.addAttribute(.foregroundColor, value: NSColor.clear, range: r)
        }

        // 1. Fenced code blocks
        codeBlockRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR   = m.range
            let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound else { return }

            result.setAttributes([
                .font: monoFont,
                .foregroundColor: codeColor,
                .backgroundColor: codeBg,
                .paragraphStyle: codePara
            ], range: wholeR)

            // Ghost the ``` fence lines
            let openFence  = NSRange(location: wholeR.location, length: contentR.location - wholeR.location)
            let closeFence = NSRange(location: contentR.location + contentR.length,
                                      length: wholeR.location + wholeR.length - (contentR.location + contentR.length))
            ghost(openFence); ghost(closeFence)
            protect(wholeR)
        }

        // 2. Blockquotes
        bqRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            result.addAttribute(.foregroundColor, value: bqColor, range: wholeR)
            result.addAttribute(.font, value: italicFont, range: wholeR)
            ghost(NSRange(location: wholeR.location, length: 2))  // "> "
        }

        // 3. Headings
        headingRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 3 else { return }
            let wholeR = m.range; let markerR = m.range(at: 1); let titleR = m.range(at: 2)
            guard titleR.location != NSNotFound, !isProtected(wholeR) else { return }
            let level = markerR.length
            result.addAttribute(.font, value: headingFont(level), range: wholeR)
            result.addAttribute(.paragraphStyle, value: headingPara(level), range: wholeR)
            ghost(NSRange(location: markerR.location, length: markerR.length + 1))
        }

        // 4. Task lists (before bullet to take priority)
        taskDoneRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            ghost(NSRange(location: wholeR.location, length: contentR.location - wholeR.location))
            result.addAttribute(.strikethroughStyle, value: NSUnderlineStyle.single.rawValue, range: contentR)
            result.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: contentR)
            // Insert ☑ symbol
            result.replaceCharacters(in: NSRange(location: wholeR.location, length: 0), with: "☑ ")
        }

        taskOpenRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            ghost(NSRange(location: wholeR.location, length: contentR.location - wholeR.location))
            result.replaceCharacters(in: NSRange(location: wholeR.location, length: 0), with: "☐ ")
        }

        // 5. Bold
        boldRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            result.addAttribute(.font, value: NSFont.boldSystemFont(ofSize: bodyFont.pointSize), range: contentR)
            ghost(NSRange(location: wholeR.location, length: 2))
            ghost(NSRange(location: wholeR.location + wholeR.length - 2, length: 2))
        }

        // 6. Italic
        italicRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            result.addAttribute(.font, value: italicFont, range: contentR)
            ghost(NSRange(location: wholeR.location, length: 1))
            ghost(NSRange(location: wholeR.location + wholeR.length - 1, length: 1))
        }

        // 7. Strikethrough
        strikeRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            result.addAttribute(.strikethroughStyle, value: NSUnderlineStyle.single.rawValue, range: contentR)
            result.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: contentR)
            ghost(NSRange(location: wholeR.location, length: 2))
            ghost(NSRange(location: wholeR.location + wholeR.length - 2, length: 2))
        }

        // 8. Inline code
        inlineCodeRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            result.addAttribute(.font, value: monoFont, range: contentR)
            result.addAttribute(.foregroundColor, value: codeColor, range: contentR)
            result.addAttribute(.backgroundColor, value: codeBg, range: contentR)
            ghost(NSRange(location: wholeR.location, length: 1))
            ghost(NSRange(location: wholeR.location + wholeR.length - 1, length: 1))
        }

        // 9. Links
        linkRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 3 else { return }
            let wholeR = m.range; let labelR = m.range(at: 1); let urlR = m.range(at: 2)
            guard labelR.location != NSNotFound, urlR.location != NSNotFound, !isProtected(wholeR) else { return }
            result.addAttribute(.foregroundColor, value: NSColor.linkColor, range: labelR)
            result.addAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, range: labelR)
            if let url = URL(string: ns.substring(with: urlR)) {
                result.addAttribute(.link, value: url, range: labelR)
            }
            let closeB = NSRange(location: labelR.location + labelR.length, length: 1)
            let paren  = NSRange(location: closeB.location + 1,
                                  length: wholeR.location + wholeR.length - (closeB.location + 1))
            ghost(NSRange(location: wholeR.location, length: 1))  // [
            ghost(closeB)                                           // ]
            if paren.length > 0 { ghost(paren) }                   // (url)
        }

        // 10. Bullet lists
        bulletRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 2 else { return }
            let wholeR = m.range; let contentR = m.range(at: 1)
            guard contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            let prefixRange = NSRange(location: wholeR.location, length: contentR.location - wholeR.location)
            ghost(prefixRange)
            result.replaceCharacters(in: NSRange(location: wholeR.location, length: 0), with: "• ")
        }

        // 11. Numbered lists
        numberedRx.enumerateMatches(in: markdown, range: full) { m, _, _ in
            guard let m = m, m.numberOfRanges >= 3 else { return }
            let wholeR = m.range; let numR = m.range(at: 1); let contentR = m.range(at: 2)
            guard numR.location != NSNotFound, contentR.location != NSNotFound, !isProtected(wholeR) else { return }
            // Keep "1. " visible but muted
            let prefixRange = NSRange(location: wholeR.location, length: contentR.location - wholeR.location)
            result.addAttribute(.foregroundColor, value: NSColor.secondaryLabelColor, range: prefixRange)
        }

        return result
    }
}
