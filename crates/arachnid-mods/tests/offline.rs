#![cfg(windows)]

use arachnid_mods::{
    bundled_catalog,
    offline::{Presence, bundled_index, parse_index},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;

fn index_value(bytes: &[u8]) -> Value {
    let catalog = bundled_catalog().unwrap();
    let mut value = serde_json::to_value(bundled_index(&catalog).unwrap()).unwrap();
    for (position, package) in value["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let hash: String = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        package["payload"] = json!({ "fileName": format!("fixture-{position}.bin"), "bytes": bytes.len(), "sha256": hash });
    }
    value
}

#[test]
fn missing_and_unprepared_packages_never_pass_the_offline_release_gate() {
    let catalog = bundled_catalog().unwrap();
    let index = bundled_index(&catalog).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let report = index.inspect(directory.path()).unwrap();
    assert!(!report.all_present);
    assert_eq!(report.packages[0].presence, Presence::Missing);
    assert_eq!(report.packages[3].presence, Presence::NotPrepared);
    assert_eq!(
        index
            .verify_release(&catalog, directory.path())
            .unwrap_err()
            .code,
        "offlinePackagesIncomplete"
    );
}

#[test]
fn local_staging_is_idempotent_and_packaging_requires_notices_and_recipes() {
    let catalog = bundled_catalog().unwrap();
    let index = parse_index(
        &serde_json::to_vec(&index_value(b"original bytes")).unwrap(),
        &catalog,
    )
    .unwrap();
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    for position in 0..5 {
        fs::write(
            source.path().join(format!("fixture-{position}.bin")),
            b"original bytes",
        )
        .unwrap();
    }
    let report = index
        .stage_available(source.path(), destination.path())
        .unwrap();
    assert!(report.all_present);
    assert!(
        report
            .packages
            .iter()
            .all(|p| p.presence == Presence::Verified)
    );
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 5);
    assert_eq!(
        index
            .verify_release(&catalog, destination.path())
            .unwrap_err()
            .code,
        "offlineNoticesIncomplete"
    );
    assert!(
        index
            .stage_available(source.path(), destination.path())
            .unwrap()
            .all_present
    );
    let mut value = index_value(b"original bytes");
    for (position, package) in value["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let mut notice = package["payload"].clone();
        notice["fileName"] = json!(format!("notice-{position}.txt"));
        package["notice"] = notice;
        fs::write(
            destination.path().join(format!("notice-{position}.txt")),
            b"original bytes",
        )
        .unwrap();
    }
    let index_with_notices = parse_index(&serde_json::to_vec(&value).unwrap(), &catalog).unwrap();
    assert_eq!(
        index_with_notices
            .verify_release(&catalog, destination.path())
            .unwrap_err()
            .code,
        "offlineRecipesUnqualified"
    );
}

#[test]
fn packaging_refuses_extra_files() {
    let catalog = bundled_catalog().unwrap();
    let index = parse_index(&serde_json::to_vec(&index_value(b"AAA")).unwrap(), &catalog).unwrap();
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("private-unlisted.txt"),
        b"unlisted fixture",
    )
    .unwrap();
    assert_eq!(
        index
            .verify_release(&catalog, directory.path())
            .unwrap_err()
            .code,
        "offlineUnexpectedFile"
    );
}

#[test]
fn same_size_corruption_truncation_and_preexisting_conflicts_preserve_files() {
    let catalog = bundled_catalog().unwrap();
    let index = parse_index(&serde_json::to_vec(&index_value(b"AAA")).unwrap(), &catalog).unwrap();
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(source.path().join("fixture-0.bin"), b"BBB").unwrap();
    assert_eq!(
        index.inspect(source.path()).unwrap().packages[0].presence,
        Presence::Invalid
    );
    assert!(
        index
            .stage_available(source.path(), destination.path())
            .is_err()
    );
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 0);
    fs::write(source.path().join("fixture-0.bin"), b"AA").unwrap();
    assert!(
        index
            .stage_available(source.path(), destination.path())
            .is_err()
    );
    fs::write(source.path().join("fixture-0.bin"), b"AAA").unwrap();
    fs::write(destination.path().join("fixture-0.bin"), b"BBB").unwrap();
    assert!(
        index
            .stage_available(source.path(), destination.path())
            .is_err()
    );
    assert_eq!(
        fs::read(destination.path().join("fixture-0.bin")).unwrap(),
        b"BBB"
    );
}

#[test]
fn invalid_indexes_cannot_add_network_paths_unbounded_payloads_or_unknown_entries() {
    let catalog = bundled_catalog().unwrap();
    for bad in [
        json!("../escape.bin"),
        json!("C:/escape"),
        json!("file.bin:stream"),
        json!("\\\\host\\file"),
        json!("NUL"),
        json!("folder/file.bin"),
    ] {
        let mut value = index_value(b"AAA");
        value["packages"][0]["payload"]["fileName"] = bad;
        assert!(parse_index(&serde_json::to_vec(&value).unwrap(), &catalog).is_err());
    }
    let mut value = index_value(b"AAA");
    value["packages"][0]["payload"]["bytes"] = json!(134217729);
    assert!(parse_index(&serde_json::to_vec(&value).unwrap(), &catalog).is_err());
    let mut value = index_value(b"AAA");
    value["packages"][0]["payload"]["sha256"] = json!("unknown");
    assert!(parse_index(&serde_json::to_vec(&value).unwrap(), &catalog).is_err());
    let mut value = index_value(b"AAA");
    value["packages"][0]["id"] = json!("removed-tool");
    assert!(parse_index(&serde_json::to_vec(&value).unwrap(), &catalog).is_err());
    let mut value = index_value(b"AAA");
    value["packages"][1]["payload"]["fileName"] = json!("FIXTURE-0.BIN");
    assert!(parse_index(&serde_json::to_vec(&value).unwrap(), &catalog).is_err());
}
