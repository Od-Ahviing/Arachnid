#![cfg(windows)]
use arachnid_saveguard::discovery::{Discovery, SAVE_NAME};
use std::{fs, path::PathBuf};

struct Fixture {
    _root: tempfile::TempDir,
    documents: PathBuf,
    roaming: PathBuf,
    installation: PathBuf,
    backups: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let documents = root.path().join("Documents");
        let roaming = root.path().join("Roaming");
        let installation = root.path().join("game");
        let backups = root.path().join("backups");
        for path in [&documents, &roaming, &installation, &backups] {
            fs::create_dir(path).unwrap();
        }
        fs::create_dir(installation.join("steam_settings")).unwrap();
        Self {
            _root: root,
            documents,
            roaming,
            installation,
            backups,
        }
    }
    fn gse(&self) -> PathBuf {
        self.roaming.join("GSE Saves/212580/remote")
    }
    fn native(&self) -> PathBuf {
        self.documents.join("Activision/ASM/Save")
    }
    fn detect(&self) -> arachnid_core::AppResult<Option<arachnid_saveguard::SourceProfile>> {
        Discovery::new(&self.documents, &self.roaming, Some(&self.installation))?
            .find(&self.backups)
    }
    fn provider(&self, id: &str) {
        fs::write(self.installation.join("steam_settings/steam_appid.txt"), id).unwrap();
        let mut bytes = vec![0; 8197];
        bytes.extend_from_slice(b"GSE Saves");
        fs::write(self.installation.join("steam_api.dll"), bytes).unwrap();
    }
}

#[test]
fn existing_gse_game_save_is_selected_over_empty_or_stale_documents() {
    let fixture = Fixture::new();
    fixture.provider("212580\r\n");
    fs::create_dir_all(fixture.native()).unwrap();
    fs::create_dir_all(fixture.gse()).unwrap();
    fs::write(fixture.gse().join(SAVE_NAME), b"current synthetic save").unwrap();
    let profile = fixture.detect().unwrap().unwrap();
    assert_eq!(profile.source, fs::canonicalize(fixture.gse()).unwrap());
    assert!(profile.enabled && profile.automatic);
    fs::write(fixture.native().join(SAVE_NAME), b"stale native save").unwrap();
    assert_eq!(fixture.detect().unwrap().unwrap().source, profile.source);
    assert_eq!(
        fs::read(fixture.gse().join(SAVE_NAME)).unwrap(),
        b"current synthetic save"
    );
}

#[test]
fn empty_or_unrelated_files_never_establish_a_save_location() {
    let fixture = Fixture::new();
    fixture.provider("212580");
    fs::create_dir_all(fixture.native()).unwrap();
    fs::create_dir_all(fixture.gse()).unwrap();
    fs::write(fixture.gse().join("unrelated.json"), b"metadata").unwrap();
    assert!(fixture.detect().unwrap().is_none());
    fs::write(fixture.gse().join(SAVE_NAME), b"").unwrap();
    assert!(fixture.detect().unwrap().is_none());
}

#[test]
fn original_provider_or_wrong_app_id_never_selects_an_unrelated_gse_save() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.native()).unwrap();
    fs::create_dir_all(fixture.gse()).unwrap();
    fs::write(fixture.gse().join(SAVE_NAME), b"other installation").unwrap();
    assert!(fixture.detect().unwrap().is_none());
    fixture.provider("999999");
    assert!(fixture.detect().unwrap().is_none());
    fs::write(fixture.native().join(SAVE_NAME), b"native save").unwrap();
    assert_eq!(
        fixture.detect().unwrap().unwrap().source,
        fs::canonicalize(fixture.native()).unwrap()
    );
    fixture.provider("212580");
    fs::write(
        fixture.installation.join("steam_api.dll"),
        b"original fixture without provider marker",
    )
    .unwrap();
    assert_eq!(
        fixture.detect().unwrap().unwrap().source,
        fs::canonicalize(fixture.native()).unwrap()
    );
}

#[test]
fn prepared_detection_reconciles_a_later_first_save_without_rereading_the_dll() {
    let fixture = Fixture::new();
    fixture.provider("212580");
    let detection = Discovery::new(
        &fixture.documents,
        &fixture.roaming,
        Some(&fixture.installation),
    )
    .unwrap();
    assert!(detection.find(&fixture.backups).unwrap().is_none());
    fs::remove_file(fixture.installation.join("steam_api.dll")).unwrap();
    fs::create_dir_all(fixture.gse()).unwrap();
    fs::write(fixture.gse().join(SAVE_NAME), b"first save").unwrap();
    assert!(detection.find(&fixture.backups).unwrap().is_some());
    assert!(!detection.matches_source(&fixture.native()));
    assert!(detection.matches_source(&fs::canonicalize(fixture.gse()).unwrap()));
}
