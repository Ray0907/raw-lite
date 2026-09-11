use raw_lite_core::{ConversionOptions, discover_raw_files, is_supported_raw};
use std::{fs, path::PathBuf};

static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "raw-lite-discovery-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn recognizes_raw_extensions_case_insensitively() {
    for name in [
        "photo.CR2",
        "photo.arw",
        "photo.Nef",
        "photo.dng",
        "photo.RAF",
    ] {
        assert!(is_supported_raw(name), "expected {name} to be supported");
    }
    for name in ["photo.jpg", "photo.png", "photo", ".dng"] {
        assert!(!is_supported_raw(name), "expected {name} to be ignored");
    }
}

#[test]
fn discovers_raw_files_recursively_in_stable_order_without_duplicates() {
    let root = TestDir::new();
    let nested = root.0.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(root.0.join("b.NEF"), b"").unwrap();
    fs::write(root.0.join("ignore.jpg"), b"").unwrap();
    fs::write(nested.join("a.dng"), b"").unwrap();

    let found = discover_raw_files(&[root.0.clone(), root.0.join("b.NEF")]).unwrap();

    assert_eq!(
        found,
        vec![
            nested.join("a.dng").canonicalize().unwrap(),
            root.0.join("b.NEF").canonicalize().unwrap(),
        ]
    );
}

#[test]
fn validates_conversion_options() {
    assert!(ConversionOptions::new(2000, 75).is_ok());
    assert!(ConversionOptions::new(0, 75).is_err());
    assert!(ConversionOptions::new(2000, 0).is_err());
    assert!(ConversionOptions::new(2000, 101).is_err());
}

#[cfg(unix)]
#[test]
fn dangling_symlink_does_not_hide_other_raw_files() {
    use std::os::unix::fs::symlink;

    let root = TestDir::new();
    let valid = root.0.join("valid.dng");
    fs::write(&valid, b"").unwrap();
    symlink(root.0.join("missing.dng"), root.0.join("broken.dng")).unwrap();

    assert_eq!(
        discover_raw_files(std::slice::from_ref(&root.0)).unwrap(),
        vec![valid.canonicalize().unwrap()]
    );
}

#[cfg(unix)]
#[test]
fn does_not_recurse_into_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let root = TestDir::new();
    let scanned = root.0.join("scanned");
    let outside = root.0.join("outside");
    fs::create_dir(&scanned).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("hidden.dng"), b"").unwrap();
    symlink(&outside, scanned.join("linked-directory")).unwrap();

    assert!(discover_raw_files(&[scanned]).unwrap().is_empty());
}
