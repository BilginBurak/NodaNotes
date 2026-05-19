import SwiftUI

// MARK: - FolderItem

struct FolderItem: Identifiable, Hashable {
    let id: String
    let url: URL
    let name: String
    var children: [FolderItem]

    static func == (lhs: FolderItem, rhs: FolderItem) -> Bool { lhs.id == rhs.id }
    func hash(into hasher: inout Hasher) { hasher.combine(id) }
}

// MARK: - FolderTreeView

struct FolderTreeView: View {

    @Binding var selection: SidebarSelection?
    @EnvironmentObject private var appState: AppState
    @State private var expandedFolders: Set<URL> = []
    @State private var renamingFolder: String? = nil
    @State private var renameText: String = ""

    private var rootFolders: [FolderItem] {
        guard let vaultURL = appState.vaultURL else { return [] }
        return loadFolders(at: vaultURL)
    }

    var body: some View {
        let _ = appState.fsVersion // Dependency trigger for re-scan
        if let vaultURL = appState.vaultURL {
            let vaultItem = FolderItem(
                id: vaultURL.path,
                url: vaultURL,
                name: vaultURL.lastPathComponent,
                children: rootFolders
            )
            folderRow(vaultItem)
        }
    }

    // MARK: - Recursive Row

    private func folderRow(_ folder: FolderItem) -> AnyView {
        if folder.children.isEmpty {
            return AnyView(leafRow(folder))
        } else {
            return AnyView(
                DisclosureGroup(
                    isExpanded: Binding(
                        get: { expandedFolders.contains(folder.url) },
                        set: { expanded in
                            if expanded { expandedFolders.insert(folder.url) }
                            else { expandedFolders.remove(folder.url) }
                        }
                    )
                ) {
                    ForEach(folder.children) { child in
                        folderRow(child)
                            .padding(.leading, 8)
                    }
                } label: {
                    folderLabel(folder)
                        .contextMenu { contextMenu(for: folder) }
                }
            )
        }
    }

    private func leafRow(_ folder: FolderItem) -> some View {
        folderLabel(folder)
            .tag(SidebarSelection.folder(folder.url))
            .contextMenu { contextMenu(for: folder) }
            .draggable(folder.url.path)
            .dropDestination(for: String.self) { paths, _ in
                moveItems(paths: paths, to: folder.url)
                return true
            }
    }

    @ViewBuilder
    private func folderLabel(_ folder: FolderItem) -> some View {
        if renamingFolder == folder.id {
            TextField("Folder name", text: $renameText)
                .textFieldStyle(.plain)
                .onSubmit { commitRename(folder: folder) }
                .onExitCommand { renamingFolder = nil }
        } else {
            Label(folder.name, systemImage: folder.url == appState.vaultURL ? "archivebox" : "folder")
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
                .tag(SidebarSelection.folder(folder.url))
                .onTapGesture { selection = .folder(folder.url) }
                .draggable(folder.url.path)
                .dropDestination(for: String.self) { paths, _ in
                    moveItems(paths: paths, to: folder.url)
                    return true
                }
        }
    }

    // MARK: - Context Menu

    @ViewBuilder
    private func contextMenu(for folder: FolderItem) -> some View {
        Button("New Folder") { createSubfolder(in: folder) }
        Button("Rename") { beginRename(folder: folder) }
        Divider()
        Button("Delete", role: .destructive) { deleteFolder(folder) }
        Divider()
        Button("Show in Finder") {
            NSWorkspace.shared.activateFileViewerSelecting([folder.url])
        }
    }

    // MARK: - Drop Target

    // MARK: - Actions

    private func createSubfolder(in parent: FolderItem) {
        Task {
            let manager = FolderManager()
            _ = try? await manager.createFolder(at: parent.url, name: "New Folder")
        }
    }

    private func beginRename(folder: FolderItem) {
        renameText = folder.name
        renamingFolder = folder.id
    }

    private func commitRename(folder: FolderItem) {
        let newName = renameText.trimmingCharacters(in: .whitespaces)
        guard !newName.isEmpty, newName != folder.name else {
            renamingFolder = nil
            return
        }
        Task {
            let manager = FolderManager()
            _ = try? await manager.renameFolder(at: folder.url, to: newName)
            renamingFolder = nil
        }
    }

    private func deleteFolder(_ folder: FolderItem) {
        Task {
            let manager = FolderManager()
            do {
                try await manager.deleteFolder(at: folder.url)
            } catch FolderManagerError.notEmpty {
                // Show confirmation — recursive delete
                try? await manager.deleteFolderRecursively(at: folder.url)
            } catch {}
        }
    }

    private func moveItems(paths: [String], to destinationFolder: URL) {
        for path in paths {
            let source = URL(fileURLWithPath: path)
            let destination = destinationFolder.appendingPathComponent(source.lastPathComponent)
            guard source != destination,
                  !destination.path.hasPrefix(source.path) else { continue } // prevent moving into self
            Task { try? FileManager.default.moveItem(at: source, to: destination) }
        }
    }

    // MARK: - Folder Loading

    private func loadFolders(at url: URL) -> [FolderItem] {
        let fm = FileManager.default
        guard let contents = try? fm.contentsOfDirectory(
            at: url,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        return contents
            .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
            .map { dir in
                FolderItem(
                    id: dir.path,
                    url: dir,
                    name: dir.lastPathComponent,
                    children: loadFolders(at: dir)
                )
            }
    }
}
