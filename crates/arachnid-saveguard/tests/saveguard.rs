#![cfg(windows)]

use arachnid_saveguard::{Snapshot, SourceProfile, Store, save_json, source};
use std::{fs, time::Duration};

struct Fixture {
    _directory: tempfile::TempDir,
    source: std::path::PathBuf,
    backup: std::path::PathBuf,
    profile: SourceProfile,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("saves");
        let backup = directory.path().join("backups");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&backup).unwrap();
        let profile = SourceProfile::new(&source, &backup).unwrap();
        Self {
            _directory: directory,
            source,
            backup,
            profile,
        }
    }
    fn store(&self) -> Store {
        Store::open(&self.backup, self.profile.clone()).unwrap()
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        fs::write(self.source.join(name), bytes).unwrap();
    }
}

#[test]
fn verified_groups_rebuild_history_without_index_and_never_claim_game_loading() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"original slot");
    fs::create_dir(fixture.source.join("nested")).unwrap();
    fixture.write("nested/progress.dat", b"original progress");
    let before = source::capture(&fixture.source).unwrap();
    let store = fixture.store();
    let snapshot = store.snapshot("manual", Duration::ZERO).unwrap();
    assert_eq!(snapshot.files.len(), 2);
    assert_eq!(snapshot.status, "copyVerified");
    assert_eq!(source::capture(&fixture.source).unwrap(), before);
    drop(store);
    let rebuilt = fixture.store().history().unwrap();
    assert_eq!(rebuilt.len(), 1);
    assert_eq!(rebuilt[0].id, snapshot.id);
    assert!(
        fixture
            .store()
            .reconcile("watch", Duration::ZERO)
            .unwrap()
            .is_none()
    );
}

#[test]
fn overlapping_paths_reparse_and_size_limits_refuse_without_touching_sources() {
    let fixture = Fixture::new();
    assert!(SourceProfile::new(&fixture.source, &fixture.source).is_err());
    let nested = fixture.source.join("nested");
    fs::create_dir(&nested).unwrap();
    assert!(SourceProfile::new(&fixture.source, &nested).is_err());
    fixture.write("oversized.dat", &vec![7; 1024 * 1024 + 1]);
    assert_eq!(
        fixture
            .store()
            .snapshot("manual", Duration::ZERO)
            .unwrap_err()
            .code,
        "operationLimit"
    );
    assert!(fixture.store().history().unwrap().is_empty());
    assert_eq!(
        fs::metadata(fixture.source.join("oversized.dat"))
            .unwrap()
            .len(),
        1024 * 1024 + 1
    );
}

#[test]
fn fragmented_or_locked_writer_publishes_nothing_then_reconciliation_recovers() {
    use std::os::windows::fs::OpenOptionsExt;
    fixture_writer_test(false);
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"original");
    let writer = fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(fixture.source.join("slot.dat"))
        .unwrap();
    assert_eq!(
        fixture
            .store()
            .snapshot("manual", Duration::ZERO)
            .unwrap_err()
            .code,
        "saveUnstable"
    );
    assert!(fixture.store().history().unwrap().is_empty());
    drop(writer);
    assert!(
        fixture
            .store()
            .reconcile("watch", Duration::ZERO)
            .unwrap()
            .is_some()
    );
}

fn fixture_writer_test(rename: bool) {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"first fragment");
    let source = fixture.source.clone();
    let writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        if rename {
            fs::rename(source.join("slot.dat"), source.join("renamed.dat")).unwrap();
        } else {
            fs::write(source.join("slot.dat"), b"complete rewritten generation").unwrap();
        }
    });
    assert_eq!(
        fixture
            .store()
            .snapshot("manual", Duration::from_millis(150))
            .unwrap_err()
            .code,
        "saveUnstable"
    );
    writer.join().unwrap();
    assert!(fixture.store().history().unwrap().is_empty());
    assert!(
        fixture
            .store()
            .reconcile("watch", Duration::ZERO)
            .unwrap()
            .is_some()
    );
}

#[test]
fn rename_during_quiet_window_is_not_published_as_the_old_group() {
    fixture_writer_test(true);
}

