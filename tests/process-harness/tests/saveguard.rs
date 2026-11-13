#![cfg(windows)]
use arachnid_saveguard::{SourceProfile, Store, source};
use std::{
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn original_multifile_writer_is_reconciled_after_fragmented_rename_and_truncate() {
    let directory = tempfile::tempdir().unwrap();
    let saves = directory.path().join("saves");
    let backups = directory.path().join("backups");
    fs::create_dir(&saves).unwrap();
    fs::create_dir(&backups).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_arachnid-process-harness"))
        .args(["save-writer", "400", "0"])
        .arg(directory.path())
        .arg("fixture")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !directory.path().join("writer-ready").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let profile = SourceProfile::new(&saves, &backups).unwrap();
    let store = Store::open(&backups, profile).unwrap();
    assert_eq!(
        store
            .snapshot("manual", Duration::from_millis(600))
            .unwrap_err()
            .code,
        "saveUnstable"
    );
    assert!(child.wait().unwrap().success());
    assert!(store.history().unwrap().is_empty());
    let snapshot = store.reconcile("watch", Duration::ZERO).unwrap().unwrap();
    assert_eq!(snapshot.files.len(), 2);
    assert_eq!(fs::read(saves.join("slot.dat")).unwrap(), b"complete-slot");
    assert_eq!(
        fs::read(saves.join("progress.dat")).unwrap(),
        b"complete-progress"
    );
}

#[test]
fn abrupt_exit_never_publishes_a_partial_snapshot_and_releases_the_store_lock() {
    for checkpoint in ["files", "manifest", "published"] {
        let directory = tempfile::tempdir().unwrap();
        let saves = directory.path().join("saves");
        let backups = directory.path().join("backups");
        fs::create_dir(&saves).unwrap();
        fs::create_dir(&backups).unwrap();
        fs::write(saves.join("slot.dat"), b"original").unwrap();
        fs::write(saves.join("progress.dat"), b"progress").unwrap();
        let before = source::capture(&saves).unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_arachnid-process-harness"))
            .args(["save-crash", "0", "77"])
            .arg(directory.path())
            .arg(checkpoint)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(77));
        assert_eq!(source::capture(&saves).unwrap(), before);
        let profile = SourceProfile::new(&saves, &backups).unwrap();
        let store = Store::open(&backups, profile).unwrap();
        assert_eq!(
            store.history().unwrap().len(),
            usize::from(checkpoint == "published")
        );
        if checkpoint != "published" {
            store.snapshot("manual", Duration::ZERO).unwrap();
        }
        assert_eq!(store.history().unwrap().len(), 1);
    }
}
