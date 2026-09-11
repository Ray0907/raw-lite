#![cfg(unix)]

use raw_lite_core::{ConversionOptions, convert_batch, discover_raw_files};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
};

static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "raw-lite-process-{}-{}-{}",
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

    fn executable(&self, name: &str, script: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, script).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn invokes_dcraw_compatible_decoder_and_uses_stdout() {
    let root = TestDir::new();
    let decoder = root.executable(
        "fake-dcraw",
        "#!/bin/sh\n[ \"$1 $2 $3 $4 $5 $6 $7 $8\" = \"-c -w -q 3 -H 2 -o 1\" ] || exit 9\nprintf 'P6\\n2 1\\n255\\n\\377\\000\\000\\000\\377\\000'\n",
    );
    let output = root.0.join("output");

    let summary = convert_batch(
        &decoder,
        &[PathBuf::from("photo.dng")],
        &output,
        ConversionOptions::new(2000, 75).unwrap(),
        |_| {},
    )
    .unwrap();

    assert_eq!(summary.converted.len(), 1);
    assert!(summary.failed.is_empty());
    assert!(output.join("photo.jpg").is_file());
}

#[test]
fn converts_symlinked_raw_file_nested_in_directory() {
    let root = TestDir::new();
    let scanned = root.0.join("scanned");
    let outside = root.0.join("outside");
    fs::create_dir(&scanned).unwrap();
    fs::create_dir(&outside).unwrap();
    let target = outside.join("photo.dng");
    fs::write(&target, b"raw fixture").unwrap();
    symlink(&target, scanned.join("photo.dng")).unwrap();
    let decoder = root.executable(
        "fake-dcraw",
        "#!/bin/sh\nprintf 'P6\\n2 1\\n255\\n\\377\\000\\000\\000\\377\\000'\n",
    );

    let inputs = discover_raw_files(&[scanned]).unwrap();
    let summary = convert_batch(
        &decoder,
        &inputs,
        &root.0.join("output"),
        ConversionOptions::new(2000, 75).unwrap(),
        |_| {},
    )
    .unwrap();

    assert_eq!(inputs, vec![target.canonicalize().unwrap()]);
    assert_eq!(summary.converted.len(), 1);
    assert!(summary.failed.is_empty());
}

#[test]
fn reports_first_decoder_error_line() {
    let root = TestDir::new();
    let decoder = root.executable(
        "failing-dcraw",
        "#!/bin/sh\nprintf 'unsupported RAW\\nmore detail\\n' >&2\nexit 2\n",
    );

    let summary = convert_batch(
        &decoder,
        &[PathBuf::from("bad.raw")],
        &root.0.join("output"),
        ConversionOptions::new(2000, 75).unwrap(),
        |_| {},
    )
    .unwrap();

    assert!(summary.converted.is_empty());
    assert_eq!(summary.failed[0].reason, "unsupported RAW");
}
