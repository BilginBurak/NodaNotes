import SwiftUI

// MARK: - ErrorAlertView

struct ErrorAlertView: View {

    @EnvironmentObject private var appState: AppState
    let error: PresentedError

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.title2)
                    .foregroundStyle(.red)
                    .padding(.top, 2)

                VStack(alignment: .leading, spacing: 4) {
                    Text("Hata Oluştu")
                        .font(.headline)
                        .foregroundStyle(.primary)

                    Text(error.message)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)

                    if let suggestion = error.suggestion {
                        Text(suggestion)
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                            .padding(.top, 2)
                    }
                }

                Spacer()

                Button {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                        appState.dismissCurrentError()
                    }
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundStyle(.secondary)
                        .padding(6)
                        .background(Color.secondary.opacity(0.1))
                        .clipShape(Circle())
                }
                .buttonStyle(.plain)
            }

            if let retry = error.retry {
                Button {
                    retry()
                    withAnimation {
                        appState.dismissCurrentError()
                    }
                } label: {
                    Text("Yeniden Dene")
                        .font(.subheadline.bold())
                        .foregroundStyle(.white)
                        .padding(.vertical, 6)
                        .frame(maxWidth: .infinity)
                        .background(Color.accentColor)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                }
                .buttonStyle(.plain)
                .transition(.opacity.combined(with: .scale))
            }
        }
        .padding(16)
        .frame(width: 320)
        .background {
            RoundedRectangle(cornerRadius: 16)
                .fill(.ultraThinMaterial)
                .shadow(color: .black.opacity(0.15), radius: 10, x: 0, y: 5)
        }
        .overlay {
            RoundedRectangle(cornerRadius: 16)
                .strokeBorder(Color.red.opacity(0.2), lineWidth: 1)
        }
        .padding(20)
        .onAppear {
            // Auto-dismiss if no retry option
            if error.retry == nil {
                DispatchQueue.main.asyncAfter(deadline: .now() + 5) {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.8)) {
                        appState.dismissCurrentError()
                    }
                }
            }
        }
    }
}

// MARK: - View Extension

extension View {
    func errorAlertOverlay() -> some View {
        modifier(ErrorAlertOverlayModifier())
    }
}

struct ErrorAlertOverlayModifier: ViewModifier {
    @EnvironmentObject private var appState: AppState

    func body(content: Content) -> some View {
        ZStack {
            content

            VStack {
                Spacer()
                if let currentError = appState.errorQueue.first {
                    ErrorAlertView(error: currentError)
                        .transition(
                            .asymmetric(
                                insertion: .move(edge: .bottom).combined(with: .opacity),
                                removal: .opacity.combined(with: .scale(scale: 0.9))
                            )
                        )
                        .id(currentError.id)
                }
            }
            .allowsHitTesting(appState.errorQueue.first != nil)
        }
    }
}

#Preview {
    ErrorAlertView(error: PresentedError(
        message: "Bağlantı hatası oluştu.",
        suggestion: "Lütfen internet bağlantınızı kontrol edip tekrar deneyin.",
        retry: { print("Retry") }
    ))
    .environmentObject(AppState())
}
