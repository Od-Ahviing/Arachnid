#![cfg(windows)]
use arachnid_mods::{
    fix_settings::{self, DisplaySettings},
    manager::{PackManager, PackOperation, PackRevision, PackSpec},
    offline::Payload,
};
use arachnid_repair::{Journal, Phase};
use sha2::{Digest, Sha256};
use std::fs;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn display() -> DisplaySettings {
    DisplaySettings {
        width: 1920,
        height: 1080,
        windowed: false,
    }
}
fn spec() -> PackSpec {
    PackSpec {
        uses_fix: true,
        previous_revisions: Vec::new(),
        version: "synthetic-1".into(),
        game_sha256: "b".repeat(64),
        original_wrapper: Some(hash(b"original")),
        binaries: vec![
            Payload {
                file_name: "d3d9.dll".into(),
                sha256: hash(b"synthetic fix"),
                bytes: 13,
            },
            Payload {
                file_name: "ar_dxvk.dll".into(),
                sha256: hash(b"synthetic dxvk"),
                bytes: 14,
            },
        ],
    }
}
fn source(directory: &std::path::Path) {
    fs::write(directory.join("d3d9.dll"), b"synthetic fix").unwrap();
    fs::write(directory.join("ar_dxvk.dll"), b"synthetic dxvk").unwrap();
}

#[test]
fn previous_pack_can_be_removed_then_replaced_without_losing_its_originals() {
    migrate_pack(b"new synthetic dxvk");
}

#[test]
fn runtime_only_revision_still_requires_approved_pack_replacement() {
    migrate_pack(b"synthetic dxvk");
}

