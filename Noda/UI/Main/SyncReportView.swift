import SwiftUI

struct SyncReportView: View {
    @Environment(\.dismiss) private var dismiss
    let operations: [SyncOperation]
    
    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Senkronizasyon Raporu")
                    .font(.headline)
                Spacer()
                Button { dismiss() } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(.secondary)
                        .font(.title3)
                }
                .buttonStyle(.plain)
            }
            .padding()
            
            Divider()
            
            if operations.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "checkmark.icloud")
                        .font(.system(size: 48))
                        .foregroundStyle(.green)
                    Text("Değişiklik yok, tüm dosyalar senkronize durumda.")
                        .foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List {
                    Section {
                        ForEach(operations.indices, id: \.self) { idx in
                            let op = operations[idx]
                            HStack(spacing: 12) {
                                icon(for: op)
                                    .font(.system(size: 16))
                                    .frame(width: 24)
                                
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(actionText(for: op))
                                        .font(.subheadline)
                                        .foregroundStyle(.primary)
                                    Text(path(for: op))
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                        .lineLimit(1)
                                        .truncationMode(.middle)
                                }
                                Spacer()
                            }
                            .padding(.vertical, 4)
                        }
                    } header: {
                        Text("\(operations.count) İşlem Tamamlandı")
                    }
                }
                .listStyle(.inset)
            }
        }
        .frame(minWidth: 400, idealWidth: 500, minHeight: 300, idealHeight: 400)
    }
    
    @ViewBuilder
    private func icon(for op: SyncOperation) -> some View {
        switch op {
        case .upload:
            Image(systemName: "arrow.up.circle.fill").foregroundStyle(.blue)
        case .download:
            Image(systemName: "arrow.down.circle.fill").foregroundStyle(.green)
        case .deleteRemote:
            Image(systemName: "trash.slash.fill").foregroundStyle(.red)
        case .deleteLocal:
            Image(systemName: "trash.fill").foregroundStyle(.orange)
        case .conflict:
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.yellow)
        case .makeDirectory:
            Image(systemName: "folder.fill.badge.plus").foregroundStyle(.cyan)
        }
    }
    
    private func actionText(for op: SyncOperation) -> String {
        switch op {
        case .upload: return "Sunucuya Yüklendi"
        case .download: return "Cihaza İndirildi"
        case .deleteRemote: return "Sunucudan Silindi"
        case .deleteLocal: return "Cihazdan Silindi"
        case .conflict: return "Çakışma Çözüldü"
        case .makeDirectory: return "Klasör Oluşturuldu"
        }
    }
    
    private func path(for op: SyncOperation) -> String {
        switch op {
        case .upload(let p), .download(let p), .deleteRemote(let p), .deleteLocal(let p), .makeDirectory(let p):
            return p
        case .conflict(let lp, _):
            return lp
        }
    }
}