#[test]
fn incomplete_staging_is_invisible_and_corrupt_snapshots_are_never_restored() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"original");
    let store = fixture.store();
    let snapshot = store.snapshot("manual", Duration::ZERO).unwrap();
    let root = fixture.backup.join(&fixture.profile.id);
    fs::create_dir(root.join("staging-interrupted")).unwrap();
    fs::write(root.join("staging-interrupted/partial.dat"), b"partial").unwrap();
    assert_eq!(store.history().unwrap().len(), 1);
    fs::write(root.join(&snapshot.id).join("files/slot.dat"), b"tampered").unwrap();
    assert!(store.history().is_err());
    fixture.write("slot.dat", b"current");
    assert!(store.prepare_restore(&snapshot.id, || Ok(())).is_err());
    assert_eq!(
        fs::read(fixture.source.join("slot.dat")).unwrap(),
        b"current"
    );
}

#[test]
fn retention_preserves_pins_newest_and_orders_by_sequence_when_clock_goes_back() {
    let mut fixture = Fixture::new();
    fixture.profile.keep = 1;
    fixture.write("slot.dat", b"one");
    let store = fixture.store();
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    store.pin(&first.id, true).unwrap();
    let manifest = fixture
        .backup
        .join(&fixture.profile.id)
        .join(&first.id)
        .join("manifest.json");
    let mut altered: Snapshot = arachnid_saveguard::load_json(&manifest, 65536).unwrap();
    altered.created_at = u64::MAX;
    save_json(&manifest, &altered).unwrap();
    fixture.write("slot.dat", b"two");
    let second = store.snapshot("manual", Duration::ZERO).unwrap();
    fixture.write("slot.dat", b"three");
    let third = store.snapshot("manual", Duration::ZERO).unwrap();
    let history = store.history().unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, third.id);
    assert!(history.iter().any(|s| s.id == first.id && s.pinned));
    assert!(!history.iter().any(|s| s.id == second.id));
    store.pin(&first.id, false).unwrap();
    assert_eq!(store.history().unwrap().len(), 1);
}

#[test]
fn quota_prunes_oldest_unpinned_copy_and_preserves_the_latest() {
    let mut fixture = Fixture::new();
    fixture.profile.quota_bytes = 12 * 1024 * 1024;
    for i in 0..4 {
        fixture.write(&format!("{i}.dat"), &vec![i; 1024 * 1024]);
    }
    let store = fixture.store();
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    let second = store.snapshot("manual", Duration::ZERO).unwrap();
    let third = store.snapshot("manual", Duration::ZERO).unwrap();
    let ids: Vec<_> = store.history().unwrap().into_iter().map(|s| s.id).collect();
    assert!(!ids.contains(&first.id));
    assert!(ids.contains(&second.id) && ids.contains(&third.id));
    store.pin(&second.id, true).unwrap();
    assert_eq!(
        store.snapshot("manual", Duration::ZERO).unwrap_err().code,
        "saveQuotaReached"
    );
    assert_eq!(store.history().unwrap().len(), 2);
}

#[test]
fn consent_restore_and_undo_keep_exact_bytes_absence_and_precautionary_copy() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"old progress");
    let store = fixture.store();
    let snapshot = store.snapshot("manual", Duration::ZERO).unwrap();
    fixture.write("slot.dat", b"new progress");
    fixture.write("extra.dat", b"extra");
    let baseline = source::capture(&fixture.source).unwrap();
    let journal = store.prepare_restore(&snapshot.id, || Ok(())).unwrap();
    assert_eq!(journal.plan.actions.len(), 2);
    assert!(
        store
            .history()
            .unwrap()
            .iter()
            .any(|s| s.reason == "beforeRestore" && s.pinned)
    );
    assert!(store.apply_restore("wrong digest", || Ok(())).is_err());
    assert_eq!(source::capture(&fixture.source).unwrap(), baseline);
    store.apply_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(
        fs::read(fixture.source.join("slot.dat")).unwrap(),
        b"old progress"
    );
    assert!(!fixture.source.join("extra.dat").exists());
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), baseline);
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
}

#[test]
fn stopped_game_gate_stale_group_and_concurrent_lease_are_required() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"old");
    let store = fixture.store();
    assert!(Store::open(&fixture.backup, fixture.profile.clone()).is_err());
    let snapshot = store.snapshot("manual", Duration::ZERO).unwrap();
    fixture.write("slot.dat", b"new");
    assert!(
        store
            .prepare_restore(&snapshot.id, || Err(arachnid_core::AppError::new(
                "gameAlreadyRunning"
            )))
            .is_err()
    );
    let journal = store.prepare_restore(&snapshot.id, || Ok(())).unwrap();
    fixture.write("unrelated.dat", b"external writer");
    assert_eq!(
        store
            .apply_restore(&journal.digest, || Ok(()))
            .unwrap_err()
            .code,
        "stalePlan"
    );
    assert_eq!(fs::read(fixture.source.join("slot.dat")).unwrap(), b"new");
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
}