fn migrate_pack(next_binary: &[u8]) {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    source(assets.path());
    let mut old = manager(game.path(), state.path());
    let prepared = old
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    old.apply(&prepared.digest, &display(), || Ok(())).unwrap();
    drop(old);
    let original_spec = spec();
    let mut next_spec = original_spec.clone();
    next_spec.version = "synthetic-2".into();
    next_spec.previous_revisions = vec![PackRevision {
        uses_fix: true,
        version: original_spec.version,
        binaries: original_spec.binaries,
    }];
    next_spec.binaries[1].sha256 = hash(next_binary);
    next_spec.binaries[1].bytes = next_binary.len() as u64;
    let mut next = PackManager::new(
        game.path(),
        state.path(),
        &"a".repeat(64),
        next_spec.clone(),
    )
    .unwrap();
    let status = next.status(&display()).unwrap();
    assert!(status.active && status.requires_update && !status.files_changed);
    assert_eq!(
        next.prepare(
            PackOperation::Sync,
            assets.path(),
            &display(),
            &"b".repeat(64)
        )
        .unwrap_err()
        .code,
        "modPackUpdateRequired"
    );
    let disable = next
        .prepare(
            PackOperation::Disable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    next.apply(&disable.digest, &display(), || Ok(())).unwrap();
    assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
    fs::write(assets.path().join("ar_dxvk.dll"), next_binary).unwrap();
    let enable = next
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    next.apply(&enable.digest, &display(), || Ok(())).unwrap();
    drop(next);
    let mut reopened =
        PackManager::new(game.path(), state.path(), &"a".repeat(64), next_spec).unwrap();
    assert!(!reopened.status(&display()).unwrap().requires_update);
    assert_eq!(
        fs::read(game.path().join("ar_dxvk.dll")).unwrap(),
        next_binary
    );
    let disable = reopened
        .prepare(
            PackOperation::Disable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    reopened
        .apply(&disable.digest, &display(), || Ok(()))
        .unwrap();
    assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
}
fn manager(game: &std::path::Path, state: &std::path::Path) -> PackManager {
    PackManager::new(game, state, &"a".repeat(64), spec()).unwrap()
}

#[test]
fn whole_pack_survives_restart_and_disable_restores_initial_absence() {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    source(assets.path());
    let mut pack = manager(game.path(), state.path());
    let prepared = pack
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
    assert!(pack.apply("wrong", &display(), || Ok(())).is_err());
    pack.apply(&prepared.digest, &display(), || Ok(())).unwrap();
    assert!(pack.status(&display()).unwrap().active);
    drop(pack);
    let mut reopened = manager(game.path(), state.path());
    assert!(reopened.status(&display()).unwrap().active);
    let disable = reopened
        .prepare(
            PackOperation::Disable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    reopened
        .apply(&disable.digest, &display(), || Ok(()))
        .unwrap();
    assert!(!reopened.status(&display()).unwrap().active);
    assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
}

#[test]
fn original_wrapper_and_ini_are_restored_while_current_custom_ini_is_backed_up() {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    source(assets.path());
    fs::write(game.path().join("d3d9.dll"), b"original").unwrap();
    let original_ini = fix_settings::configure(
        None,
        &DisplaySettings {
            width: 1280,
            height: 720,
            windowed: true,
        },
    )
    .unwrap();
    fs::write(game.path().join("d3d9.ini"), &original_ini).unwrap();
    let mut pack = manager(game.path(), state.path());
    let prepared = pack
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    pack.apply(&prepared.digest, &display(), || Ok(())).unwrap();
    let custom = fs::read_to_string(game.path().join("d3d9.ini"))
        .unwrap()
        .replace("fov=1", "fov=2");
    fs::write(game.path().join("d3d9.ini"), custom.as_bytes()).unwrap();
    let disable = pack
        .prepare(
            PackOperation::Disable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    pack.apply(&disable.digest, &display(), || Ok(())).unwrap();
    assert_eq!(fs::read(game.path().join("d3d9.dll")).unwrap(), b"original");
    assert_eq!(
        fs::read(game.path().join("d3d9.ini")).unwrap(),
        original_ini
    );
    assert!(
        state
            .path()
            .join("blobs")
            .join(format!("{}.blob", hash(custom.as_bytes())))
            .exists()
    );
}

#[test]
fn interrupted_enable_never_looks_active_and_can_recover_after_restart() {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    source(assets.path());
    let mut pack = manager(game.path(), state.path());
    let mut journal = pack
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    journal.phase = Phase::Applying;
    journal.intent_count = 1;
    journal
        .save(&state.path().join("performance-transaction.json"))
        .unwrap();
    fs::write(game.path().join("d3d9.dll"), b"synthetic fix").unwrap();
    drop(pack);
    let mut reopened = manager(game.path(), state.path());
    assert!(reopened.status(&display()).unwrap().recovery_required);
    assert!(!reopened.status(&display()).unwrap().active);
    reopened.recover(&journal.digest, || Ok(())).unwrap();
    assert!(!game.path().join("d3d9.dll").exists());
    assert!(!reopened.status(&display()).unwrap().recovery_required);
}

#[test]
fn settings_sync_and_unsupported_new_ratio_never_prevent_disable() {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    source(assets.path());
    let mut pack = manager(game.path(), state.path());
    let initial = pack
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    pack.apply(&initial.digest, &display(), || Ok(())).unwrap();
    let next = DisplaySettings {
        width: 1280,
        height: 720,
        windowed: true,
    };
    assert!(pack.status(&next).unwrap().settings_out_of_sync);
    let sync = pack
        .prepare(PackOperation::Sync, assets.path(), &next, &"b".repeat(64))
        .unwrap();
    assert!(pack.apply(&sync.digest, &display(), || Ok(())).is_err());
    pack.apply(&sync.digest, &next, || Ok(())).unwrap();
    assert!(!pack.status(&next).unwrap().settings_out_of_sync);
    let unsupported = DisplaySettings {
        width: 3440,
        height: 1440,
        windowed: false,
    };
    let disable = pack
        .prepare(
            PackOperation::Disable,
            assets.path(),
            &unsupported,
            &"b".repeat(64),
        )
        .unwrap();
    pack.apply(&disable.digest, &unsupported, || Ok(()))
        .unwrap();
    assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
}

#[test]
fn external_binary_changes_and_forged_target_scope_are_preserved_or_refused() {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    source(assets.path());
    let mut pack = manager(game.path(), state.path());
    let initial = pack
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    pack.apply(&initial.digest, &display(), || Ok(())).unwrap();
    fs::write(game.path().join("ar_dxvk.dll"), b"external").unwrap();
    assert!(pack.status(&display()).unwrap().files_changed);
    assert!(
        pack.prepare(
            PackOperation::Disable,
            assets.path(),
            &display(),
            &"b".repeat(64)
        )
        .is_err()
    );
    assert_eq!(
        fs::read(game.path().join("ar_dxvk.dll")).unwrap(),
        b"external"
    );
    let path = state.path().join("performance-transaction.json");
    let mut journal = Journal::load(&path).unwrap();
    journal.context.as_mut().unwrap()["baseline"]["actions"][0]["target"]["path"] =
        "Game.exe".into();
    journal.digest = journal.expected_digest().unwrap();
    journal.save(&path).unwrap();
    assert!(pack.status(&display()).is_err());
}
