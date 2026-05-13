import SwiftUI

// MARK: - NoteRowView

struct NoteRowView: View, Equatable {

    let note: Note
    let isSelected: Bool
    var onRename: ((Note) -> Void)? = nil
    var onMoveToTrash: ((Note) -> Void)? = nil

    static func == (lhs: NoteRowView, rhs: NoteRowView) -> Bool {
        lhs.isSelected == rhs.isSelected &&
        lhs.note.id == rhs.note.id &&
        lhs.note.updated == rhs.note.updated &&
        lhs.note.title == rhs.note.title &&
        lhs.note.tags == rhs.note.tags
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(note.title)
                .font(.headline)
                .lineLimit(1)

            HStack(spacing: 4) {
                Text(note.updated, formatter: Self.dateFormatter)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Spacer()

                ForEach(note.tags.prefix(3), id: \.self) { tag in
                    TagBadge(tagName: tag)
                }
            }
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .contextMenu { contextMenu }
        .draggable(note.filePath.path)
    }

    private static let dateFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateStyle = .medium
        f.timeStyle = .short
        f.doesRelativeDateFormatting = true
        return f
    }()

    // MARK: - Context Menu

    @ViewBuilder
    private var contextMenu: some View {
        Button("Rename") { onRename?(note) }
        Button("Move to Trash", role: .destructive) { onMoveToTrash?(note) }
        Divider()
        Button("Show in Finder") {
            NSWorkspace.shared.activateFileViewerSelecting([note.filePath])
        }
    }
}

// MARK: - TagBadge

struct TagBadge: View {

    let tagName: String

    var color: Color {
        let hash = abs(tagName.hashValue)
        let hue = Double(hash % 360) / 360.0
        return Color(hue: hue, saturation: 0.6, brightness: 0.85)
    }

    var body: some View {
        Text(tagName)
            .font(.caption2)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(color.opacity(0.2))
            .foregroundStyle(color)
            .clipShape(Capsule())
            .lineLimit(1)
    }
}
