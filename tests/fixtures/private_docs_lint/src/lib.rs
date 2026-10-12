//! Minimal crate used to exercise the repository's private-item Clippy lint.

/// Calls an undocumented private function so Clippy checks the intended lint.
pub fn fixture_entrypoint() -> u8 {
    undocumented_private_item()
}

fn undocumented_private_item() -> u8 {
    1
}
