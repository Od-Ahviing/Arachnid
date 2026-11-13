use crate::{
    fix_adapter,
    fix_settings::{self, DisplaySettings},
    offline::Payload as SourcePayload,
    payloads::{Payload, PayloadResources},
    performance,
};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::{
    Action, Image, Journal, OperationPlan, Phase, Resources, Target,
    filesystem::{self, DirectoryGuard},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

#[derive(Debug, Clone)]
pub struct PackSpec {
    pub uses_fix: bool,
    pub version: String,
    pub game_sha256: String,
    pub binaries: Vec<SourcePayload>,
    pub original_wrapper: Option<String>,
    pub previous_revisions: Vec<PackRevision>,
}

#[derive(Debug, Clone)]
pub struct PackRevision {
    pub uses_fix: bool,
    pub version: String,
    pub binaries: Vec<SourcePayload>,
}

impl PackSpec {
    pub fn for_composition(composition: crate::composition::Composition) -> Self {
        let mut spec = Self::candidate();
        spec.previous_revisions.push(PackRevision {
            version: performance::VERSION.into(),
            binaries: performance::binaries(),
            uses_fix: true,
        });
        for bits in 1..=7 {
            let other =
                crate::composition::Composition::from_bits(bits).expect("bounded composition");
            if other != composition {
                spec.previous_revisions.push(PackRevision {
                    version: other.version(),
                    binaries: other.binaries(),
                    uses_fix: other.fix,
                });
            }
        }
        spec.version = composition.version();
        spec.binaries = composition.binaries();
        spec.uses_fix = composition.fix;
        spec
    }
    pub fn candidate() -> Self {
        Self {
            uses_fix: true,
            version: performance::VERSION.into(),
            game_sha256: performance::GAME_SHA256.into(),
            binaries: performance::binaries(),
            original_wrapper: Some(fix_adapter::ORIGINAL_SHA256.into()),
            previous_revisions: vec![
                PackRevision {
                    uses_fix: true,
                    version: "local-preview-1".into(),
                    binaries: performance::legacy_binaries(),
                },
                PackRevision {
                    uses_fix: true,
                    version: "local-preview-2".into(),
                    binaries: performance::binaries(),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PackOperation {
    Enable,
    Disable,
    Sync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_context: Option<Box<Context>>,
    domain: String,
    version: String,
    previous_active: bool,
    operation: PackOperation,
    display: DisplaySettings,
    baseline: OperationPlan,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackStatus {
    pub active: bool,
    pub recovery_required: bool,
    pub settings_out_of_sync: bool,
    pub files_changed: bool,
    pub phase: Option<Phase>,
    pub requires_update: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFileChange {
    pub name: String,
    pub before: Option<Payload>,
    pub after: Option<Payload>,
    pub conflict: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackPreview {
    pub digest: String,
    pub operation: PackOperation,
    pub display: DisplaySettings,
    pub phase: Phase,
    pub files: Vec<PackFileChange>,
}

pub struct PackManager {
    root: PathBuf,
    state: PathBuf,
    installation: String,
    spec: PackSpec,
    resources: PayloadResources,
    _state_guard: DirectoryGuard,
}

impl PackManager {
    pub fn new(root: &Path, state: &Path, installation: &str, spec: PackSpec) -> AppResult<Self> {
        if !crate::hash(installation)
            || !crate::hash(&spec.game_sha256)
            || spec.version.is_empty()
            || spec.version.len() > 64
            || spec.binaries.is_empty()
            || spec.binaries.len() > 2
            || spec.binaries[0].file_name != "d3d9.dll"
            || spec
                .binaries
                .get(1)
                .is_some_and(|p| p.file_name != "ar_dxvk.dll")
            || spec.binaries.iter().any(|p| {
                !crate::hash(&p.sha256)
                    || p.bytes == 0
                    || p.bytes > crate::payloads::MAX_PAYLOAD_BYTES
            })
        {
            return Err(invalid());
        }
        if spec.previous_revisions.len() > 16 {
            return Err(invalid());
        }
        let mut versions = BTreeSet::new();
        versions.insert(spec.version.as_str());
        for revision in &spec.previous_revisions {
            if revision.version.is_empty()
                || revision.version.len() > 64
                || !versions.insert(revision.version.as_str())
                || revision.binaries.is_empty()
                || revision.binaries.len() > 2
                || revision.binaries[0].file_name != "d3d9.dll"
                || revision
                    .binaries
                    .get(1)
                    .is_some_and(|p| p.file_name != "ar_dxvk.dll")
                || revision.binaries.iter().any(|p| {
                    !crate::hash(&p.sha256)
                        || p.bytes == 0
                        || p.bytes > crate::payloads::MAX_PAYLOAD_BYTES
                })
            {
                return Err(invalid());
            }
        }
        let state_guard = DirectoryGuard::acquire(state)?;
        let blobs = state.join("blobs");
        match fs::create_dir(&blobs) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(invalid()),
        }
        let resources = PayloadResources::new(root, &blobs)?;
        Ok(Self {
            root: root.into(),
            state: state.into(),
            installation: installation.into(),
            spec,
            resources,
            _state_guard: state_guard,
        })
    }

    fn journal_path(&self) -> PathBuf {
        self.state.join("performance-transaction.json")
    }

    fn binaries_for_version(&self, version: &str) -> AppResult<&[SourcePayload]> {
        if version == self.spec.version {
            return Ok(&self.spec.binaries);
        }
        self.spec
            .previous_revisions
            .iter()
            .find(|revision| revision.version == version)
            .map(|revision| revision.binaries.as_slice())
            .ok_or_else(invalid)
    }

    fn uses_fix_for_version(&self, version: &str) -> AppResult<bool> {
        if version == self.spec.version {
            return Ok(self.spec.uses_fix);
        }
        self.spec
            .previous_revisions
            .iter()
            .find(|r| r.version == version)
            .map(|r| r.uses_fix)
            .ok_or_else(invalid)
    }

    pub fn applied_version(&self) -> AppResult<Option<String>> {
        Ok(self
            .load()?
            .filter(|(j, c)| Self::active(j, c))
            .map(|(j, c)| effective_context(&j, &c).version.clone()))
    }

    fn load(&self) -> AppResult<Option<(Journal, Context)>> {
        let path = self.journal_path();
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Ok(metadata) if metadata.is_file() && !arachnid_detector::is_link(&metadata) => (),
            _ => return Err(AppError::new("recoveryRequired")),
        }
        let journal = Journal::load(&path)?;
        let context: Context = serde_json::from_value(journal.context.clone().ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        if context.domain != "arachnid-performance-pack"
            || context.baseline.installation != self.installation
            || journal.plan.installation != self.installation
        {
            return Err(invalid());
        }
        validate_baseline(&context.baseline)?;
        if journal.plan.actions.is_empty() {
            journal.plan.validate_context_only()?;
        } else {
            PayloadResources::validate_plan(&journal.plan)?;
        }
        let allowed: BTreeSet<_> = ["d3d9.dll", "ar_dxvk.dll", "d3d9.ini"]
            .into_iter()
            .collect();
        for plan in [&context.baseline, &journal.plan] {
            if plan.actions.iter().any(|action| !matches!(&action.target, Target::File { path } if allowed.contains(path.as_str()))) {
                return Err(invalid());
            }
        }
        if context.baseline.actions.len() != 3
            || context
                .baseline
                .actions
                .iter()
                .map(|a| &a.target)
                .collect::<BTreeSet<_>>()
                .len()
                != 3
        {
            return Err(invalid());
        }
        for payload in self.binaries_for_version(&context.version)? {
            let expected = Payload {
                sha256: payload.sha256.clone(),
                bytes: payload.bytes,
            }
            .image()?;
            if !context.baseline.actions.iter().any(|a| {
                a.target == target(&payload.file_name) && a.after.as_ref() == Some(&expected)
            }) {
                return Err(invalid());
            }
        }
        for action in &context.baseline.actions {
            let is_binary = self
                .binaries_for_version(&context.version)?
                .iter()
                .any(|p| action.target == target(&p.file_name));
            let is_ini = action.target == target("d3d9.ini")
                && self.uses_fix_for_version(&context.version)?;
            if !is_binary && !is_ini && action.after != action.before {
                return Err(invalid());
            }
        }
        context.display.validate_bounds()?;
        if self.uses_fix_for_version(&context.version)? {
            context.display.validate()?;
        }
        if let Some(previous) = &context.previous_context {
            if previous.previous_context.is_some()
                || previous.domain != context.domain
                || previous.baseline.installation != context.baseline.installation
            {
                return Err(invalid());
            }
            validate_baseline(&previous.baseline)?;
            if previous.baseline.actions.len() != 3
                || previous.baseline.actions.iter().map(|a| &a.target).collect::<BTreeSet<_>>().len() != 3
                || previous.baseline.actions.iter().any(|a| !matches!(&a.target, Target::File {path} if allowed.contains(path.as_str()))) {
                return Err(invalid());
            }
            for payload in self.binaries_for_version(&previous.version)? {
                let expected = Payload {
                    sha256: payload.sha256.clone(),
                    bytes: payload.bytes,
                }
                .image()?;
                if !previous.baseline.actions.iter().any(|a| {
                    a.target == target(&payload.file_name) && a.after.as_ref() == Some(&expected)
                }) {
                    return Err(invalid());
                }
            }
        }
        Ok(Some((journal, context)))
    }

    fn active(journal: &Journal, context: &Context) -> bool {
        if journal.phase == Phase::Committed {
            context.operation != PackOperation::Disable
        } else {
            context.previous_active
        }
    }

    pub fn status(&self, display: &DisplaySettings) -> AppResult<PackStatus> {
        let Some((journal, context)) = self.load()? else {
            return Ok(PackStatus {
                active: false,
                recovery_required: false,
                settings_out_of_sync: false,
                files_changed: false,
                phase: None,
                requires_update: false,
            });
        };
        let active = Self::active(&journal, &context);
        let context = effective_context(&journal, &context);
        let recovery_required = !matches!(
            journal.phase,
            Phase::Prepared | Phase::Committed | Phase::Cancelled | Phase::RolledBack
        );
        let mut files_changed = false;
        if active && !recovery_required {
            for action in &context.baseline.actions {
                if action.target != target("d3d9.ini") {
                    files_changed |= self.resources.observe(&action.target)? != action.after;
                }
            }
        }
        let settings_out_of_sync = active
            && self.uses_fix_for_version(&context.version)?
            && !recovery_required
            && self
                .ini()?
                .is_none_or(|bytes| !fix_settings::matches(&bytes, display));
        Ok(PackStatus {
            active,
            recovery_required,
            settings_out_of_sync,
            files_changed,
            phase: Some(journal.phase),
            requires_update: active && context.version != self.spec.version,
        })
    }

    fn ini(&self) -> AppResult<Option<Vec<u8>>> {
        let image = filesystem::read(&self.root, "d3d9.ini")?;
        if image.as_ref().is_some_and(|i| i.bytes.len() > 64 * 1024) {
            return Err(invalid());
        }
        Ok(image.map(|i| i.bytes))
    }

    fn import_bytes(&self, bytes: &[u8]) -> AppResult<Image> {
        let mut file = tempfile::NamedTempFile::new_in(&self.state).map_err(|_| invalid())?;
        file.write_all(bytes).map_err(|_| invalid())?;
        file.as_file().sync_all().map_err(|_| invalid())?;
        let path = file.into_temp_path();
        self.resources.import(
            &path,
            &Payload {
                sha256: fix_adapter::digest(bytes),
                bytes: bytes.len() as u64,
            },
        )
    }

    pub fn prepare(
        &mut self,
        operation: PackOperation,
        source: &Path,
        display: &DisplaySettings,
        game_sha256: &str,
    ) -> AppResult<Journal> {
        if operation != PackOperation::Disable && game_sha256 != self.spec.game_sha256 {
            return Err(AppError::new("modBuildUnsupported"));
        }
        if operation != PackOperation::Disable {
            display.validate_bounds()?;
            if self.spec.uses_fix {
                display.validate()?;
            }
        }
        if let Some((mut journal, _)) = self.load()?
            && journal.phase == Phase::Prepared
        {
            let path = self.journal_path();
            arachnid_repair::rollback_guarded(&mut journal, &mut self.resources, &path, || Ok(()))?;
        }
        let previous = self.load()?.map(|(j, c)| {
            let effective = effective_context(&j, &c).clone();
            (j, effective)
        });
        let recorded_version = previous
            .as_ref()
            .map(|(_, context)| context.version.clone());
        let recorded_display = previous
            .as_ref()
            .map(|(_, context)| context.display.clone());
        let status = self.status(display)?;
        if status.recovery_required {
            return Err(AppError::new("recoveryRequired"));
        }
        if status.files_changed {
            return Err(AppError::new("modOwnedFileChanged"));
        }
        let replacing =
            operation == PackOperation::Enable && self.spec.version.starts_with("composition-1-");
        if status.requires_update && operation != PackOperation::Disable && !replacing {
            return Err(AppError::new("modPackUpdateRequired"));
        }
        let mut actions = Vec::new();
        let baseline;
        match operation {
            PackOperation::Enable => {
                if status.active && !replacing {
                    return Err(AppError::new("modAlreadyPresent"));
                }
                performance::check_conflicts(&self.root)?;
                let _source_guard = DirectoryGuard::acquire(source)?;
                performance::verified_files(source, &self.spec.binaries)?;
                let original = previous
                    .as_ref()
                    .filter(|_| status.active)
                    .map(|(_, c)| &c.baseline);
                let mut originals: Vec<Action> = Vec::new();
                for name in ["d3d9.dll", "ar_dxvk.dll", "d3d9.ini"] {
                    let target = target(name);
                    let before = self.resources.read(&target)?;
                    let initial = if let Some(original) = original {
                        original
                            .actions
                            .iter()
                            .find(|a| a.target == target)
                            .ok_or_else(invalid)?
                            .before
                            .clone()
                    } else {
                        before.clone()
                    };
                    if original.is_none()
                        && name != "d3d9.ini"
                        && let Some(image) = &before
                    {
                        let observed = Payload::from_image(image)?;
                        if name != "d3d9.dll"
                            || Some(&observed.sha256) != self.spec.original_wrapper.as_ref()
                        {
                            return Err(AppError::new("modUnmanagedFiles"));
                        }
                    }
                    let after = if let Some(payload) =
                        self.spec.binaries.iter().find(|p| p.file_name == name)
                    {
                        Some(self.resources.import(
                            &source.join(name),
                            &Payload {
                                sha256: payload.sha256.clone(),
                                bytes: payload.bytes,
                            },
                        )?)
                    } else if name == "d3d9.ini" && self.spec.uses_fix {
                        let existing = if status.active
                            && previous.as_ref().is_some_and(|(_, c)| {
                                self.uses_fix_for_version(&c.version).unwrap_or(false)
                            }) {
                            self.ini()?
                        } else {
                            None
                        };
                        if original.is_none() && before.is_some() && originals[0].before.is_none() {
                            return Err(AppError::new("modUnmanagedFiles"));
                        }
                        let initial_ini = if existing.is_none() && initial.is_some() {
                            self.ini()?
                        } else {
                            existing
                        };
                        Some(self.import_bytes(&fix_settings::configure(
                            initial_ini.as_deref(),
                            display,
                        )?)?)
                    } else {
                        initial.clone()
                    };
                    originals.push(Action {
                        target: target.clone(),
                        before: initial,
                        after: after.clone(),
                    });
                    if before != after {
                        actions.push(Action {
                            target,
                            before,
                            after,
                        });
                    }
                }
                baseline = OperationPlan {
                    schema_version: 1,
                    installation: self.installation.clone(),
                    actions: originals,
                };
            }
            PackOperation::Disable => {
                if !status.active {
                    return Err(AppError::new("modNotActive"));
                }
                baseline = previous.ok_or_else(invalid)?.1.baseline;
                for original in &baseline.actions {
                    let before = self.resources.read(&original.target)?;
                    if before != original.before {
                        actions.push(Action {
                            target: original.target.clone(),
                            before,
                            after: original.before.clone(),
                        });
                    }
                }
            }
            PackOperation::Sync => {
                if !status.active {
                    return Err(AppError::new("modNotActive"));
                }
                baseline = previous.ok_or_else(invalid)?.1.baseline;
                let existing = self.ini()?.ok_or_else(invalid)?;
                let bytes = fix_settings::configure(Some(&existing), display)?;
                let target = target("d3d9.ini");
                let before = self.resources.read(&target)?;
                let after = Some(self.import_bytes(&bytes)?);
                if before == after {
                    return Err(AppError::new("modNoChanges"));
                }
                actions.push(Action {
                    target,
                    before,
                    after,
                });
            }
        }
        let previous_context = if status.active {
            self.load()?.map(|(j, c)| {
                let mut previous = effective_context(&j, &c).clone();
                previous.previous_context = None;
                Box::new(previous)
            })
        } else {
            None
        };
        let context = Context {
            previous_context,
            domain: "arachnid-performance-pack".into(),
            version: if operation == PackOperation::Disable {
                recorded_version.ok_or_else(invalid)?
            } else {
                self.spec.version.clone()
            },
            previous_active: status.active,
            operation,
            display: if operation == PackOperation::Disable {
                recorded_display.ok_or_else(invalid)?
            } else {
                display.clone()
            },
            baseline,
        };
        let plan = OperationPlan {
            schema_version: 1,
            installation: self.installation.clone(),
            actions,
        };
        if plan.actions.is_empty() {
            plan.validate_context_only()?;
        } else {
            PayloadResources::validate_plan(&plan)?;
        }
        arachnid_repair::prepare_with_context(
            plan,
            &self.resources,
            &self.journal_path(),
            Some(serde_json::to_value(context).map_err(|_| invalid())?),
        )
    }

    pub fn apply(
        &mut self,
        digest: &str,
        display: &DisplaySettings,
        gate: impl Fn() -> AppResult<()>,
    ) -> AppResult<()> {
        let (mut journal, context) = self.load()?.ok_or_else(invalid)?;
        if context.operation != PackOperation::Disable && &context.display != display {
            return Err(AppError::new("stalePlan"));
        }
        for baseline in &context.baseline.actions {
            let expected = journal
                .plan
                .actions
                .iter()
                .find(|a| a.target == baseline.target)
                .map(|a| &a.before)
                .unwrap_or(&baseline.after);
            if self.resources.observe(&baseline.target)?.as_ref() != expected.as_ref() {
                return Err(AppError::new("stalePlan"));
            }
        }
        let path = self.journal_path();
        arachnid_repair::apply(
            &mut journal,
            digest,
            &mut self.resources,
            &path,
            &AtomicBool::new(false),
            gate,
        )
    }

    pub fn recover(&mut self, digest: &str, gate: impl Fn() -> AppResult<()>) -> AppResult<()> {
        let (mut journal, _) = self.load()?.ok_or_else(invalid)?;
        if digest != journal.digest {
            return Err(AppError::new("approvalInvalid"));
        }
        let path = self.journal_path();
        arachnid_repair::rollback_guarded(&mut journal, &mut self.resources, &path, gate)
    }

    pub fn operation(&self) -> AppResult<Option<Journal>> {
        Ok(self.load()?.map(|(journal, _)| journal))
    }

    pub fn applied(&self) -> AppResult<bool> {
        Ok(self
            .load()?
            .is_some_and(|(journal, context)| Self::active(&journal, &context)))
    }

    pub fn preview(&self) -> AppResult<Option<PackPreview>> {
        let Some((journal, context)) = self.load()? else {
            return Ok(None);
        };
        let recovery = !matches!(
            journal.phase,
            Phase::Prepared | Phase::Committed | Phase::Cancelled | Phase::RolledBack
        );
        let actions = if recovery {
            &journal.plan.actions[..journal.intent_count]
        } else {
            &journal.plan.actions[..]
        };
        let files = actions
            .iter()
            .map(|action| {
                let Target::File { path } = &action.target else {
                    return Err(invalid());
                };
                let observed = if recovery {
                    self.resources.observe(&action.target)?
                } else {
                    action.before.clone()
                };
                let conflict = recovery && observed != action.before && observed != action.after;
                let after = if recovery {
                    &action.before
                } else {
                    &action.after
                };
                Ok(PackFileChange {
                    name: path.clone(),
                    before: observed.as_ref().map(Payload::from_image).transpose()?,
                    after: after.as_ref().map(Payload::from_image).transpose()?,
                    conflict,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Some(PackPreview {
            digest: journal.digest,
            operation: context.operation,
            display: context.display,
            phase: journal.phase,
            files,
        }))
    }
}

fn target(path: &str) -> Target {
    Target::File { path: path.into() }
}
fn effective_context<'a>(journal: &Journal, context: &'a Context) -> &'a Context {
    if journal.phase != Phase::Committed {
        context.previous_context.as_deref().unwrap_or(context)
    } else {
        context
    }
}
fn validate_baseline(plan: &OperationPlan) -> AppResult<()> {
    for image in plan
        .actions
        .iter()
        .flat_map(|a| [&a.before, &a.after])
        .flatten()
    {
        Payload::from_image(image)?;
    }
    let mut changes = plan.clone();
    changes.actions.retain(|a| a.before != a.after);
    if changes.actions.is_empty() {
        changes.validate_context_only()
    } else {
        PayloadResources::validate_plan(&changes)
    }
}
fn invalid() -> AppError {
    AppError::new("invalidModState")
}
