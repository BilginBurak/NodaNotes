import SwiftUI

// MARK: - TagSidebarView

struct TagSidebarView: View {

    @EnvironmentObject private var appState: AppState

    var body: some View {
        ForEach(appState.tags) { tag in
            HStack(spacing: 6) {
                Circle()
                    .fill(tag.color)
                    .frame(width: 8, height: 8)

                Text(tag.name)
                    .lineLimit(1)

                Spacer()

                Text("\(tag.count)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .contentShape(Rectangle())
            .tag(SidebarSelection.tag(tag.name))
            .onTapGesture { appState.filterByTag(tag.name) }
        }
    }
}
