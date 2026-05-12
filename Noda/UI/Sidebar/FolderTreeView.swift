import SwiftUI

// MARK: - FolderItem

struct FolderItem: Identifiable, Hashable {
    let id: URL
    let url: URL
    let name: String
    var children: [FolderItem]

    static func == (lhs: FolderItem, rhs: FolderItem) -> Bool { lhs.url == rhs.url }
    func hash(into hasher: inout Hasher) { hasher.combine(url) }
}

// MARK: - FolderTreeView

struct FolderTreeView: View {

    @Binding var selection: SidebarSelection?
    @EnvironmentObject private var appState: AppState
    @State private var expandedFolders: Set<URL> = []
    @State private var renamingFolder: URL? = nil
    @State private var renameText: String = ""

    private var rootFolders: [FolderItem] {
        guard let vaultURL = appState.vaultURL else { return [] }
        return loadFolders(at: vaultURL)
    }

    var body: some View {
        ForEach(rootFolders) { folder in
            folderRow(folder)
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
                }
                .contextMenu { contextMenu(for: folder) }
            )
        }
    }

    private func leafRow(_ folder: FolderItem) -> some View {
        folderLabel(folder)
            .tag(SidebarSelection.folder(folder.url))
            .contextMenu { contextMenu(for: folder) }
    }

    @ViewBuilder
    private func folderLabel(_ folder: FolderItem) -> some View {
        if renamingFolder == folder.url {
            TextField("Folder name", text: $renameText)
                .textFieldStyle(.plain)
                .onSubmit { commitRename(folder: folder) }
                .onExitCommand { renamingFolder = nil }
        } else {
            Label(folder.name, systemImage: "folder")
                .tag(SidebarSelection.folder(folder.url))
                .onTapGesture { selection = .folder(folder.url) }
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
            NSWorkspace.shared.selectFile(nil, inFileViewerRootedAtPath: folder.url.path)
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
        renamingFolder = folder.url
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
                    id: dir,
                    url: dir,
                    name: dir.lastPathComponent,
                    children: loadFolders(at: dir)
                )
            }
    }
}
