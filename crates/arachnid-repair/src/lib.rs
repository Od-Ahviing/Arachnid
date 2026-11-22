pub mod filesystem;
pub mod registry;

use arachnid_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_IMAGE_BYTES: usize = 1024 * 1024;
// A restore includes two save generations, each bounded to 64 MiB and 128 files.
const MAX_PLAN_BYTES: usize = 128 * MAX_IMAGE_BYTES;
const MAX_JOURNAL_BYTES: u64 = 640 * MAX_IMAGE_BYTES as u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Target {
    File { path: String },
    Registry { section: String, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Image {
    pub value_type: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    pub target: Target,
    pub before: Option<Image>,
    pub after: Option<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationPlan {
    pub schema_version: u32,
    pub installation: String,
    pub actions: Vec<Action>,
}

impl OperationPlan {
    pub fn validate_context_only(&self) -> AppResult<()> {
        if self.schema_version != 1 || self.installation.is_empty() || !self.actions.is_empty() {
            return Err(AppError::new("invalidPlan"));
        }
        Ok(())
    }
    pub fn validate(&self) -> AppResult<()> {
        let mut targets = BTreeSet::new();
        let mut bytes = 0;
        if self.schema_version != 1
            || self.installation.is_empty()
            || self.actions.is_empty()
            || self.actions.len() > 256
        {
            return Err(AppError::new("invalidPlan"));
        }
        for action in &self.actions {
            let identity = match &action.target {
                Target::File { path } => format!("file:{}", path.to_lowercase()),
                Target::Registry { section, name } => format!(
                    "registry:{}:{}",
                    section.to_lowercase(),
                    name.to_lowercase()
                ),
            };
            if !targets.insert(identity) || action.before == action.after {
                return Err(AppError::new("invalidPlan"));
            }
            if let Target::File { path } = &action.target {
                filesystem::validate_relative(path)?;
            }
            for image in [&action.before, &action.after].into_iter().flatten() {
                if image.bytes.len() > MAX_IMAGE_BYTES {
                    return Err(AppError::new("operationLimit"));
                }
                if matches!(action.target, Target::File { .. }) && image.value_type != 0 {
                    return Err(AppError::new("invalidPlan"));
                }
                bytes += image.bytes.len();
            }
        }
        if bytes > MAX_PLAN_BYTES {
            return Err(AppError::new("operationLimit"));
        }
        Ok(())
    }

    pub fn digest(&self) -> AppResult<String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| AppError::new("invalidPlan"))?;
        Ok(Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }
}

pub trait Resources {
    fn read(&self, target: &Target) -> AppResult<Option<Image>>;
    fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()>;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Prepared,
    Applying,
    Committed,
    RollingBack,
    RolledBack,
    Cancelled,
    RecoveryRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Journal {
    pub schema_version: u32,
    pub plan: OperationPlan,
    pub digest: String,
    pub prepared_at: u64,
    pub phase: Phase,
    pub intent_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl Journal {
    pub fn expected_digest(&self) -> AppResult<String> {
        context_digest(&self.plan, &self.context)
    }
    pub fn terminal(&self) -> bool {
        matches!(
            self.phase,
            Phase::Committed | Phase::RolledBack | Phase::Cancelled
        )
    }
    pub fn save(&self, path: &Path) -> AppResult<()> {
        let parent = path
            .parent()
            .ok_or_else(|| AppError::new("journalUnavailable"))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| AppError::new("journalUnavailable"))?;
        {
            let mut writer = BufWriter::new(file.as_file_mut());
            serde_json::to_writer(&mut writer, self)
                .map_err(|_| AppError::new("journalUnavailable"))?;
            writer
                .flush()
                .map_err(|_| AppError::new("journalUnavailable"))?;
        }
        file.as_file()
            .sync_all()
            .map_err(|_| AppError::new("journalUnavailable"))?;
        file.persist(path)
            .map_err(|_| AppError::new("journalUnavailable"))?;
        Ok(())
    }
    pub fn load(path: &Path) -> AppResult<Self> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| AppError::new("recoveryRequired"))?
            .take(MAX_JOURNAL_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::new("recoveryRequired"))?;
        if bytes.len() as u64 > MAX_JOURNAL_BYTES {
            return Err(AppError::new("recoveryRequired"));
        }
        let journal: Self =
            serde_json::from_slice(&bytes).map_err(|_| AppError::new("recoveryRequired"))?;
        if journal.schema_version != 1
            || journal.expected_digest()? != journal.digest
            || journal.intent_count > journal.plan.actions.len()
            || (matches!(journal.phase, Phase::Prepared | Phase::Cancelled)
                && journal.intent_count != 0)
            || (journal.phase == Phase::Committed
                && journal.intent_count != journal.plan.actions.len())
            || (journal.phase == Phase::Applying && journal.intent_count == 0)
        {
            return Err(AppError::new("recoveryRequired"));
        }
        Ok(journal)
    }
}

pub fn prepare(
    plan: OperationPlan,
    resources: &impl Resources,
    journal_path: &Path,
) -> AppResult<Journal> {
    prepare_with_context(plan, resources, journal_path, None)
}

pub fn prepare_with_context(
    plan: OperationPlan,
    resources: &impl Resources,
    journal_path: &Path,
    context: Option<serde_json::Value>,
) -> AppResult<Journal> {
    if plan.actions.is_empty() && context.is_some() {
        plan.validate_context_only()?;
    } else {
        plan.validate()?;
    }
    if journal_path.exists() && !Journal::load(journal_path)?.terminal() {
        return Err(AppError::new("recoveryRequired"));
    }
    for action in &plan.actions {
        if resources.read(&action.target)? != action.before {
            return Err(AppError::new("stalePlan"));
        }
    }
    let journal = Journal {
        schema_version: 1,
        digest: context_digest(&plan, &context)?,
        plan,
        prepared_at: now()?,
        phase: Phase::Prepared,
        intent_count: 0,
        context,
    };
    let bytes = serde_json::to_vec(&journal)
        .map_err(|_| AppError::new("invalidPlan"))?
        .len() as u64;
    filesystem::ensure_space(
        journal_path
            .parent()
            .ok_or_else(|| AppError::new("journalUnavailable"))?,
        bytes.saturating_mul(2),
    )?;
    journal.save(journal_path)?;
    // Re-read the durable preimages before any mutation is possible.
    if Journal::load(journal_path)?.digest != journal.digest {
        return Err(AppError::new("recoveryRequired"));
    }
    Ok(journal)
}

pub fn apply(
    journal: &mut Journal,
    approval: &str,
    resources: &mut impl Resources,
    path: &Path,
    cancelled: &AtomicBool,
    gate: impl Fn() -> AppResult<()>,
) -> AppResult<()> {
    if journal.phase != Phase::Prepared
        || approval != journal.digest
        || journal.expected_digest()? != approval
        || now()?
            .checked_sub(journal.prepared_at)
            .is_none_or(|age| age > 600)
    {
        return Err(AppError::new("approvalInvalid"));
    }
    gate()?;
    if cancelled.load(Ordering::Relaxed) {
        journal.phase = Phase::Cancelled;
        journal.save(path)?;
        return Err(AppError::new("cancelled"));
    }
    for action in &journal.plan.actions {
        if resources.read(&action.target)? != action.before {
            return Err(AppError::new("stalePlan"));
        }
    }
    let result = (|| {
        for index in 0..journal.plan.actions.len() {
            gate()?;
            if cancelled.load(Ordering::Relaxed) {
                return Err(AppError::new("cancelled"));
            }
            let action = journal.plan.actions[index].clone();
            if resources.read(&action.target)? != action.before {
                return Err(AppError::new("stalePlan"));
            }
            journal.phase = Phase::Applying;
            journal.intent_count = index + 1;
            journal.save(path)?;
            resources.write(&action.target, &action.after)?;
            if resources.read(&action.target)? != action.after {
                return Err(AppError::new("validationFailed"));
            }
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::new("cancelled"));
        }
        journal.phase = Phase::Committed;
        journal.save(path)
    })();
    if let Err(error) = result {
        rollback_guarded(journal, resources, path, &gate)?;
        return Err(error);
    }
    Ok(())
}