#[test]
fn native_notifications_detect_change_and_periodic_pass_recovers_without_events() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"old");
    let watcher = arachnid_saveguard::watcher::Watcher::new(&fixture.source).unwrap();
    fixture.write("slot.dat", b"new");
    assert!(watcher.changed(Duration::from_secs(2)));
    let store = fixture.store();
    store.reconcile("watch", Duration::ZERO).unwrap();
    drop(watcher);
    fixture.write("slot.dat", b"missed notification");
    assert!(store.reconcile("watch", Duration::ZERO).unwrap().is_some());
    assert!(store.reconcile("watch", Duration::ZERO).unwrap().is_none());
}

#[test]
fn discovery_uses_only_the_known_folder_and_enables_automatic_protection() {
    let fixture = Fixture::new();
    let documents = fixture._directory.path().join("Documents");
    fs::create_dir(&documents).unwrap();
    assert!(
        SourceProfile::discover(&documents, &fixture.backup)
            .unwrap()
            .is_none()
    );
    let source = documents.join("Activision/ASM/Save");
    fs::create_dir_all(&source).unwrap();
    assert!(
        SourceProfile::discover(&documents, &fixture.backup)
            .unwrap()
            .is_none()
    );
    fs::write(source.join("asm_save.fpop"), b"synthetic save").unwrap();
    let profile = SourceProfile::discover(&documents, &fixture.backup)
        .unwrap()
        .unwrap();
    assert!(profile.enabled && profile.automatic && !profile.manual_source);
    assert_eq!(profile.source, fs::canonicalize(source).unwrap());
}

#[test]
fn same_length_bytes_trigger_a_copy_but_metadata_only_changes_do_not() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"AAAA");
    let store = fixture.store();
    store.snapshot("manual", Duration::ZERO).unwrap();
    fixture.write("slot.dat", b"BBBB");
    let changed = store.reconcile("watch", Duration::ZERO).unwrap().unwrap();
    fixture.write("slot.dat", b"BBBB");
    assert!(store.reconcile("watch", Duration::ZERO).unwrap().is_none());
    assert_eq!(store.history().unwrap().len(), 2);
    store.delete(&changed.id).unwrap();
    assert!(store.reconcile("watch", Duration::ZERO).unwrap().is_none());
}

#[test]
fn automatic_copies_share_a_fixed_five_minute_window_across_restarts() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"AAAA");
    let store = fixture.store();
    let first = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    fixture.write("slot.dat", b"BBBB");
    let second = store
        .reconcile_at("afterGame", Duration::ZERO, 1299)
        .unwrap()
        .unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.window_started_at, Some(1000));
    assert_eq!(second.files[0].sha256, source::hash(b"BBBB"));
    assert_eq!(store.history().unwrap().len(), 1);
    let files = store
        .directory()
        .join(&second.id)
        .join(&second.payload_directory);
    assert_eq!(fs::read(files.join("slot.dat")).unwrap(), b"BBBB");
    drop(store);
    let store = fixture.store();
    fixture.write("slot.dat", b"CCCC");
    let third = store
        .reconcile_at("beforeGame", Duration::ZERO, 1300)
        .unwrap()
        .unwrap();
    assert_ne!(first.id, third.id);
    assert_eq!(store.history().unwrap().len(), 2);
    fixture.write("slot.dat", b"DDDD");
    let fourth = store
        .reconcile_at("watch", Duration::ZERO, 1301)
        .unwrap()
        .unwrap();
    assert_eq!(third.id, fourth.id);
    assert_eq!(store.history().unwrap().len(), 2);
    assert_eq!(fs::read(files.join("slot.dat")).unwrap(), b"BBBB");
}

