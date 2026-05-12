import SwiftUI

// MARK: - TagPickerView

struct TagPickerView: View {

    @Binding var tags: [String]
    var allTags: [String] = []
    var onChanged: (([String]) -> Void)? = nil

    @State private var inputText: String = ""
    @State private var showSuggestions: Bool = false

    private var suggestions: [String] {
        guard !inputText.isEmpty else { return [] }
        let lower = inputText.lowercased()
        return allTags.filter {
            $0.lowercased().hasPrefix(lower) && !tags.contains($0)
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            // Current tags + input field
            FlowLayout(spacing: 4) {
                ForEach(tags, id: \.self) { tag in
                    tagChip(tag)
                }

                TextField("Add tag…", text: $inputText)
                    .textFieldStyle(.plain)
                    .frame(minWidth: 80)
                    .onSubmit { commitInput() }
                    .onChange(of: inputText) { showSuggestions = !suggestions.isEmpty }
            }

            // Autocomplete dropdown
            if showSuggestions {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(suggestions.prefix(5), id: \.self) { suggestion in
                        Text(suggestion)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .contentShape(Rectangle())
                            .onTapGesture { addTag(suggestion) }
                        Divider()
                    }
                }
                .background(.regularMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 6))
                .shadow(radius: 4)
            }
        }
        // Commit on comma
        .onChange(of: inputText) { value in
            if value.hasSuffix(",") {
                inputText = String(value.dropLast())
                commitInput()
            }
        }
    }

    // MARK: - Tag Chip

    private func tagChip(_ tag: String) -> some View {
        HStack(spacing: 3) {
            Text(tag)
                .font(.caption)
            Button {
                removeTag(tag)
            } label: {
                Image(systemName: "xmark")
                    .font(.caption2)
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 7)
        .padding(.vertical, 3)
        .background(TagBadge(tagName: tag).color.opacity(0.2))
        .clipShape(Capsule())
    }

    // MARK: - Actions

    private func commitInput() {
        let trimmed = inputText.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        addTag(trimmed)
    }

    private func addTag(_ tag: String) {
        let sanitized = tag.lowercased().trimmingCharacters(in: .whitespaces)
        guard !sanitized.isEmpty, !tags.contains(sanitized) else {
            inputText = ""
            showSuggestions = false
            return
        }
        tags.append(sanitized)
        inputText = ""
        showSuggestions = false
        onChanged?(tags)
    }

    private func removeTag(_ tag: String) {
        tags.removeAll { $0 == tag }
        onChanged?(tags)
    }
}

// MARK: - FlowLayout

/// Simple left-to-right wrapping layout for tag chips + input field.
struct FlowLayout: Layout {

    var spacing: CGFloat = 4

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? .infinity
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0

        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x + size.width > width, x > 0 {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return CGSize(width: width, height: y + rowHeight)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX
        var y = bounds.minY
        var rowHeight: CGFloat = 0

        for subview in subviews {
            let size = subview.sizeThatFits(.unspecified)
            if x + size.width > bounds.maxX, x > bounds.minX {
                x = bounds.minX
                y += rowHeight + spacing
                rowHeight = 0
            }
            subview.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
    }
}
