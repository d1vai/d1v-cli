//! Cross-platform key label helpers for prompt hint lines.

/// Returns the platform-specific label for the Ctrl+C shortcut.
///
/// `⌃C` on macOS, `Ctrl+C` elsewhere.
pub const fn ctrl_c_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌃C"
    } else {
        "Ctrl+C"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn ctrl_c_label_on_macos() {
        assert_eq!(ctrl_c_label(), "⌃C");
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn ctrl_c_label_off_macos() {
        assert_eq!(ctrl_c_label(), "Ctrl+C");
    }
}
