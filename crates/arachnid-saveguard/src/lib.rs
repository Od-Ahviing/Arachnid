pub mod discovery;
pub mod source;
pub mod watcher;

use arachnid_core::{AppError, AppResult};
use arachnid_repair::{Action, Image, Journal, OperationPlan, Resources, Target};
use serde::{Deserialize, Serialize};
use source::{Generation, hash};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Duration,
};

pub const STABILITY_WINDOW: Duration = Duration::from_millis(750);
pub const AUTOMATIC_COPY_WINDOW: Duration = Duration::from_secs(5 * 60);
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_HISTORY: usize = 1000;
const DEFAULT_QUOTA: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotStage {
    FilesCopied,
    ManifestSynced,
    Published,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceProfile {
    pub schema_version: u32,
    pub id: String,
    pub source: PathBuf,
    pub enabled: bool,
    #[serde(default)]
    pub automatic: bool,
    #[serde(default)]
    pub manual_source: bool,
    pub keep: usize,
    pub quota_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub id: String,
    pub profile_id: String,
    pub created_at: u64,
    pub sequence: u64,
    pub reason: String,
    pub pinned: bool,
    pub status: String,
    pub files: Vec<Entry>,
    #[serde(default)]
    pub window_started_at: Option<u64>,
    #[serde(default = "legacy_payload_directory")]
    pub payload_directory: String,
}

fn legacy_payload_directory() -> String {
    "files".into()
}

pub struct Store {
    root: PathBuf,
    pub profile: SourceProfile,
    _lock: File,
    _guard: arachnid_repair::filesystem::DirectoryGuard,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RestoreContext {
    digest: String,
    before: Vec<Entry>,
    after: Vec<Entry>,
}

fn describe(generation: &Generation) -> Vec<Entry> {
    generation
        .iter()
        .map(|(path, image)| Entry {
            path: path.clone(),
            bytes: image.bytes.len() as u64,
            sha256: hash(&image.bytes),
        })
        .collect()
}

fn valid_id(id: &str) -> bool {
    id.len() >= 8
        && id.len() <= 96
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

pub fn save_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| AppError::new("invalidPath"))?;
    let _guard = arachnid_repair::filesystem::DirectoryGuard::acquire(parent)?;
    let mut staged =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| AppError::new("writeFailed"))?;
    serde_json::to_writer(&mut staged, value).map_err(|_| AppError::new("writeFailed"))?;
    staged
        .flush()
        .and_then(|_| staged.as_file().sync_all())
        .map_err(|_| AppError::new("writeFailed"))?;
    staged
        .persist(path)
        .map_err(|_| AppError::new("writeFailed"))?;
    Ok(())
}

pub fn load_json<T: serde::de::DeserializeOwned>(path: &Path, limit: u64) -> AppResult<T> {
    let parent = path.parent().ok_or_else(|| AppError::new("invalidPath"))?;
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| AppError::new("invalidPath"))?;
    let image = arachnid_repair::filesystem::read(parent, name)?
        .ok_or_else(|| AppError::new("saveCorrupt"))?;
    if image.bytes.len() as u64 > limit {
        return Err(AppError::new("operationLimit"));
    }
    serde_json::from_slice(&image.bytes).map_err(|_| AppError::new("saveCorrupt"))
}

impl SourceProfile {
    pub fn discover(documents: &Path, backup_root: &Path) -> AppResult<Option<Self>> {
        discovery::Discovery::native(documents).find(backup_root)
    }

    pub fn new(source: &Path, backup_root: &Path) -> AppResult<Self> {
        source::separate(source, backup_root)?;
        let source = source::safe_directory(source)?;
        let id = hash(source.to_string_lossy().to_lowercase().as_bytes());
        Ok(Self {
            schema_version: 1,
            id,
            source,
            enabled: false,
            automatic: false,
            manual_source: false,
            keep: 20,
            quota_bytes: DEFAULT_QUOTA,
        })
    }
}