pub fn rollback(
    journal: &mut Journal,
    resources: &mut impl Resources,
    path: &Path,
) -> AppResult<()> {
    rollback_guarded(journal, resources, path, || Ok(()))
}

pub fn rollback_guarded(
    journal: &mut Journal,
    resources: &mut impl Resources,
    path: &Path,
    gate: impl Fn() -> AppResult<()>,
) -> AppResult<()> {
    if journal.intent_count > journal.plan.actions.len()
        || journal.expected_digest()? != journal.digest
    {
        return Err(AppError::new("recoveryRequired"));
    }
    if journal.phase == Phase::RolledBack || journal.phase == Phase::Cancelled {
        return Ok(());
    }
    journal.phase = Phase::RollingBack;
    journal.save(path)?;
    let mut conflict = false;
    for action in journal.plan.actions[..journal.intent_count].iter().rev() {
        if gate().is_err() {
            conflict = true;
            break;
        }
        let restored = (|| {
            let current = resources.read(&action.target)?;
            if current == action.before {
                return Ok(());
            }
            if current != action.after {
                return Err(AppError::new("rollbackConflict"));
            }
            resources.write(&action.target, &action.before)?;
            if resources.read(&action.target)? != action.before {
                return Err(AppError::new("validationFailed"));
            }
            Ok(())
        })();
        if restored.is_err() {
            conflict = true;
        }
    }
    journal.phase = if conflict {
        Phase::RecoveryRequired
    } else {
        Phase::RolledBack
    };
    journal.save(path)?;
    if conflict {
        Err(AppError::new("recoveryRequired"))
    } else {
        Ok(())
    }
}

fn context_digest(plan: &OperationPlan, context: &Option<serde_json::Value>) -> AppResult<String> {
    if plan.actions.is_empty() && context.is_some() {
        plan.validate_context_only()?;
    } else {
        plan.validate()?;
    }
    let Some(context) = context else {
        return plan.digest();
    };
    let context_bytes = serde_json::to_vec(context).map_err(|_| AppError::new("invalidPlan"))?;
    if context_bytes.len() > 32 * 1024 {
        return Err(AppError::new("operationLimit"));
    }
    let bytes = serde_json::to_vec(&(plan, context)).map_err(|_| AppError::new("invalidPlan"))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn now() -> AppResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| AppError::new("clockInvalid"))
}

pub fn pending(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(_) => !Journal::load(path)
            .is_ok_and(|journal| journal.terminal() || journal.phase == Phase::Prepared),
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    }
}
