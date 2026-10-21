//! Portable bundles opt in with a marker beside the actual executable.
//! Resolve on each call: never depend on the working directory or persist the
//! absolute installation path, so moving the whole folder preserves its data.
use std::path::{Path, PathBuf};

pub const PORTABLE_MARKER: &str = "portable.txt";

pub fn portable_root() -> Option<PathBuf> {
    portable_root_at(&super::exe_dir()?)
}

fn portable_root_at(exe_dir: &Path) -> Option<PathBuf> {
    exe_dir.join(PORTABLE_MARKER).is_file().then(|| exe_dir.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_opts_in_and_data_follows_a_moved_bundle() {
        let base = std::env::temp_dir().join(format!("ffta-portable-{}", std::process::id()));
        let original = base.join("original folder");
        let moved = base.join("moved folder");
        std::fs::create_dir_all(&original).unwrap();
        assert_eq!(portable_root_at(&original), None);
        // A directory of the same name must not opt in.
        std::fs::create_dir(original.join(PORTABLE_MARKER)).unwrap();
        assert_eq!(portable_root_at(&original), None);
        std::fs::remove_dir(original.join(PORTABLE_MARKER)).unwrap();
        std::fs::write(original.join(PORTABLE_MARKER), b"portable").unwrap();
        std::fs::create_dir_all(original.join("Data/config")).unwrap();
        std::fs::write(original.join("Data/config/input.cfg"), b"key.menu = Escape").unwrap();
        assert_eq!(portable_root_at(&original), Some(original.clone()));
        std::fs::rename(&original, &moved).unwrap();
        let root = portable_root_at(&moved).unwrap();
        assert_eq!(std::fs::read(root.join("Data/config/input.cfg")).unwrap(), b"key.menu = Escape");
        assert_eq!(portable_root_at(&original), None);
        std::fs::remove_dir_all(base).unwrap();
    }
}
