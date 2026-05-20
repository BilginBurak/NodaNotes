use tauri::WebviewWindow;

/// Configures platform-specific window properties.
/// Especially focuses on native transparent frame rendering on macOS Sonoma+.
pub fn setup_window(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        // On macOS, the window features are already largely set via tauri.conf.json
        // (transparent: true, hiddenTitle: true).
        // Let's ensure smooth native shadows and look.
        let _ = window.set_shadow(true);
    }

    Ok(())
}