#[test]
fn manual_pinned_and_precautionary_copies_are_never_coalesced() {
    let fixture = Fixture::new();
    let store = fixture.store();
    fixture.write("slot.dat", b"A");
    let manual = store.snapshot("manual", Duration::ZERO).unwrap();
    fixture.write("slot.dat", b"B");
    let automatic = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    assert_ne!(manual.id, automatic.id);
    store.pin(&automatic.id, true).unwrap();
    fixture.write("slot.dat", b"C");
    let next = store
        .reconcile_at("watch", Duration::ZERO, 1001)
        .unwrap()
        .unwrap();
    assert_ne!(automatic.id, next.id);
    let journal = store.prepare_restore(&manual.id, || Ok(())).unwrap();
    let precaution = store.history().unwrap()[0].clone();
    assert!(precaution.pinned);
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
    fixture.write("slot.dat", b"D");
    let last = store
        .reconcile_at("watch", Duration::ZERO, 1002)
        .unwrap()
        .unwrap();
    assert_ne!(precaution.id, last.id);
    assert_eq!(store.history().unwrap().len(), 5);
    assert_eq!(
        store
            .history()
            .unwrap()
            .iter()
            .find(|s| s.id == automatic.id)
            .unwrap()
            .files[0]
            .sha256,
        source::hash(b"B")
    );
}

#[test]
fn clock_rollback_starts_a_new_window_and_retention_handles_updated_payloads() {
    let mut fixture = Fixture::new();
    fixture.profile.keep = 1;
    let store = fixture.store();
    fixture.write("slot.dat", b"A");
    let first = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    fixture.write("slot.dat", b"B");
    store.reconcile_at("watch", Duration::ZERO, 1001).unwrap();
    fixture.write("slot.dat", b"C");
    let next = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    assert_ne!(first.id, next.id);
    assert!(!store.directory().join(first.id).exists());
    assert_eq!(store.history().unwrap().len(), 1);
}

#[test]
fn unpublished_payload_is_ignored_then_cleaned_without_losing_the_current_copy() {
    let fixture = Fixture::new();
    let store = fixture.store();
    fixture.write("slot.dat", b"A");
    let first = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    let orphan = store
        .directory()
        .join(&first.id)
        .join("payload-interrupted");
    fs::create_dir(&orphan).unwrap();
    fs::write(orphan.join("slot.dat"), b"partial").unwrap();
    assert_eq!(
        store.history().unwrap()[0].files[0].sha256,
        source::hash(b"A")
    );
    fixture.write("slot.dat", b"B");
    let next = store
        .reconcile_at("watch", Duration::ZERO, 1001)
        .unwrap()
        .unwrap();
    assert_eq!(first.id, next.id);
    assert!(!orphan.exists());
    assert_eq!(
        store.history().unwrap()[0].files[0].sha256,
        source::hash(b"B")
    );
}

#[test]
fn updated_copy_restores_its_latest_bytes_and_undo_recovers_live_progress() {
    let fixture = Fixture::new();
    let store = fixture.store();
    fixture.write("slot.dat", b"A");
    let first = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    fixture.write("slot.dat", b"B");
    store.reconcile_at("watch", Duration::ZERO, 1001).unwrap();
    fixture.write("slot.dat", b"C");
    let journal = store.prepare_restore(&first.id, || Ok(())).unwrap();
    store.apply_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(fs::read(fixture.source.join("slot.dat")).unwrap(), b"B");
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(fs::read(fixture.source.join("slot.dat")).unwrap(), b"C");
}

#[test]
fn replacement_quota_failure_preserves_the_previous_complete_payload() {
    let mut fixture = Fixture::new();
    fixture.profile.quota_bytes = 8 * 1024 * 1024;
    let store = fixture.store();
    fixture.write("slot.dat", b"A");
    let first = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    fs::write(
        store.directory().join("quota-fixture.dat"),
        vec![0; 8 * 1024 * 1024],
    )
    .unwrap();
    fixture.write("slot.dat", b"B");
    assert_eq!(
        store
            .reconcile_at("watch", Duration::ZERO, 1001)
            .unwrap_err()
            .code,
        "saveQuotaReached"
    );
    assert_eq!(
        store.history().unwrap()[0].files[0].sha256,
        source::hash(b"A")
    );
    assert_eq!(
        fs::read(store.directory().join(first.id).join("files/slot.dat")).unwrap(),
        b"A"
    );
}

