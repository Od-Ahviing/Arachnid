#![cfg(windows)]
use arachnid_mods::{
    composition::Composition,
    fix_settings::DisplaySettings,
    manager::{PackManager, PackOperation, PackSpec},
    offline::Payload,
};
use sha2::{Digest, Sha256};
use std::{cell::Cell, fs};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn content(payload: &Payload) -> &'static [u8] {
    if payload.sha256 == arachnid_mods::fix_adapter::ORIGINAL_SHA256 {
        b"original-fix"
    } else if payload.sha256 == arachnid_mods::fix_adapter::ADAPTED_SHA256 {
        b"adapted-fix"
    } else if payload.bytes == 4_460_558 {
        b"dxvk"
    } else {
        b"reshade"
    }
}
fn synthetic_spec(bits: u8, assets: &std::path::Path) -> PackSpec {
    let profile = Composition::from_bits(bits.max(1)).unwrap();
    let mut spec = PackSpec::for_composition(profile);
    for payload in &mut spec.binaries {
        let bytes = content(payload);
        fs::write(assets.join(&payload.file_name), bytes).unwrap();
        payload.bytes = bytes.len() as u64;
        payload.sha256 = hash(bytes);
    }
    for revision in &mut spec.previous_revisions {
        for payload in &mut revision.binaries {
            let bytes = content(payload);
            payload.bytes = bytes.len() as u64;
            payload.sha256 = hash(bytes);
        }
    }
    spec.game_sha256 = "b".repeat(64);
    spec.original_wrapper = None;
    spec
}
fn display() -> DisplaySettings {
    DisplaySettings {
        width: 1920,
        height: 1080,
        windowed: false,
    }
}

#[test]
fn every_composition_can_transition_to_every_other_and_restore_original_absence() {
    for from in 0..=7 {
        for to in 0..=7 {
            let game = tempfile::tempdir().unwrap();
            let state = tempfile::tempdir().unwrap();
            let assets = tempfile::tempdir().unwrap();
            if from != 0 {
                let mut manager = PackManager::new(
                    game.path(),
                    state.path(),
                    &"a".repeat(64),
                    synthetic_spec(from, assets.path()),
                )
                .unwrap();
                let preview = manager
                    .prepare(
                        PackOperation::Enable,
                        assets.path(),
                        &display(),
                        &"b".repeat(64),
                    )
                    .unwrap();
                manager
                    .apply(&preview.digest, &display(), || Ok(()))
                    .unwrap();
            }
            let spec = synthetic_spec(to, assets.path());
            let expected = spec.binaries.clone();
            let mut manager =
                PackManager::new(game.path(), state.path(), &"a".repeat(64), spec).unwrap();
            if from != 0 || to != 0 {
                let operation = if to == 0 {
                    PackOperation::Disable
                } else {
                    PackOperation::Enable
                };
                let preview = manager
                    .prepare(operation, assets.path(), &display(), &"b".repeat(64))
                    .unwrap();
                assert!(manager.apply("wrong", &display(), || Ok(())).is_err());
                manager
                    .apply(&preview.digest, &display(), || Ok(()))
                    .unwrap();
            }
            if to == 0 {
                assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
                continue;
            }
            assert_eq!(
                manager.applied_version().unwrap(),
                Some(Composition::from_bits(to).unwrap().version())
            );
            assert!(!manager.status(&display()).unwrap().files_changed);
            for payload in expected {
                assert_eq!(
                    hash(&fs::read(game.path().join(payload.file_name)).unwrap()),
                    payload.sha256
                );
            }
            assert_eq!(game.path().join("d3d9.ini").exists(), to & 1 != 0);
            let preview = manager
                .prepare(
                    PackOperation::Disable,
                    assets.path(),
                    &display(),
                    &"b".repeat(64),
                )
                .unwrap();
            manager
                .apply(&preview.digest, &display(), || Ok(()))
                .unwrap();
            assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
        }
    }
}

