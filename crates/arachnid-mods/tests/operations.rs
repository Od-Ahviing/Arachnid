#![cfg(windows)]

use arachnid_mods::{
    Artifact, Compatibility, License, Preimage, RecipeFile, Removal, archive::extract_zip,
    bundled_catalog, operations::plan_installation, payloads::PayloadResources,
    selection::Selection,
};
use arachnid_repair::{apply, prepare};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Cursor, Write},
    sync::atomic::AtomicBool,
};
use zip::{ZipWriter, write::SimpleFileOptions};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn fixture() -> (arachnid_mods::Catalog, Vec<u8>) {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("fixture.dll", SimpleFileOptions::default())
        .unwrap();
    writer
        .write_all(b"original Arachnid fixture; not a DLL")
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let mut catalog = bundled_catalog().unwrap();
    catalog.groups.clear();
    catalog.entries.truncate(1);
    let recipe = &mut catalog.entries[0];
    recipe.id = "synthetic-fixture".into();
    recipe.name = "Arachnid fixture only".into();
    recipe.author = "Arachnid tests".into();
    recipe.source = "https://example.invalid/fixture".into();
    recipe.version = Some("synthetic-1".into());
    recipe.default_enabled = true;
    recipe.license = Some(License {
        name: "Synthetic fixture".into(),
        source: "https://example.invalid/license".into(),
    });
    recipe.artifact = Some(Artifact {
        url: "https://example.invalid/fixture.zip".into(),
        sha256: digest(&bytes),
        archive_bytes: bytes.len() as u64,
        extracted_bytes: 1024,
    });
    recipe.compatibility = vec![Compatibility {
        game_sha256: "b".repeat(64),
        evidence: "Synthetic fixture only".into(),
    }];
    recipe.files = vec![RecipeFile {
        source: "fixture.dll".into(),
        target: "d3d9.dll".into(),
        sha256: digest(b"original Arachnid fixture; not a DLL"),
        bytes: b"original Arachnid fixture; not a DLL".len() as u64,
        preimage: Preimage::Absent,
    }];
    recipe.conflicts.clear();
    recipe.removal = Some(Removal::RestorePreimages);
    (catalog, bytes)
}

#[test]
fn exact_qualified_recipe_prepares_off_game_and_rechecks_consent_and_preimages() {
    let game = tempfile::tempdir().unwrap();
    let blobs = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let journal_directory = tempfile::tempdir().unwrap();
    let (catalog, bytes) = fixture();
    let recipe = &catalog.entries[0];
    let path = source.path().join("fixture.zip");
    fs::write(&path, bytes).unwrap();
    let archive = extract_zip(
        &path,
        recipe.artifact.as_ref().unwrap(),
        &recipe.files,
        staging.path(),
    )
    .unwrap();
    let mut archives = BTreeMap::new();
    archives.insert(recipe.id.clone(), archive);
    let mut resources = PayloadResources::new(game.path(), blobs.path()).unwrap();
    let selection = Selection::defaults(&catalog);
    let plan = plan_installation(
        &catalog,
        &selection,
        &"b".repeat(64),
        &"c".repeat(64),
        &resources,
        &archives,
    )
    .unwrap();
    assert!(!game.path().join("d3d9.dll").exists());
    assert_eq!(plan.actions.len(), 1);
    let path = journal_directory.path().join("mods.json");
    let mut journal = prepare(plan, &resources, &path).unwrap();
    assert!(
        apply(
            &mut journal,
            "wrong consent",
            &mut resources,
            &path,
            &AtomicBool::new(false),
            || Ok(())
        )
        .is_err()
    );
    fs::write(game.path().join("d3d9.dll"), b"external content").unwrap();
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
    assert_eq!(
        fs::read(game.path().join("d3d9.dll")).unwrap(),
        b"external content"
    );
}

#[test]
fn unqualified_and_wrong_build_recipes_cannot_prepare_game_actions() {
    let game = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let resources = PayloadResources::new(game.path(), store.path()).unwrap();
    let (mut catalog, _) = fixture();
    let selection = Selection::defaults(&catalog);
    assert_eq!(
        plan_installation(
            &catalog,
            &selection,
            &"a".repeat(64),
            &"c".repeat(64),
            &resources,
            &BTreeMap::new()
        )
        .unwrap_err()
        .code,
        "modBuildUnsupported"
    );
    catalog.entries[0].license = None;
    assert_eq!(
        plan_installation(
            &catalog,
            &selection,
            &"b".repeat(64),
            &"c".repeat(64),
            &resources,
            &BTreeMap::new()
        )
        .unwrap_err()
        .code,
        "modRecipeUnqualified"
    );
    assert_eq!(fs::read_dir(store.path()).unwrap().count(), 0);
    assert_eq!(fs::read_dir(game.path()).unwrap().count(), 0);
}