#[test]
fn copy_deletion_is_reversible_and_never_touches_live_progress() {
    let fixture = Fixture::new();
    fixture.write("slot.dat", b"A");
    let store = fixture.store();
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    assert_eq!(store.delete(&first.id).unwrap_err().code, "saveLastCopy");
    fixture.write("slot.dat", b"B");
    let second = store.snapshot("manual", Duration::ZERO).unwrap();
    let live = source::capture(&fixture.source).unwrap();
    store.pin(&first.id, true).unwrap();
    assert_eq!(store.delete(&first.id).unwrap_err().code, "savePinned");
    store.pin(&first.id, false).unwrap();
    store.delete(&first.id).unwrap();
    assert_eq!(store.index().unwrap().len(), 1);
    assert_eq!(source::capture(&fixture.source).unwrap(), live);
    store.undo_delete(&first.id).unwrap();
    assert_eq!(store.history().unwrap().len(), 2);
    assert_eq!(source::capture(&fixture.source).unwrap(), live);
    assert!(store.undo_delete("../outside").is_err());
    let journal = store.prepare_restore(&first.id, || Ok(())).unwrap();
    assert_eq!(
        store.delete(&second.id).unwrap_err().code,
        "saveRestorePending"
    );
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
}

#[test]
fn empty_sources_wait_normally_and_a_corrupt_copy_cannot_justify_deleting_the_last_valid_copy() {
    let fixture = Fixture::new();
    let store = fixture.store();
    assert!(store.reconcile("watch", Duration::ZERO).unwrap().is_none());
    assert!(store.index().unwrap().is_empty());
    assert_eq!(
        store.snapshot("manual", Duration::ZERO).unwrap_err().code,
        "saveEmpty"
    );
    fixture.write("slot.dat", b"A");
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    fixture.write("slot.dat", b"B");
    let second = store.snapshot("manual", Duration::ZERO).unwrap();
    fs::write(
        store.directory().join(&first.id).join("files/slot.dat"),
        b"X",
    )
    .unwrap();
    assert_eq!(store.delete(&second.id).unwrap_err().code, "saveLastCopy");
    assert_eq!(store.index().unwrap().len(), 2);
}

#[test]
fn default_quota_upgrade_preserves_custom_limits() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.profile.quota_bytes, 1024 * 1024 * 1024);
    fixture.profile.quota_bytes = 256 * 1024 * 1024;
    assert!(fixture.profile.upgrade_default_quota());
    assert_eq!(fixture.profile.quota_bytes, 1024 * 1024 * 1024);
    assert!(!fixture.profile.upgrade_default_quota());
    fixture.profile.quota_bytes = 128 * 1024 * 1024;
    assert!(!fixture.profile.upgrade_default_quota());
    assert_eq!(fixture.profile.quota_bytes, 128 * 1024 * 1024);
}

#[test]
fn photo_heavy_save_can_be_copied_restored_and_undone() {
    let fixture = Fixture::new();
    for i in 0..9 {
        fixture.write(&format!("photo-{i}.fpop"), &vec![i; 512 * 1024]);
    }
    let store = fixture.store();
    let original = source::capture(&fixture.source).unwrap();
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    for i in 0..9 {
        fixture.write(&format!("photo-{i}.fpop"), &vec![i + 1; 512 * 1024]);
    }
    let changed = source::capture(&fixture.source).unwrap();
    let journal = store.prepare_restore(&first.id, || Ok(())).unwrap();
    store.apply_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), original);
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), changed);
}

#[test]
fn photo_album_over_32_files_keeps_automatic_copies_restore_and_undo_working() {
    let fixture = Fixture::new();
    fixture.write("asm_save.fpop", b"original progress");
    for i in 0..49 {
        fixture.write(&format!("ASM_File_Slot_0_Image_{i}.fpop"), &[i; 8192]);
    }
    let original = source::capture(&fixture.source).unwrap();
    let store = fixture.store();
    let first = store
        .reconcile_at("watch", Duration::ZERO, 1000)
        .unwrap()
        .unwrap();
    assert_eq!(first.files.len(), 50);
    for i in 0..49 {
        fixture.write(&format!("ASM_File_Slot_0_Image_{i}.fpop"), &[i + 1; 8192]);
    }
    fixture.write("asm_save.fpop", b"new progress");
    let changed = source::capture(&fixture.source).unwrap();
    let second = store
        .reconcile_at("watch", Duration::ZERO, 1301)
        .unwrap()
        .unwrap();
    assert_ne!(first.id, second.id);
    let journal = store.prepare_restore(&first.id, || Ok(())).unwrap();
    assert_eq!(journal.plan.actions.len(), 50);
    store.apply_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), original);
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), changed);
}

