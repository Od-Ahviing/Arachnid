#![cfg(windows)]

use arachnid_mods::payloads::{MAX_PAYLOAD_BYTES, Payload, PayloadResources};
use arachnid_repair::{
    Action, Journal, OperationPlan, Phase, Resources, Target, apply, prepare, rollback_guarded,
};
use sha2::{Digest, Sha256};
use std::{fs, sync::atomic::AtomicBool};

fn payload(bytes: &[u8]) -> Payload {
    Payload {
        sha256: Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        bytes: bytes.len() as u64,
    }
}

#[test]
fn multi_megabyte_group_commits_and_restores_without_inline_binary_journals() {
    let game = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let journal_directory = tempfile::tempdir().unwrap();
    let path = journal_directory.path().join("mods.json");
    let original = vec![3u8; 2 * 1024 * 1024];
    fs::write(game.path().join("d3d9.dll"), &original).unwrap();
    let mut resources = PayloadResources::new(game.path(), store.path()).unwrap();
    let mut actions = Vec::new();
    for (index, name) in ["d3d9.dll", "dxvk.dll", "ReShade32.dll"].iter().enumerate() {
        let bytes = vec![index as u8; 4 * 1024 * 1024 + 17];
        let source = sources.path().join(name);
        fs::write(&source, &bytes).unwrap();
        let target = Target::File {
            path: name.to_string(),
        };
        actions.push(Action {
            before: resources.read(&target).unwrap(),
            target,
            after: Some(resources.import(&source, &payload(&bytes)).unwrap()),
        });
    }
    let plan = OperationPlan {
        schema_version: 1,
        installation: "synthetic-originals-only".into(),
        actions,
    };
    PayloadResources::validate_plan(&plan).unwrap();
    let mut journal = prepare(plan, &resources, &path).unwrap();
    assert!(fs::metadata(&path).unwrap().len() < 4096);
    let digest = journal.digest.clone();
    apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || Ok(()),
    )
    .unwrap();
    assert_eq!(journal.phase, Phase::Committed);
    drop(resources);
    let mut reopened = PayloadResources::new(game.path(), store.path()).unwrap();
    let mut journal = Journal::load(&path).unwrap();
    rollback_guarded(&mut journal, &mut reopened, &path, || Ok(())).unwrap();
    assert_eq!(fs::read(game.path().join("d3d9.dll")).unwrap(), original);
    assert!(!game.path().join("dxvk.dll").exists());
    assert!(!game.path().join("ReShade32.dll").exists());
}

#[test]
fn altered_staged_payload_fails_before_publication() {
    let game = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let journal_directory = tempfile::tempdir().unwrap();
    let path = journal_directory.path().join("mods.json");
    fs::write(game.path().join("d3d9.dll"), b"original").unwrap();
    let mut resources = PayloadResources::new(game.path(), store.path()).unwrap();
    let target = Target::File {
        path: "d3d9.dll".into(),
    };
    let bytes = b"replacement";
    let descriptor = payload(bytes);
    fs::write(source.path().join("fixture"), bytes).unwrap();
    let after = resources
        .import(&source.path().join("fixture"), &descriptor)
        .unwrap();
    let plan = OperationPlan {
        schema_version: 1,
        installation: "synthetic".into(),
        actions: vec![Action {
            before: resources.read(&target).unwrap(),
            target,
            after: Some(after),
        }],
    };
    let mut journal = prepare(plan, &resources, &path).unwrap();
    fs::write(
        store.path().join(format!("{}.blob", descriptor.sha256)),
        b"corruption!",
    )
    .unwrap();
    let digest = journal.digest.clone();
    assert!(
        apply(
            &mut journal,
            &digest,
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .is_err()
    );
    assert_eq!(fs::read(game.path().join("d3d9.dll")).unwrap(), b"original");
    assert_eq!(journal.phase, Phase::RolledBack);
}

#[test]
fn external_changes_and_interrupted_group_are_recovered_conservatively() {
    let game = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let journal_directory = tempfile::tempdir().unwrap();
    let path = journal_directory.path().join("mods.json");
    let mut resources = PayloadResources::new(game.path(), store.path()).unwrap();
    let target = Target::File {
        path: "d3d9.dll".into(),
    };
    fs::write(source.path().join("fixture"), b"replacement").unwrap();
    let after = resources
        .import(&source.path().join("fixture"), &payload(b"replacement"))
        .unwrap();
    let plan = OperationPlan {
        schema_version: 1,
        installation: "synthetic".into(),
        actions: vec![Action {
            before: None,
            target: target.clone(),
            after: Some(after.clone()),
        }],
    };
    let mut journal = prepare(plan, &resources, &path).unwrap();
    journal.phase = Phase::Applying;
    journal.intent_count = 1;
    journal.save(&path).unwrap();
    resources.write(&target, &Some(after)).unwrap();
    let mut reopened = Journal::load(&path).unwrap();
    rollback_guarded(&mut reopened, &mut resources, &path, || Ok(())).unwrap();
    assert!(!game.path().join("d3d9.dll").exists());
    fs::write(game.path().join("d3d9.dll"), b"external").unwrap();
    assert!(rollback_guarded(&mut journal, &mut resources, &path, || Ok(())).is_err());
    assert_eq!(journal.phase, Phase::RecoveryRequired);
    assert_eq!(fs::read(game.path().join("d3d9.dll")).unwrap(), b"external");
}

#[test]
fn wrong_hash_excessive_file_size_and_overlapping_roots_are_refused() {
    let game = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let resources = PayloadResources::new(game.path(), store.path()).unwrap();
    fs::write(source.path().join("fixture"), b"bad").unwrap();
    assert!(
        resources
            .import(&source.path().join("fixture"), &payload(b"good"))
            .is_err()
    );
    assert_eq!(fs::read_dir(store.path()).unwrap().count(), 0);
    let file = fs::File::create(source.path().join("large")).unwrap();
    file.set_len(MAX_PAYLOAD_BYTES + 1).unwrap();
    drop(file);
    assert!(
        resources
            .import(
                &source.path().join("large"),
                &Payload {
                    sha256: "a".repeat(64),
                    bytes: MAX_PAYLOAD_BYTES + 1
                }
            )
            .is_err()
    );
    assert!(PayloadResources::new(game.path(), game.path()).is_err());
}
