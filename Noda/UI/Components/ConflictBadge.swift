import SwiftUI

// MARK: - ConflictBadge

struct ConflictBadge: View {

    let count: Int

    var body: some View {
        if count > 0 {
            Text("\(count)")
                .font(.caption2)
                .fontWeight(.semibold)
                .foregroundStyle(.white)
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(Color.red)
                .clipShape(Capsule())
        }
    }
}