#[test]
fn failed_switch_restores_the_previous_recipe_and_session_only_changes_recheck_files() {
    let game = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    let mut initial = PackManager::new(
        game.path(),
        state.path(),
        &"a".repeat(64),
        synthetic_spec(7, assets.path()),
    )
    .unwrap();
    let preview = initial
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    initial
        .apply(&preview.digest, &display(), || Ok(()))
        .unwrap();
    drop(initial);
    let mut next = PackManager::new(
        game.path(),
        state.path(),
        &"a".repeat(64),
        synthetic_spec(4, assets.path()),
    )
    .unwrap();
    let preview = next
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    let calls = Cell::new(0);
    assert!(
        next.apply(&preview.digest, &display(), || {
            calls.set(calls.get() + 1);
            if calls.get() == 3 {
                Err(arachnid_core::AppError::new("fixtureFailure"))
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    assert_eq!(
        next.applied_version().unwrap(),
        Some(Composition::from_bits(7).unwrap().version())
    );
    assert!(!next.status(&display()).unwrap().files_changed);
    drop(next);
    let mut dxvk = PackManager::new(
        game.path(),
        state.path(),
        &"a".repeat(64),
        synthetic_spec(2, assets.path()),
    )
    .unwrap();
    let preview = dxvk
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    dxvk.apply(&preview.digest, &display(), || Ok(())).unwrap();
    drop(dxvk);
    let mut reshade = PackManager::new(
        game.path(),
        state.path(),
        &"a".repeat(64),
        synthetic_spec(6, assets.path()),
    )
    .unwrap();
    let preview = reshade
        .prepare(
            PackOperation::Enable,
            assets.path(),
            &display(),
            &"b".repeat(64),
        )
        .unwrap();
    assert!(preview.plan.actions.is_empty());
    fs::write(game.path().join("d3d9.dll"), b"external").unwrap();
    assert_eq!(
        reshade
            .apply(&preview.digest, &display(), || Ok(()))
            .unwrap_err()
            .code,
        "stalePlan"
    );
    fs::write(game.path().join("d3d9.dll"), b"dxvk").unwrap();
    reshade.recover(&preview.digest, || Ok(())).unwrap();
    assert_eq!(
        reshade.applied_version().unwrap(),
        Some(Composition::from_bits(2).unwrap().version())
    );
    assert!(!reshade.status(&display()).unwrap().files_changed);
}

#[test]
fn composition_changes_keep_preexisting_wrapper_and_custom_ini_for_final_restore() {
    for bits in 1..=7 {
        let game = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let assets = tempfile::tempdir().unwrap();
        let original_ini = arachnid_mods::fix_settings::configure(None, &display()).unwrap();
        fs::write(game.path().join("d3d9.dll"), b"preexisting wrapper").unwrap();
        fs::write(game.path().join("d3d9.ini"), &original_ini).unwrap();
        let mut spec = synthetic_spec(7, assets.path());
        spec.original_wrapper = Some(hash(b"preexisting wrapper"));
        let mut initial =
            PackManager::new(game.path(), state.path(), &"a".repeat(64), spec).unwrap();
        let plan = initial
            .prepare(
                PackOperation::Enable,
                assets.path(),
                &display(),
                &"b".repeat(64),
            )
            .unwrap();
        initial.apply(&plan.digest, &display(), || Ok(())).unwrap();
        drop(initial);
        let mut next = PackManager::new(
            game.path(),
            state.path(),
            &"a".repeat(64),
            synthetic_spec(bits, assets.path()),
        )
        .unwrap();
        let plan = next
            .prepare(
                PackOperation::Enable,
                assets.path(),
                &display(),
                &"b".repeat(64),
            )
            .unwrap();
        next.apply(&plan.digest, &display(), || Ok(())).unwrap();
        let plan = next
            .prepare(
                PackOperation::Disable,
                assets.path(),
                &display(),
                &"b".repeat(64),
            )
            .unwrap();
        next.apply(&plan.digest, &display(), || Ok(())).unwrap();
        assert_eq!(
            fs::read(game.path().join("d3d9.dll")).unwrap(),
            b"preexisting wrapper"
        );
        assert_eq!(
            fs::read(game.path().join("d3d9.ini")).unwrap(),
            original_ini
        );
        assert!(!game.path().join("ar_dxvk.dll").exists());
    }
}
