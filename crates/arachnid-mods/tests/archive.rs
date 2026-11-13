#![cfg(windows)]

use arachnid_mods::{
    Artifact, Preimage, RecipeFile,
    archive::{extract_reshade_setup, extract_zip},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Write},
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn aligned_setup_overlay_is_read_as_data_with_the_full_source_hash_checked() {
    let source = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let mut bytes = vec![0x90; 1024];
    bytes[..2].copy_from_slice(b"MZ");
    bytes.extend(archive(&[("folder/fix.dll", b"synthetic")]));
    let path = source.path().join("synthetic-setup.exe");
    fs::write(&path, &bytes).unwrap();
    let prepared =
        extract_reshade_setup(&path, &artifact(&bytes), &[expected()], staging.path()).unwrap();
    assert_eq!(
        fs::read(prepared.file("d3d9.dll").unwrap()).unwrap(),
        b"synthetic"
    );
    drop(prepared);
    bytes[2] ^= 1;
    fs::write(&path, &bytes).unwrap();
    let mut incorrect = artifact(&bytes);
    incorrect.sha256 = "a".repeat(64);
    assert!(extract_reshade_setup(&path, &incorrect, &[expected()], staging.path()).is_err());
    let missing_overlay = vec![0; 1024];
    fs::write(&path, &missing_overlay).unwrap();
    assert!(
        extract_reshade_setup(
            &path,
            &artifact(&missing_overlay),
            &[expected()],
            staging.path()
        )
        .is_err()
    );
}

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        archive
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

fn artifact(bytes: &[u8]) -> Artifact {
    Artifact {
        url: "https://example.invalid/synthetic.zip".into(),
        sha256: digest(bytes),
        archive_bytes: bytes.len() as u64,
        extracted_bytes: 1024,
    }
}

fn expected() -> RecipeFile {
    RecipeFile {
        source: "folder/fix.dll".into(),
        target: "d3d9.dll".into(),
        sha256: digest(b"synthetic"),
        bytes: 9,
        preimage: Preimage::Absent,
    }
}

#[test]
fn private_extraction_verifies_content_and_cleans_up_on_drop() {
    let source = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let bytes = archive(&[("folder/fix.dll", b"synthetic"), ("readme.txt", b"fixture")]);
    let path = source.path().join("source.zip");
    fs::write(&path, &bytes).unwrap();
    let prepared = extract_zip(&path, &artifact(&bytes), &[expected()], staging.path()).unwrap();
    let output = prepared.file("d3d9.dll").unwrap();
    assert_eq!(fs::read(&output).unwrap(), b"synthetic");
    assert!(prepared.file("other.dll").is_none());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    drop(prepared);
    assert!(!output.exists());
    assert_eq!(fs::read_dir(staging.path()).unwrap().count(), 0);
}

#[test]
fn malicious_paths_are_refused_even_when_not_selected_for_extraction() {
    for name in [
        "../escape",
        "/absolute",
        "C:/escape",
        "bad:stream",
        "folder\\escape",
        "NUL.txt",
        "bad. /file",
        "a/b/c/d/e/f/g/h/i",
    ] {
        let source = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let bytes = archive(&[("folder/fix.dll", b"synthetic"), (name, b"hostile")]);
        let path = source.path().join("source.zip");
        fs::write(&path, &bytes).unwrap();
        assert!(
            extract_zip(&path, &artifact(&bytes), &[expected()], staging.path()).is_err(),
            "{name}"
        );
        assert_eq!(fs::read_dir(staging.path()).unwrap().count(), 0);
    }
}

#[test]
fn case_collisions_parent_files_and_symlinks_are_refused() {
    for collision in ["FOLDER/FIX.DLL", "folder"] {
        let source = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let bytes = archive(&[("folder/fix.dll", b"synthetic"), (collision, b"other")]);
        let path = source.path().join("source.zip");
        fs::write(&path, &bytes).unwrap();
        assert!(extract_zip(&path, &artifact(&bytes), &[expected()], staging.path()).is_err());
    }
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .add_symlink(
            "folder/fix.dll",
            "../../escape",
            SimpleFileOptions::default(),
        )
        .unwrap();
    let bytes = archive.finish().unwrap().into_inner();
    let source = tempfile::tempdir().unwrap();
    let path = source.path().join("source.zip");
    fs::write(&path, &bytes).unwrap();
    let staging = tempfile::tempdir().unwrap();
    assert!(extract_zip(&path, &artifact(&bytes), &[expected()], staging.path()).is_err());
}

#[test]
fn wrong_size_hash_and_expansion_budgets_leave_no_publication() {
    let source = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let bytes = archive(&[("folder/fix.dll", b"synthetic")]);
    let path = source.path().join("source.zip");
    fs::write(&path, &bytes).unwrap();
    let mut invalid = artifact(&bytes);
    invalid.sha256 = "f".repeat(64);
    assert!(extract_zip(&path, &invalid, &[expected()], staging.path()).is_err());
    invalid = artifact(&bytes);
    invalid.archive_bytes += 1;
    assert!(extract_zip(&path, &invalid, &[expected()], staging.path()).is_err());
    invalid = artifact(&bytes);
    invalid.extracted_bytes = 8;
    assert!(extract_zip(&path, &invalid, &[expected()], staging.path()).is_err());
    let mut file = expected();
    file.sha256 = "f".repeat(64);
    assert!(extract_zip(&path, &artifact(&bytes), &[file], staging.path()).is_err());
    assert_eq!(fs::read_dir(staging.path()).unwrap().count(), 0);
}
