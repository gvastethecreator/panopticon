//! Canonical snapshot of user-facing top-level windows.

use panopticon::window_enum::{enumerate_windows, WindowInfo};

/// Latest eligible window discovery shared by dashboard, tray, settings, and palette.
#[derive(Debug, Default)]
pub(crate) struct WindowCatalogSnapshot {
    generation: u64,
    windows: Vec<WindowInfo>,
}

impl WindowCatalogSnapshot {
    pub(crate) fn refresh(&mut self) {
        self.windows = enumerate_windows();
        self.generation = self.generation.saturating_add(1);
    }

    pub(crate) fn windows(&self) -> &[WindowInfo] {
        &self.windows
    }

    #[cfg(test)]
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::WindowCatalogSnapshot;

    #[test]
    fn catalog_starts_empty_at_generation_zero() {
        let catalog = WindowCatalogSnapshot::default();
        assert_eq!(catalog.generation(), 0);
        assert!(catalog.windows().is_empty());
    }
}