impl Store {
    pub fn open(backup_root: &Path, profile: SourceProfile) -> AppResult<Self> {
        if profile.schema_version != 1
            || !valid_id(&profile.id)
            || !(1..=100).contains(&profile.keep)
            || !(8 * 1024 * 1024..=1024 * 1024 * 1024).contains(&profile.quota_bytes)
        {
            return Err(AppError::new("saveCorrupt"));
        }
        source::separate(&profile.source, backup_root)?;
        if SourceProfile::new(&profile.source, backup_root)?.id != profile.id {
            return Err(AppError::new("saveCorrupt"));
        }
        let root = backup_root.join(&profile.id);
        fs::create_dir_all(&root).map_err(|_| AppError::new("writeFailed"))?;
        let guard = arachnid_repair::filesystem::DirectoryGuard::acquire(&root)?;
        let mut options = fs::OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };
            options
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        }
        let lock = options
            .open(root.join("store.lock"))
            .map_err(|_| AppError::new("writeFailed"))?;
        if !lock
            .metadata()
            .is_ok_and(|m| m.is_file() && !arachnid_detector::is_link(&m))
        {
            return Err(AppError::new("invalidPath"));
        }
        lock.try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        Ok(Self {
            root,
            profile,
            _lock: lock,
            _guard: guard,
        })
    }

    pub fn journal_path(&self) -> PathBuf {
        self.root.join("restore.json")
    }

    pub fn directory(&self) -> &Path {
        &self.root
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let snapshot = self.manifest(id)?;
        if snapshot.pinned {
            return Err(AppError::new("savePinned"));
        }
        if !self
            .index()?
            .iter()
            .any(|copy| copy.id != id && self.load_snapshot(&copy.id).is_ok())
        {
            return Err(AppError::new("saveLastCopy"));
        }
        if self.operation()?.is_some_and(|j| !j.terminal()) {
            return Err(AppError::new("saveRestorePending"));
        }
        let path = self.snapshot_path(id)?;
        let destination = self.root.join(format!("deleted-{id}"));
        if destination.exists() {
            return Err(AppError::new("saveCorrupt"));
        }
        // Removal from history is reversible and never deletes the live game files.
        fs::rename(path, destination).map_err(|_| AppError::new("writeFailed"))
    }

    pub fn undo_delete(&self, id: &str) -> AppResult<()> {
        if !valid_id(id) || !id.starts_with("snapshot-") {
            return Err(AppError::new("invalidPath"));
        }
        let deleted = self.root.join(format!("deleted-{id}"));
        source::safe_directory(&deleted)?;
        let manifest: Snapshot = load_json(&deleted.join("manifest.json"), MAX_MANIFEST_BYTES)?;
        if manifest.id != id
            || manifest.profile_id != self.profile.id
            || self.root.join(id).exists()
        {
            return Err(AppError::new("saveCorrupt"));
        }
        fs::rename(&deleted, self.root.join(id)).map_err(|_| AppError::new("writeFailed"))?;
        if let Err(error) = self.load_snapshot(id) {
            fs::rename(self.root.join(id), deleted).map_err(|_| AppError::new("writeFailed"))?;
            return Err(error);
        }
        Ok(())
    }

    fn snapshot_path(&self, id: &str) -> AppResult<PathBuf> {
        if !valid_id(id) {
            return Err(AppError::new("invalidPath"));
        }
        let path = self.root.join(id);
        source::safe_directory(&path)?;
        Ok(path)
    }

    fn manifest(&self, id: &str) -> AppResult<Snapshot> {
        let path = self.snapshot_path(id)?;
        let snapshot: Snapshot = load_json(&path.join("manifest.json"), MAX_MANIFEST_BYTES)?;
        if snapshot.schema_version != 1
            || snapshot.id != id
            || snapshot.profile_id != self.profile.id
            || snapshot.status != "copyVerified"
            || snapshot.files.is_empty()
            || snapshot.files.len() > source::MAX_FILES
            || !(snapshot.payload_directory == "files"
                || (snapshot.payload_directory.starts_with("payload-")
                    && valid_id(&snapshot.payload_directory)))
        {
            return Err(AppError::new("saveCorrupt"));
        }
        let mut identities = BTreeSet::new();
        let mut total = 0_u64;
        for entry in &snapshot.files {
            arachnid_repair::filesystem::validate_relative(&entry.path)?;
            total = total.saturating_add(entry.bytes);
            if !identities.insert(entry.path.to_lowercase())
                || entry.bytes > arachnid_repair::MAX_IMAGE_BYTES as u64
                || total > source::MAX_BYTES as u64
                || entry.sha256.len() != 64
                || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(AppError::new("saveCorrupt"));
            }
        }
        Ok(snapshot)
    }

    fn load_snapshot(&self, id: &str) -> AppResult<(Snapshot, Generation)> {
        let path = self.snapshot_path(id)?;
        let snapshot = self.manifest(id)?;
        let mut generation = Generation::new();
        for entry in &snapshot.files {
            let image = arachnid_repair::filesystem::read(
                &path.join(&snapshot.payload_directory),
                &entry.path,
            )?
            .ok_or_else(|| AppError::new("saveCorrupt"))?;
            if entry.bytes != image.bytes.len() as u64 || entry.sha256 != hash(&image.bytes) {
                return Err(AppError::new("saveCorrupt"));
            }
            generation.insert(entry.path.clone(), image);
        }
        Ok((snapshot, generation))
    }

    pub fn history(&self) -> AppResult<Vec<Snapshot>> {
        let snapshots = self.index()?;
        for snapshot in &snapshots {
            self.load_snapshot(&snapshot.id)?;
        }
        Ok(snapshots)
    }

    /// Listing reads manifests only; restoring and deleting always revalidate the payload hashes.
    pub fn index(&self) -> AppResult<Vec<Snapshot>> {
        let _guard = arachnid_repair::filesystem::DirectoryGuard::acquire(&self.root)?;
        let mut snapshots = Vec::new();
        let mut count = 0;
        for entry in fs::read_dir(&self.root).map_err(|_| AppError::new("saveCorrupt"))? {
            count += 1;
            if count > MAX_HISTORY + 10 {
                return Err(AppError::new("operationLimit"));
            }
            let entry = entry.map_err(|_| AppError::new("saveCorrupt"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("snapshot-") {
                continue;
            }
            snapshots.push(self.manifest(&name)?);
        }
        snapshots.sort_by(|a, b| b.sequence.cmp(&a.sequence).then_with(|| b.id.cmp(&a.id)));
        Ok(snapshots)
    }

    pub fn snapshot(&self, reason: &str, pause: Duration) -> AppResult<Snapshot> {
        self.snapshot_with_checkpoint(reason, pause, |_| {})
    }

    pub fn snapshot_with_checkpoint(
        &self,
        reason: &str,
        pause: Duration,
        checkpoint: impl FnMut(SnapshotStage),
    ) -> AppResult<Snapshot> {
        if arachnid_repair::pending(&self.journal_path()) {
            return Err(AppError::new("recoveryRequired"));
        }
        let generation = source::stable(&self.profile.source, pause)?;
        self.publish_with_checkpoint(&generation, reason, checkpoint)
    }

    pub fn reconcile(&self, reason: &str, pause: Duration) -> AppResult<Option<Snapshot>> {
        self.reconcile_at(reason, pause, arachnid_repair::now()?)
    }

    pub fn reconcile_at(
        &self,
        reason: &str,
        pause: Duration,
        now: u64,
    ) -> AppResult<Option<Snapshot>> {
        if !["watch", "beforeGame", "afterGame"].contains(&reason) {
            return Err(AppError::new("invalidPlan"));
        }
        if arachnid_repair::pending(&self.journal_path()) {
            return Err(AppError::new("recoveryRequired"));
        }
        let generation = source::stable(&self.profile.source, pause)?;
        if generation.is_empty() {
            return Ok(None);
        }
        let observed = self.root.join("observed.json");
        if observed.exists()
            && load_json::<Vec<Entry>>(&observed, MAX_MANIFEST_BYTES)? == describe(&generation)
        {
            return Ok(None);
        }
        let history = self.index()?;
        if let Some(latest) = history.first()
            && self.load_snapshot(&latest.id)?.1 == generation
        {
            return Ok(None);
        }
        let replacement = history.first().filter(|snapshot| {
            !snapshot.pinned
                && now >= snapshot.created_at
                && snapshot.window_started_at.is_some_and(|start| {
                    now.checked_sub(start)
                        .is_some_and(|elapsed| elapsed < AUTOMATIC_COPY_WINDOW.as_secs())
                })
                && ["watch", "beforeGame", "afterGame"].contains(&snapshot.reason.as_str())
        });
        if replacement.is_some() && self.operation()?.is_some_and(|journal| !journal.terminal()) {
            return Err(AppError::new("saveRestorePending"));
        }
        self.publish_generation(&generation, reason, |_| {}, replacement, Some(now))
            .map(Some)
    }

    fn publish(&self, generation: &Generation, reason: &str) -> AppResult<Snapshot> {
        self.publish_with_checkpoint(generation, reason, |_| {})
    }

    fn publish_with_checkpoint(
        &self,
        generation: &Generation,
        reason: &str,
        checkpoint: impl FnMut(SnapshotStage),
    ) -> AppResult<Snapshot> {
        self.publish_generation(generation, reason, checkpoint, None, None)
    }

    fn publish_generation(
        &self,
        generation: &Generation,
        reason: &str,
        mut checkpoint: impl FnMut(SnapshotStage),
        replacement: Option<&Snapshot>,
        automatic_time: Option<u64>,
    ) -> AppResult<Snapshot> {
        if generation.is_empty() {
            return Err(AppError::new("saveEmpty"));
        }
        if ![
            "manual",
            "watch",
            "beforeGame",
            "afterGame",
            "beforeRestore",
        ]
        .contains(&reason)
        {
            return Err(AppError::new("invalidPlan"));
        }
        let history = self.index()?;
        if let Some(previous) = replacement {
            self.load_snapshot(&previous.id)?;
            self.clean_payloads(previous, false)?;
        }
        let size: u64 = generation
            .values()
            .map(|image| image.bytes.len() as u64)
            .sum();
        let used = used_bytes(&self.root, &mut 0, 0)?;
        // Retention never sacrifices an existing generation to make a new copy fit.
        if used.saturating_add(size).saturating_add(MAX_MANIFEST_BYTES) > self.profile.quota_bytes {
            return Err(AppError::new("saveQuotaReached"));
        }
        arachnid_repair::filesystem::ensure_space(&self.root, size + MAX_MANIFEST_BYTES)?;
        let staging = tempfile::Builder::new()
            .prefix("staging-")
            .tempdir_in(&self.root)
            .map_err(|_| AppError::new("writeFailed"))?;
        let suffix = staging
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::new("invalidPath"))?;
        let id = format!("snapshot-{}-{}", arachnid_repair::now()?, suffix);
        let files = staging.path().join("files");
        fs::create_dir(&files).map_err(|_| AppError::new("writeFailed"))?;
        let mut entries = Vec::new();
        for (name, image) in generation {
            arachnid_repair::filesystem::validate_relative(name)?;
            let target = files.join(name);
            fs::create_dir_all(
                target
                    .parent()
                    .ok_or_else(|| AppError::new("invalidPath"))?,
            )
            .map_err(|_| AppError::new("writeFailed"))?;
            arachnid_repair::filesystem::write(&files, name, &Some(image.clone()))?;
            if arachnid_repair::filesystem::read(&files, name)? != Some(image.clone()) {
                return Err(AppError::new("validationFailed"));
            }
            entries.push(Entry {
                path: name.clone(),
                bytes: image.bytes.len() as u64,
                sha256: hash(&image.bytes),
            });
        }
        // A second complete source pass rejects a producer that changed the group during staging.
        if source::capture(&self.profile.source)? != *generation {
            return Err(AppError::new("saveUnstable"));
        }
        checkpoint(SnapshotStage::FilesCopied);
        let mut snapshot = Snapshot {
            schema_version: 1,
            id,
            profile_id: self.profile.id.clone(),
            created_at: automatic_time.unwrap_or(arachnid_repair::now()?),
            sequence: history
                .iter()
                .map(|s| s.sequence)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| AppError::new("operationLimit"))?,
            reason: reason.into(),
            pinned: reason == "beforeRestore",
            status: "copyVerified".into(),
            files: entries,
            window_started_at: automatic_time,
            payload_directory: legacy_payload_directory(),
        };
        if let Some(previous) = replacement {
            snapshot.id = previous.id.clone();
            snapshot.sequence = previous.sequence;
            snapshot.window_started_at = previous.window_started_at;
            snapshot.payload_directory = format!("payload-{suffix}");
            let path = self.snapshot_path(&snapshot.id)?;
            fs::rename(&files, path.join(&snapshot.payload_directory))
                .map_err(|_| AppError::new("writeFailed"))?;
            // The atomic manifest switch keeps the previous complete payload readable until publication.
            save_json(&staging.path().join("manifest.json"), &snapshot)?;
            checkpoint(SnapshotStage::ManifestSynced);
            save_json(&path.join("manifest.json"), &snapshot)?;
        } else {
            save_json(&staging.path().join("manifest.json"), &snapshot)?;
            checkpoint(SnapshotStage::ManifestSynced);
            let final_path = self.root.join(&snapshot.id);
            fs::rename(staging.path(), &final_path).map_err(|_| AppError::new("writeFailed"))?;
        }
        checkpoint(SnapshotStage::Published);
        self.load_snapshot(&snapshot.id)?;
        self.clean_payloads(&snapshot, false)?;
        save_json(&self.root.join("observed.json"), &describe(generation))?;
        self.retain(&snapshot.id)?;
        Ok(snapshot)
    }

    fn clean_payloads(&self, snapshot: &Snapshot, include_current: bool) -> AppResult<()> {
        let path = self.snapshot_path(&snapshot.id)?;
        for entry in fs::read_dir(&path)
            .map_err(|_| AppError::new("saveCorrupt"))?
            .take(MAX_HISTORY + 1)
        {
            let entry = entry.map_err(|_| AppError::new("saveCorrupt"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name != "files" && !(name.starts_with("payload-") && valid_id(&name)) {
                continue;
            }
            if !include_current && name == snapshot.payload_directory {
                continue;
            }
            let files = path.join(name);
            let contents = source::capture(&files)?;
            for name in contents.keys() {
                arachnid_repair::filesystem::write(&files, name, &None)?;
            }
            remove_empty_tree(&files)?;
        }
        Ok(())
    }

    fn retain(&self, newest: &str) -> AppResult<()> {
        self.load_snapshot(newest)?;
        let mut history = self.index()?;
        let keep = self.profile.keep.saturating_sub(usize::from(
            history.iter().any(|s| s.id == newest && !s.pinned),
        ));
        history.retain(|s| s.id != newest && !s.pinned);
        for snapshot in history.into_iter().skip(keep) {
            let (_, expected) = self.load_snapshot(&snapshot.id)?;
            let path = self.snapshot_path(&snapshot.id)?;
            // Deletion is restricted to a validated, direct child snapshot directory, never a source path.
            if path.parent() != Some(self.root.as_path()) {
                return Err(AppError::new("invalidPath"));
            }
            let files = path.join(&snapshot.payload_directory);
            if source::capture(&files)? != expected {
                return Err(AppError::new("saveCorrupt"));
            }
            self.clean_payloads(&snapshot, true)?;
            arachnid_repair::filesystem::write(&path, "manifest.json", &None)?;
            fs::remove_dir(path).map_err(|_| AppError::new("writeFailed"))?;
        }
        Ok(())
    }

    pub fn pin(&self, id: &str, pinned: bool) -> AppResult<()> {
        let (mut snapshot, _) = self.load_snapshot(id)?;
        snapshot.pinned = pinned;
        save_json(&self.snapshot_path(id)?.join("manifest.json"), &snapshot)?;
        if let Some(latest) = self.index()?.first() {
            self.retain(&latest.id)?;
        }
        Ok(())
    }

    pub fn prepare_restore(
        &self,
        id: &str,
        gate: impl Fn() -> AppResult<()>,
    ) -> AppResult<Journal> {
        gate()?;
        if arachnid_repair::pending(&self.journal_path()) {
            return Err(AppError::new("recoveryRequired"));
        }
        if self.journal_path().exists() {
            let journal = Journal::load(&self.journal_path())?;
            if !journal.terminal() {
                return Err(AppError::new("recoveryRequired"));
            }
            let archive = self.root.join(format!("restore-{}.json", journal.digest));
            if !archive.exists() {
                fs::copy(self.journal_path(), archive).map_err(|_| AppError::new("writeFailed"))?;
            }
        }
        let (_, desired) = self.load_snapshot(id)?;
        let current = source::stable(&self.profile.source, STABILITY_WINDOW)?;
        let keys: BTreeSet<_> = current.keys().chain(desired.keys()).cloned().collect();
        let mut actions = Vec::new();
        for path in keys {
            let before = current.get(&path).cloned();
            let after = desired.get(&path).cloned();
            if before != after {
                // No automatic directory creation/deletion in the live source.
                arachnid_repair::filesystem::resolve(&self.profile.source, &path)?;
                actions.push(Action {
                    target: Target::File { path },
                    before,
                    after,
                });
            }
        }
        if actions.is_empty() {
            return Err(AppError::new("saveNoChanges"));
        }
        let plan = OperationPlan {
            schema_version: 1,
            installation: self.profile.id.clone(),
            actions,
        };
        plan.validate()?;
        let journal_estimate = serde_json::to_vec(&plan)
            .map_err(|_| AppError::new("invalidPlan"))?
            .len() as u64;
        if used_bytes(&self.root, &mut 0, 0)?
            .saturating_add(journal_estimate.saturating_mul(2))
            .saturating_add(source::MAX_BYTES as u64)
            > self.profile.quota_bytes
        {
            return Err(AppError::new("saveQuotaReached"));
        }
        if !current.is_empty() {
            self.publish(&current, "beforeRestore")?;
        }
        gate()?;
        let journal = arachnid_repair::prepare(
            plan,
            &FileResources(&self.profile.source),
            &self.journal_path(),
        )?;
        save_json(
            &self.root.join("restore-context.json"),
            &RestoreContext {
                digest: journal.digest.clone(),
                before: describe(&current),
                after: describe(&desired),
            },
        )?;
        Ok(journal)
    }

    pub fn operation(&self) -> AppResult<Option<Journal>> {
        if !self.journal_path().exists() {
            return Ok(None);
        }
        let journal = Journal::load(&self.journal_path())?;
        if journal.plan.installation != self.profile.id
            || journal
                .plan
                .actions
                .iter()
                .any(|a| !matches!(a.target, Target::File { .. }))
        {
            return Err(AppError::new("invalidPlan"));
        }
        Ok(Some(journal))
    }

    pub fn apply_restore(&self, digest: &str, gate: impl Fn() -> AppResult<()>) -> AppResult<()> {
        let mut journal = self
            .operation()?
            .ok_or_else(|| AppError::new("invalidPlan"))?;
        let context: RestoreContext =
            load_json(&self.root.join("restore-context.json"), MAX_MANIFEST_BYTES)?;
        if context.digest != digest
            || context.before != describe(&source::stable(&self.profile.source, STABILITY_WINDOW)?)
        {
            return Err(AppError::new("stalePlan"));
        }
        arachnid_repair::apply(
            &mut journal,
            digest,
            &mut FileResources(&self.profile.source),
            &self.journal_path(),
            &AtomicBool::new(false),
            &gate,
        )?;
        let validated = source::capture(&self.profile.source).and_then(|generation| {
            if describe(&generation) == context.after {
                Ok(())
            } else {
                Err(AppError::new("validationFailed"))
            }
        });
        if let Err(error) = validated {
            arachnid_repair::rollback_guarded(
                &mut journal,
                &mut FileResources(&self.profile.source),
                &self.journal_path(),
                gate,
            )?;
            return Err(error);
        }
        Ok(())
    }

    pub fn undo_restore(&self, digest: &str, gate: impl Fn() -> AppResult<()>) -> AppResult<()> {
        let mut journal = self
            .operation()?
            .ok_or_else(|| AppError::new("invalidPlan"))?;
        if journal.digest != digest {
            return Err(AppError::new("approvalInvalid"));
        }
        arachnid_repair::rollback_guarded(
            &mut journal,
            &mut FileResources(&self.profile.source),
            &self.journal_path(),
            gate,
        )
    }
}

fn used_bytes(path: &Path, visited: &mut usize, depth: usize) -> AppResult<u64> {
    if depth > 12 {
        return Err(AppError::new("operationLimit"));
    }
    let _guard = arachnid_repair::filesystem::DirectoryGuard::acquire(path)?;
    let mut bytes = 0_u64;
    for entry in fs::read_dir(path).map_err(|_| AppError::new("saveCorrupt"))? {
        *visited += 1;
        if *visited > 4096 {
            return Err(AppError::new("operationLimit"));
        }
        let entry = entry.map_err(|_| AppError::new("saveCorrupt"))?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| AppError::new("saveCorrupt"))?;
        if arachnid_detector::is_link(&metadata) {
            return Err(AppError::new("invalidPath"));
        }
        bytes = bytes.saturating_add(if metadata.is_dir() {
            used_bytes(&entry.path(), visited, depth + 1)?
        } else if metadata.is_file() {
            metadata.len()
        } else {
            return Err(AppError::new("invalidPath"));
        });
    }
    Ok(bytes)
}

fn remove_empty_tree(path: &Path) -> AppResult<()> {
    source::safe_directory(path)?;
    for entry in fs::read_dir(path).map_err(|_| AppError::new("writeFailed"))? {
        let entry = entry.map_err(|_| AppError::new("writeFailed"))?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| AppError::new("writeFailed"))?;
        if !metadata.is_dir() || arachnid_detector::is_link(&metadata) {
            return Err(AppError::new("saveCorrupt"));
        }
        remove_empty_tree(&entry.path())?;
    }
    fs::remove_dir(path).map_err(|_| AppError::new("writeFailed"))
}

struct FileResources<'a>(&'a Path);
impl Resources for FileResources<'_> {
    fn read(&self, target: &Target) -> AppResult<Option<Image>> {
        match target {
            Target::File { path } => arachnid_repair::filesystem::read(self.0, path),
            _ => Err(AppError::new("invalidPlan")),
        }
    }
    fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
        match target {
            Target::File { path } => arachnid_repair::filesystem::write(self.0, path, image),
            _ => Err(AppError::new("invalidPlan")),
        }
    }
}
