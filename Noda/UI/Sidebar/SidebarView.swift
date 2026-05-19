import OSLog
import SwiftUI

// MARK: - SidebarSelection

enum SidebarSelection: Hashable {
    case allNotes
    case recent
    case folder(URL)
    case tag(String)
    case conflicts
    case trash
}

// MARK: - SidebarView

struct SidebarView: View {

    @EnvironmentObject private var appState: AppState
    @Binding var selection: SidebarSelection?
    @State private var searchText = ""
    @FocusState private var searchFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            List(selection: $selection) {
                // MARK: Top section
                Section {
                    Label("All Notes", systemImage: "note.text")
                        .tag(SidebarSelection.allNotes)

                    Label("Recent", systemImage: "clock")
                        .tag(SidebarSelection.recent)
                }

                // MARK: Folders
                Section("Folders") {
                    FolderTreeView(selection: $selection)
                }

                // MARK: Tags
                if !appState.tags.isEmpty {
                    Section("Tags") {
                        TagSidebarView()
                    }
                }
            }
            .listStyle(.sidebar)

            Divider()

            // MARK: Bottom Sticky Group
            VStack(alignment: .leading, spacing: 8) {
                // Sync
                Button { appState.syncManually() } label: {
                    HStack {
                        switch appState.syncStatus {
                        case .idle:
                            Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                                .foregroundStyle(.secondary)
                        case .syncing:
                            HStack(spacing: 6) {
                                ProgressView().scaleEffect(0.65).frame(width: 14, height: 14)
                                Text("Syncing…").foregroundStyle(.secondary)
                            }
                        case .error:
                            Label("Sync Error", systemImage: "exclamationmark.icloud")
                                .foregroundStyle(.red)
                        }
                        Spacer()
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .disabled({ if case .syncing = appState.syncStatus { return true }; return false }())
                .help(syncHelp)
                .overlay(alignment: .topLeading) {
                    if case .syncing(_, let detail) = appState.syncStatus {
                        Text(detail)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.tail)
                            .frame(width: 180, alignment: .leading)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 4)
                            .background(.thinMaterial)
                            .clipShape(RoundedRectangle(cornerRadius: 4))
                            .offset(y: -26)
                            .allowsHitTesting(false)
                            .animation(.easeOut(duration: 0.15), value: detail)
                            .transition(.opacity.combined(with: .move(edge: .bottom)))
                    }
                }

                // Conflicts
                HStack {
                    Label("Conflicts", systemImage: "exclamationmark.triangle")
                        .foregroundStyle(selection == .conflicts ? .primary : .secondary)
                    Spacer()
                    ConflictBadge(count: appState.conflictCount)
                }
                .contentShape(Rectangle())
                .onTapGesture { selection = .conflicts }

                // Trash
                Label("Trash", systemImage: "trash")
                    .foregroundStyle(selection == .trash ? .primary : .secondary)
                    .contentShape(Rectangle())
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .onTapGesture { selection = .trash }

                // Settings
                Button {
                    NotificationCenter.default.post(name: NSNotification.Name("com.noda.showSettings"), object: nil)
                } label: {
                    Label("Settings", systemImage: "gear")
                        .foregroundStyle(.secondary)
                }
                .buttonStyle(.plain)

                // Search (Bottom-most)
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                    TextField("Search", text: $searchText)
                        .textFieldStyle(.plain)
                        .focused($searchFocused)
                        .onSubmit { appState.searchQuery = searchText }
                        .onChange(of: searchText) { _, newValue in appState.searchQuery = newValue }
                    
                    if !searchText.isEmpty {
                        Button {
                            searchText = ""
                            appState.searchQuery = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundStyle(.secondary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.top, 4)
            }
            .padding(12)
            .background(.thinMaterial)
        }
        .navigationTitle("Noda")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    NotificationCenter.default.post(name: .createNoteRequest, object: nil)
                } label: {
                    Image(systemName: "square.and.pencil")
                }
                .help("New Note (⌘N)")
            }
        }
        .onReceive(NotificationCenter.default.publisher(for: .focusSearch)) { _ in
            searchFocused = true
        }
        .disabled(appState.isNavigationLocked)
    }

    // MARK: - Sync Button

    @ViewBuilder
    private var syncButton: some View {
        Button { appState.syncManually() } label: {
            switch appState.syncStatus {
            case .idle:
                Image(systemName: "arrow.triangle.2.circlepath")
            case .syncing:
                ProgressView().scaleEffect(0.7).frame(width: 16, height: 16)
            case .error:
                Image(systemName: "exclamationmark.icloud").foregroundStyle(.red)
            }
        }
        .help(syncHelp)
        .disabled({ if case .syncing = appState.syncStatus { return true }; return false }())
    }

    private var syncHelp: String {
        switch appState.syncStatus {
        case .idle:           return "Sync Now (⌘⇧S)"
        case .syncing:        return "Syncing…"
        case .error(let m):   return "Sync error: \(m)"
        }
    }
}
