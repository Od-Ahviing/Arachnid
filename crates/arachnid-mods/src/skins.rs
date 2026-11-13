use crate::{
    offline::Payload as SourcePayload,
    payloads::{Payload, PayloadResources},
    performance,
};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::{
    Action, Image, Journal, OperationPlan, Phase, Resources, Target, filesystem::DirectoryGuard,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

const NAMES: [&str; 3] = ["dinput8.dll", "arachnid_skin.asi", "retex.ini"];
pub const RESOURCE_DIRECTORY: &str = "skins-preview";
pub const RUNTIME_QUALIFIED: bool = true;

fn legacy_loaders() -> Vec<SourcePayload> {
    vec![
        SourcePayload {
            file_name: "dinput8.dll".into(),
            bytes: 1_192_448,
            sha256: "326acf0d3acecfae3450529adbea562406aae3df1a3e1a3c0ef21734d090e145".into(),
        },
        SourcePayload {
            file_name: "arachnid_skin.asi".into(),
            bytes: 327_680,
            sha256: "0b51d4f01707362b61f87cabbbc1f32d1fdd9d9647d8ffdba27a1c0919ff036b".into(),
        },
    ]
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub author: String,
    pub file_name: String,
    pub bytes: u64,
    pub sha256: String,
    pub archive_path: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Bundle {
    schema_version: u32,
    archive_sha256: String,
    source: String,
    loaders: Vec<SourcePayload>,
    skins: Vec<Skin>,
}

fn bundle() -> AppResult<Bundle> {
    let bundle: Bundle = serde_json::from_str(include_str!("../../../manifests/skins.json"))
        .map_err(|_| invalid())?;
    let mut ids = BTreeSet::new();
    let mut files = BTreeSet::new();
    if bundle.schema_version != 1
        || !crate::hash(&bundle.archive_sha256)
        || !crate::https(&bundle.source)
        || bundle.loaders.len() != 2
        || bundle.skins.is_empty()
        || bundle.skins.len() > 64
    {
        return Err(invalid());
    }
    for (loader, name) in bundle.loaders.iter().zip(NAMES) {
        if loader.file_name != name
            || !crate::hash(&loader.sha256)
            || loader.bytes == 0
            || loader.bytes > crate::payloads::MAX_PAYLOAD_BYTES
        {
            return Err(invalid());
        }
    }
    for skin in &bundle.skins {
        if !crate::identifier(&skin.id)
            || !ids.insert(&skin.id)
            || !crate::text(&skin.name, 160)
            || !crate::text(&skin.author, 160)
            || !crate::relative_path(&skin.file_name)
            || skin.file_name.contains('/')
            || !skin.file_name.ends_with(".tpf")
            || !files.insert(skin.file_name.to_ascii_lowercase())
            || !crate::hash(&skin.sha256)
            || skin.bytes == 0
            || skin.bytes > 80 * 1024 * 1024
            || !crate::text(&skin.archive_path, 512)
        {
            return Err(invalid());
        }
    }
    Ok(bundle)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    domain: String,
    previous: Option<String>,
    next: Option<String>,
    #[serde(default)]
    previous_revision: u8,
    #[serde(default)]
    next_revision: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinPreview {
    pub digest: String,
    pub selected: Option<String>,
    pub phase: Phase,
    pub files: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinStatus {
    pub skins: Vec<Skin>,
    pub selected: Option<String>,
    pub available: bool,
    pub files_changed: bool,
    pub recovery_required: bool,
    pub requires_update: bool,
    pub preview: Option<SkinPreview>,
}

pub struct SkinManager {
    root: PathBuf,
    state: PathBuf,
    assets: PathBuf,
    installation: String,
    resources: PayloadResources,
    bundle: Bundle,
    _guard: DirectoryGuard,
}

impl SkinManager {
    pub fn new(root: &Path, state: &Path, assets: &Path, installation: &str) -> AppResult<Self> {
        Self::with_bundle(root, state, assets, installation, bundle()?)
    }

    fn with_bundle(
        root: &Path,
        state: &Path,
        assets: &Path,
        installation: &str,
        bundle: Bundle,
    ) -> AppResult<Self> {
        if !crate::hash(installation) {
            return Err(invalid());
        }
        let guard = DirectoryGuard::acquire(state)?;
        let blobs = state.join("blobs");
        match fs::create_dir(&blobs) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(invalid()),
        }
        Ok(Self {
            root: root.into(),
            state: state.into(),
            assets: assets.into(),
            installation: installation.into(),
            resources: PayloadResources::new(root, &blobs)?,
            bundle,
            _guard: guard,
        })
    }

    fn path(&self) -> PathBuf {
        self.state.join("skin-transaction.json")
    }

    fn config(&self, id: &str) -> AppResult<Vec<u8>> {
        let bundle = &self.bundle;
        let skin = bundle
            .skins
            .iter()
            .find(|skin| skin.id == id)
            .ok_or_else(invalid)?;
        let path = std::path::absolute(self.assets.join(&skin.file_name)).map_err(|_| invalid())?;
        let text = path.to_str().ok_or_else(invalid)?;
        if text.contains(['\r', '\n', '"']) || text.trim() != text {
            return Err(invalid());
        }
        Ok(format!("pack={text}\n").into_bytes())
    }

    fn loaders(&self, revision: u8) -> AppResult<Vec<SourcePayload>> {
        match revision {
            1 => Ok(self.bundle.loaders.clone()),
            0 => Ok(legacy_loaders()),
            _ => Err(invalid()),
        }
    }

    fn expected(&self, selected: Option<&str>, revision: u8) -> AppResult<Vec<Option<Image>>> {
        let loaders = self.loaders(revision)?;
        let Some(id) = selected else {
            return Ok(vec![None; 3]);
        };
        let mut files = loaders
            .iter()
            .map(|file| {
                Payload {
                    sha256: file.sha256.clone(),
                    bytes: file.bytes,
                }
                .image()
                .map(Some)
            })
            .collect::<AppResult<Vec<_>>>()?;
        let config = self.config(id)?;
        files.push(Some(
            Payload {
                sha256: crate::fix_adapter::digest(&config),
                bytes: config.len() as u64,
            }
            .image()?,
        ));
        Ok(files)
    }

    fn journal(&self) -> AppResult<Option<(Journal, Context)>> {
        if !self.path().try_exists().map_err(|_| invalid())? {
            return Ok(None);
        }
        let journal = Journal::load(&self.path())?;
        let context: Context = serde_json::from_value(journal.context.clone().ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        if journal.plan.installation != self.installation || context.domain != "arachnid-skins-v1" {
            return Err(invalid());
        }
        let previous = self.expected(context.previous.as_deref(), context.previous_revision)?;
        let next = self.expected(context.next.as_deref(), context.next_revision)?;
        let actions = NAMES
            .iter()
            .zip(previous)
            .zip(next)
            .filter_map(|((name, before), after)| {
                (before != after).then(|| Action {
                    target: Target::File {
                        path: (*name).into(),
                    },
                    before,
                    after,
                })
            })
            .collect::<Vec<_>>();
        if journal.plan.actions != actions || actions.is_empty() {
            return Err(invalid());
        }
        Ok(Some((journal, context)))
    }

    fn selected(&self) -> AppResult<Option<String>> {
        Ok(self.journal()?.and_then(|(journal, context)| {
            if journal.phase == Phase::Committed {
                context.next
            } else {
                context.previous
            }
        }))
    }

    fn check_files(&self, selected: Option<&str>) -> AppResult<()> {
        for (name, expected) in NAMES.iter().zip(self.expected(selected, self.revision()?)?) {
            if self.resources.read(&Target::File {
                path: (*name).into(),
            })? != expected
            {
                return Err(AppError::new("modOwnedFileChanged"));
            }
        }
        Ok(())
    }

    fn revision(&self) -> AppResult<u8> {
        Ok(self.journal()?.map_or(1, |(journal, context)| {
            if journal.phase == Phase::Committed {
                context.next_revision
            } else {
                context.previous_revision
            }
        }))
    }

    fn source_files(&self, id: &str) -> AppResult<Vec<File>> {
        let bundle = &self.bundle;
        let skin = bundle
            .skins
            .iter()
            .find(|skin| skin.id == id)
            .ok_or_else(invalid)?;
        let mut descriptors = bundle.loaders.clone();
        descriptors.push(SourcePayload {
            file_name: skin.file_name.clone(),
            bytes: skin.bytes,
            sha256: skin.sha256.clone(),
        });
        performance::verified_files(&self.assets, &descriptors)
    }

    pub fn status(&self) -> AppResult<SkinStatus> {
        let bundle = &self.bundle;
        let journal = self.journal()?;
        let recovery_required = journal
            .as_ref()
            .is_some_and(|(journal, _)| !journal.terminal() && journal.phase != Phase::Prepared);
        let selected = self.selected()?;
        let files_changed = !recovery_required && self.check_files(selected.as_deref()).is_err();
        let available = (cfg!(debug_assertions) || RUNTIME_QUALIFIED)
            && performance::verified_files(&self.assets, &bundle.loaders).is_ok()
            && bundle.skins.iter().all(|skin| {
                fs::metadata(self.assets.join(&skin.file_name))
                    .is_ok_and(|m| m.is_file() && m.len() == skin.bytes)
            });
        let preview = journal.map(|(journal, context)| SkinPreview {
            digest: journal.digest,
            selected: context.next,
            phase: journal.phase,
            files: journal
                .plan
                .actions
                .iter()
                .filter_map(|action| {
                    if let Target::File { path } = &action.target {
                        Some(path.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        });
        Ok(SkinStatus {
            requires_update: selected.is_some() && self.revision()? != 1,
            skins: bundle.skins.clone(),
            selected,
            available,
            files_changed,
            recovery_required,
            preview,
        })
    }

    pub fn prepare(&self, selected: Option<String>, game_sha256: &str) -> AppResult<SkinPreview> {
        if game_sha256 != performance::GAME_SHA256 {
            return Err(AppError::new("modBuildUnsupported"));
        }
        let status = self.status()?;
        if status.recovery_required {
            return Err(AppError::new("recoveryRequired"));
        }
        if selected.is_some() && !status.available {
            return Err(AppError::new("modPayloadMissing"));
        }
        self.check_files(status.selected.as_deref())?;
        if let Some(id) = &selected {
            let _source = self.source_files(id)?;
            for descriptor in &self.bundle.loaders {
                self.resources.import(
                    &self.assets.join(&descriptor.file_name),
                    &Payload {
                        sha256: descriptor.sha256.clone(),
                        bytes: descriptor.bytes,
                    },
                )?;
            }
            let config = self.config(id)?;
            let file = tempfile::NamedTempFile::new_in(&self.state).map_err(|_| invalid())?;
            fs::write(file.path(), &config).map_err(|_| invalid())?;
            let path = file.into_temp_path();
            self.resources.import(
                &path,
                &Payload {
                    sha256: crate::fix_adapter::digest(&config),
                    bytes: config.len() as u64,
                },
            )?;
        }
        let previous_revision = self.revision()?;
        let before = self.expected(status.selected.as_deref(), previous_revision)?;
        let after = self.expected(selected.as_deref(), 1)?;
        let actions = NAMES
            .iter()
            .zip(before)
            .zip(after)
            .filter_map(|((name, before), after)| {
                (before != after).then(|| Action {
                    target: Target::File {
                        path: (*name).into(),
                    },
                    before,
                    after,
                })
            })
            .collect::<Vec<_>>();
        if actions.is_empty() {
            return Err(AppError::new("modAlreadyPresent"));
        }
        let context = Context {
            previous_revision,
            next_revision: 1,
            domain: "arachnid-skins-v1".into(),
            previous: status.selected,
            next: selected.clone(),
        };
        if let Some((mut old, _)) = self.journal()?
            && old.phase == Phase::Prepared
        {
            old.phase = Phase::Cancelled;
            old.save(&self.path())?;
        }
        let journal = arachnid_repair::prepare_with_context(
            OperationPlan {
                schema_version: 1,
                installation: self.installation.clone(),
                actions,
            },
            &self.resources,
            &self.path(),
            Some(serde_json::to_value(context).map_err(|_| invalid())?),
        )?;
        Ok(SkinPreview {
            digest: journal.digest,
            selected,
            phase: journal.phase,
            files: journal
                .plan
                .actions
                .iter()
                .filter_map(|action| {
                    if let Target::File { path } = &action.target {
                        Some(path.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        })
    }

    pub fn apply(
        &mut self,
        digest: &str,
        recover: bool,
        gate: impl Fn() -> AppResult<()>,
    ) -> AppResult<()> {
        let (mut journal, context) = self.journal()?.ok_or_else(invalid)?;
        if journal.digest != digest {
            return Err(AppError::new("approvalInvalid"));
        }
        if recover {
            if journal.terminal() || journal.phase == Phase::Prepared {
                return Err(invalid());
            }
            let path = self.path();
            return arachnid_repair::rollback_guarded(
                &mut journal,
                &mut self.resources,
                &path,
                gate,
            );
        }
        let _source = context
            .next
            .as_ref()
            .map(|id| self.source_files(id))
            .transpose()?;
        let path = self.path();
        arachnid_repair::apply(
            &mut journal,
            digest,
            &mut self.resources,
            &path,
            &AtomicBool::new(false),
            gate,
        )
    }

    pub fn runtime_files(&self) -> AppResult<Option<Vec<File>>> {
        let status = self.status()?;
        if status.recovery_required {
            return Err(AppError::new("recoveryRequired"));
        }
        if status.requires_update {
            return Err(AppError::new("skinLoaderUpdateRequired"));
        }
        self.check_files(status.selected.as_deref())?;
        let Some(id) = status.selected else {
            return Ok(None);
        };
        if !status.available {
            return Err(AppError::new("modPayloadMissing"));
        }
        let mut files = self.source_files(&id)?;
        let config = self.config(&id)?;
        let mut descriptors = self.bundle.loaders.clone();
        descriptors.push(SourcePayload {
            file_name: "retex.ini".into(),
            bytes: config.len() as u64,
            sha256: crate::fix_adapter::digest(&config),
        });
        files.extend(performance::verified_files(&self.root, &descriptors)?);
        Ok(Some(files))
    }
}

pub fn managed_loader(game: &Path) -> bool {
    bundle().is_ok_and(|bundle| performance::verified_files(game, &bundle.loaders).is_ok())
        || performance::verified_files(game, &legacy_loaders()).is_ok()
}

pub fn verify_release(assets: &Path) -> AppResult<()> {
    if !RUNTIME_QUALIFIED {
        return Err(AppError::new("skinBundleUnqualified"));
    }
    verify_prepared(assets)
}

pub fn verify_prepared(assets: &Path) -> AppResult<()> {
    let bundle = bundle()?;
    let mut files = bundle.loaders;
    files.extend(bundle.skins.into_iter().map(|skin| SourcePayload {
        file_name: skin.file_name,
        bytes: skin.bytes,
        sha256: skin.sha256,
    }));
    let mut names: Vec<_> = files.iter().map(|file| file.file_name.as_str()).collect();
    names.push("notices");
    performance::exact_directory(assets, &names)?;
    performance::verified_files(assets, &files)?;
    Ok(())
}

fn invalid() -> AppError {
    AppError::new("invalidSkinState")
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    struct Fixture {
        game: tempfile::TempDir,
        state: tempfile::TempDir,
        assets: tempfile::TempDir,
        bundle: Bundle,
    }
    impl Fixture {
        fn new() -> Self {
            let game = tempfile::tempdir().unwrap();
            let state = tempfile::tempdir().unwrap();
            let assets = tempfile::tempdir().unwrap();
            let mut bundle = bundle().unwrap();
            for loader in &mut bundle.loaders {
                let bytes = format!("original synthetic {}", loader.file_name).into_bytes();
                fs::write(assets.path().join(&loader.file_name), &bytes).unwrap();
                loader.bytes = bytes.len() as u64;
                loader.sha256 = crate::fix_adapter::digest(&bytes);
            }
            bundle.skins.truncate(2);
            for (index, skin) in bundle.skins.iter_mut().enumerate() {
                skin.id = format!("skin-{:02}", index + 1);
                let bytes = format!("original synthetic {}", skin.id).into_bytes();
                fs::write(assets.path().join(&skin.file_name), &bytes).unwrap();
                skin.bytes = bytes.len() as u64;
                skin.sha256 = crate::fix_adapter::digest(&bytes);
            }
            Self {
                game,
                state,
                assets,
                bundle,
            }
        }
        fn manager(&self) -> SkinManager {
            SkinManager::with_bundle(
                self.game.path(),
                self.state.path(),
                self.assets.path(),
                &"a".repeat(64),
                self.bundle.clone(),
            )
            .unwrap()
        }
    }

    #[test]
    fn activation_switch_and_removal_are_exact_and_do_not_touch_graphics_mods() {
        let fixture = Fixture::new();
        fs::write(
            fixture.game.path().join("d3d9.dll"),
            b"personal graphics wrapper",
        )
        .unwrap();
        let mut manager = fixture.manager();
        assert!(manager.status().unwrap().selected.is_none());
        let preview = manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        assert!(!fixture.game.path().join("dinput8.dll").exists());
        assert!(manager.apply("wrong approval", false, || Ok(())).is_err());
        manager.apply(&preview.digest, false, || Ok(())).unwrap();
        assert_eq!(
            manager.status().unwrap().selected.as_deref(),
            Some("skin-01")
        );
        assert!(manager.runtime_files().unwrap().is_some());
        let preview = manager
            .prepare(Some("skin-02".into()), performance::GAME_SHA256)
            .unwrap();
        assert_eq!(preview.files, vec!["retex.ini"]);
        manager.apply(&preview.digest, false, || Ok(())).unwrap();
        assert!(
            fs::read_to_string(fixture.game.path().join("retex.ini"))
                .unwrap()
                .contains("skin-02.tpf")
        );
        let preview = manager.prepare(None, performance::GAME_SHA256).unwrap();
        manager.apply(&preview.digest, false, || Ok(())).unwrap();
        for name in NAMES {
            assert!(!fixture.game.path().join(name).exists());
        }
        assert!(manager.runtime_files().unwrap().is_none());
        assert_eq!(
            fs::read(fixture.game.path().join("d3d9.dll")).unwrap(),
            b"personal graphics wrapper"
        );
    }

    #[test]
    fn foreign_files_changed_payload_and_external_edits_are_preserved() {
        let fixture = Fixture::new();
        fs::write(fixture.game.path().join("dinput8.dll"), b"foreign loader").unwrap();
        let manager = fixture.manager();
        assert!(
            manager
                .prepare(Some("skin-01".into()), performance::GAME_SHA256)
                .is_err()
        );
        assert_eq!(
            fs::read(fixture.game.path().join("dinput8.dll")).unwrap(),
            b"foreign loader"
        );
        drop(manager);
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let preview = manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        fs::write(
            fixture.assets.path().join("skin-01.tpf"),
            b"modified package bytes",
        )
        .unwrap();
        assert!(manager.apply(&preview.digest, false, || Ok(())).is_err());
        assert!(!fixture.game.path().join("dinput8.dll").exists());
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let preview = manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        manager.apply(&preview.digest, false, || Ok(())).unwrap();
        fs::write(fixture.game.path().join("retex.ini"), b"personal edits").unwrap();
        assert!(manager.status().unwrap().files_changed);
        assert!(manager.runtime_files().is_err());
        assert!(manager.prepare(None, performance::GAME_SHA256).is_err());
        assert_eq!(
            fs::read(fixture.game.path().join("retex.ini")).unwrap(),
            b"personal edits"
        );
    }

    #[test]
    fn interrupted_application_recovers_previous_selection() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        let preview = manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        let (mut journal, _) = manager.journal().unwrap().unwrap();
        journal.phase = Phase::Applying;
        journal.intent_count = 1;
        journal.save(&manager.path()).unwrap();
        let action = &journal.plan.actions[0];
        manager
            .resources
            .write(&action.target, &action.after)
            .unwrap();
        assert!(manager.status().unwrap().recovery_required);
        assert!(manager.runtime_files().is_err());
        manager.apply(&preview.digest, true, || Ok(())).unwrap();
        assert!(manager.status().unwrap().selected.is_none());
        assert!(!manager.status().unwrap().files_changed);
    }

    #[test]
    fn build_checks_and_game_running_gate_precede_mutations() {
        let fixture = Fixture::new();
        let mut manager = fixture.manager();
        assert!(
            manager
                .prepare(Some("skin-01".into()), &"b".repeat(64))
                .is_err()
        );
        let preview = manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        assert!(
            manager
                .apply(&preview.digest, false, || Err(AppError::new("gameRunning")))
                .is_err()
        );
        assert!(!fixture.game.path().join("dinput8.dll").exists());
    }

    #[test]
    fn journal_cannot_widen_the_three_file_recipe() {
        let fixture = Fixture::new();
        let manager = fixture.manager();
        manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        let (mut journal, _) = manager.journal().unwrap().unwrap();
        journal.plan.actions[0].target = Target::File {
            path: "Game.exe".into(),
        };
        journal.digest = journal.expected_digest().unwrap();
        journal.save(&manager.path()).unwrap();
        assert!(manager.status().is_err());
        assert!(manager.runtime_files().is_err());
    }

    #[test]
    fn initial_preview_history_remains_readable_after_loader_hardening() {
        let fixture = Fixture::new();
        let manager = fixture.manager();
        manager
            .prepare(Some("skin-01".into()), performance::GAME_SHA256)
            .unwrap();
        let (mut journal, mut context) = manager.journal().unwrap().unwrap();
        context.next_revision = 0;
        let images = manager.expected(Some("skin-01"), 0).unwrap();
        for (action, image) in journal.plan.actions.iter_mut().zip(images) {
            action.after = image;
        }
        let mut context = serde_json::to_value(context).unwrap();
        context.as_object_mut().unwrap().remove("previousRevision");
        context.as_object_mut().unwrap().remove("nextRevision");
        journal.context = Some(context);
        journal.phase = Phase::Committed;
        journal.intent_count = journal.plan.actions.len();
        journal.digest = journal.expected_digest().unwrap();
        journal.save(&manager.path()).unwrap();
        let status = manager.status().unwrap();
        assert_eq!(status.selected.as_deref(), Some("skin-01"));
        assert!(status.requires_update);
        assert!(manager.runtime_files().is_err());
    }
}