#[test]
fn expanded_album_still_rejects_excess_files_and_total_bytes_without_pruning() {
    for excess_bytes in [false, true] {
        let fixture = Fixture::new();
        fixture.write("asm_save.fpop", b"progress");
        let store = fixture.store();
        let first = store.snapshot("manual", Duration::ZERO).unwrap();
        let count = if excess_bytes { 65 } else { source::MAX_FILES };
        for i in 0..count {
            let file = fs::File::create(fixture.source.join(format!("photo-{i}.fpop"))).unwrap();
            file.set_len(if excess_bytes { 1024 * 1024 } else { 1 })
                .unwrap();
        }
        assert_eq!(
            store.reconcile("watch", Duration::ZERO).unwrap_err().code,
            "operationLimit"
        );
        assert_eq!(store.history().unwrap()[0].id, first.id);
        assert_eq!(
            fs::read(fixture.source.join("asm_save.fpop")).unwrap(),
            b"progress"
        );
    }
}

#[test]
fn full_album_restore_supports_disjoint_generations_and_undo() {
    let fixture = Fixture::new();
    for i in 0..source::MAX_FILES {
        fixture.write(&format!("old-{i}.fpop"), b"old");
    }
    let original = source::capture(&fixture.source).unwrap();
    let store = fixture.store();
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    assert!(serde_json::to_vec(&first).unwrap().len() < 64 * 1024);
    for i in 0..source::MAX_FILES {
        fs::remove_file(fixture.source.join(format!("old-{i}.fpop"))).unwrap();
        fixture.write(&format!("new-{i}.fpop"), b"new");
    }
    let changed = source::capture(&fixture.source).unwrap();
    let journal = store.prepare_restore(&first.id, || Ok(())).unwrap();
    assert_eq!(journal.plan.actions.len(), 2 * source::MAX_FILES);
    store.apply_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), original);
    store.undo_restore(&journal.digest, || Ok(())).unwrap();
    assert_eq!(source::capture(&fixture.source).unwrap(), changed);
}

#[test]
fn automatic_copies_continue_past_storage_and_count_limits() {
    let mut fixture = Fixture::new();
    fixture.profile.keep = 2;
    fixture.profile.quota_bytes = 8 * 1024 * 1024;
    let store = fixture.store();
    for i in 0..6 {
        fixture.write("slot.dat", &vec![i; 1024 * 1024]);
        store
            .reconcile_at("watch", Duration::ZERO, 1000 + u64::from(i) * 301)
            .unwrap()
            .unwrap();
        assert!(store.history().unwrap().len() <= 2);
    }
    assert_eq!(
        store.history().unwrap()[0].files[0].sha256,
        source::hash(&vec![5; 1024 * 1024])
    );
}

#[test]
fn quota_reclaims_deleted_copies_before_visible_history() {
    let mut fixture = Fixture::new();
    fixture.profile.quota_bytes = 8 * 1024 * 1024;
    for i in 0..3 {
        fixture.write(&format!("{i}.dat"), &vec![i; 1024 * 1024]);
    }
    let store = fixture.store();
    let first = store.snapshot("manual", Duration::ZERO).unwrap();
    let second = store.snapshot("manual", Duration::ZERO).unwrap();
    store.delete(&first.id).unwrap();
    let third = store.snapshot("manual", Duration::ZERO).unwrap();
    assert!(
        !store
            .directory()
            .join(format!("deleted-{}", first.id))
            .exists()
    );
    let history = store.history().unwrap();
    assert!(history.iter().any(|copy| copy.id == second.id));
    assert!(history.iter().any(|copy| copy.id == third.id));
}

#[test]
fn unstable_copy_at_quota_does_not_prune_existing_history() {
    let mut fixture = Fixture::new();
    fixture.profile.quota_bytes = 8 * 1024 * 1024;
    for i in 0..3 {
        fixture.write(&format!("{i}.dat"), &vec![i; 1024 * 1024]);
    }
    let store = fixture.store();
    store.snapshot("manual", Duration::ZERO).unwrap();
    store.snapshot("manual", Duration::ZERO).unwrap();
    let before: Vec<_> = store
        .history()
        .unwrap()
        .into_iter()
        .map(|copy| copy.id)
        .collect();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        store.snapshot_with_checkpoint("manual", Duration::ZERO, |stage| {
            if stage == arachnid_saveguard::SnapshotStage::ManifestSynced {
                panic!("simulated interruption before publication")
            }
        })
    }));
    assert!(result.is_err());
    let after: Vec<_> = store
        .history()
        .unwrap()
        .into_iter()
        .map(|copy| copy.id)
        .collect();
    assert_eq!(after, before);
}
