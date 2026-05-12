import SwiftUI

// MARK: - WYSIWYGEditor

struct WYSIWYGEditor: View {

    let note: Note
    @Binding var isDirty: Bool
    var onSave: (String) -> Void

    @State private var editingLineIndex: Int? = nil
    @State private var editingLineText: String = ""
    @State private var fullRawMode: Bool = false

    private var lines: [String] { note.content.components(separatedBy: "\n") }

    var body: some View {
        if fullRawMode {
            RawMarkdownEditor(note: note, isDirty: $isDirty, onSave: { content in
                onSave(content)
            })
            .onTapGesture(count: 2) { fullRawMode = false }
        } else {
            ScrollView {
                VStack(alignment: .leading, spacing: 2) {
                    ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                        lineView(index: index, line: line)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
            }
            // Double-click anywhere → full raw mode
            .onTapGesture(count: 2) { fullRawMode = true }
        }
    }

    // MARK: - Line View

    @ViewBuilder
    private func lineView(index: Int, line: String) -> some View {
        if editingLineIndex == index {
            // Inline raw editing for this line
            TextField("", text: $editingLineText)
                .textFieldStyle(.plain)
                .font(.system(.body, design: .monospaced))
                .onSubmit { commitLineEdit(index: index) }
                .onExitCommand { editingLineIndex = nil }
        } else {
            renderedLine(line)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                // Single click → edit this line
                .onTapGesture {
                    editingLineText = line
                    editingLineIndex = index
                }
        }
    }

    // MARK: - Rendered Line

    @ViewBuilder
    private func renderedLine(_ line: String) -> some View {
        if line.isEmpty {
            Text(" ").frame(height: 20)
        } else if let attributed = try? AttributedString(markdown: line,
                                                          options: .init(interpretedSyntax: .inlineOnlyPreservingWhitespace)) {
            Text(attributed)
        } else {
            Text(line)
        }
    }

    // MARK: - Commit Line Edit

    private func commitLineEdit(index: Int) {
        var updated = lines
        updated[index] = editingLineText
        let newContent = updated.joined(separator: "\n")
        editingLineIndex = nil
        isDirty = true
        onSave(newContent)
    }
}
