use arachnid_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use url::Url;

pub mod archive;
pub mod composition;
pub mod effects;
pub mod fix_adapter;
pub mod fix_settings;
pub mod frame_limit;
pub mod manager;
pub mod offline;
pub mod operations;
pub mod payloads;
pub mod performance;
pub mod selection;
pub mod skins;

pub const MAX_CATALOG_BYTES: usize = 256 * 1024;
const MAX_ENTRIES: usize = 32;
const MAX_FILES: usize = 32;
const MAX_ARCHIVE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub game: String,
    pub entries: Vec<Recipe>,
    pub groups: Vec<ModGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModGroup {
    pub id: String,
    pub name: String,
    pub default_enabled: bool,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub author: String,
    pub source: String,
    pub kind: ModKind,
    pub default_enabled: bool,
    pub version: Option<String>,
    pub license: Option<License>,
    pub artifact: Option<Artifact>,
    pub compatibility: Vec<Compatibility>,
    pub files: Vec<RecipeFile>,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<String>,
    pub removal: Option<Removal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModKind {
    Wrapper,
    TexturePack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct License {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub url: String,
    pub sha256: String,
    pub archive_bytes: u64,
    pub extracted_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Compatibility {
    pub game_sha256: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecipeFile {
    pub source: String,
    pub target: String,
    pub sha256: String,
    pub bytes: u64,
    pub preimage: Preimage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Preimage {
    Absent,
    Sha256 { sha256: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Dependency {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Removal {
    RestorePreimages,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingEvidence {
    Version,
    License,
    Artifact,
    Compatibility,
    Files,
    Removal,
}

impl Recipe {
    /// Completeness is not installation authority: catalogue trust and local checks remain separate.
    pub fn missing_evidence(&self) -> Vec<MissingEvidence> {
        let mut missing = Vec::new();
        if self.version.is_none() {
            missing.push(MissingEvidence::Version);
        }
        if self.license.is_none() {
            missing.push(MissingEvidence::License);
        }
        if self.artifact.is_none() {
            missing.push(MissingEvidence::Artifact);
        }
        if self.compatibility.is_empty() {
            missing.push(MissingEvidence::Compatibility);
        }
        if self.files.is_empty() {
            missing.push(MissingEvidence::Files);
        }
        if self.removal.is_none() {
            missing.push(MissingEvidence::Removal);
        }
        missing
    }
}

fn invalid() -> AppError {
    AppError::new("invalidModCatalog")
}

fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn https(value: &str) -> bool {
    if !text(value, 2048) || value.contains('\\') {
        return false;
    }
    Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && url.port_or_known_default() == Some(443)
    })
}

fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && value.split('/').all(|part| {
            let stem = part
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            !part.is_empty()
                && part.is_ascii()
                && !part.ends_with([' ', '.'])
                && !part
                    .bytes()
                    .any(|b| b < 32 || b == 127 || b"\\:<>\"|?*".contains(&b))
                && !matches!(
                    stem.as_str(),
                    "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
                )
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        })
}

fn parent_collision(paths: &BTreeSet<String>) -> bool {
    paths.iter().any(|path| {
        path.match_indices('/')
            .any(|(position, _)| paths.contains(&path[..position]))
    })
}

pub fn parse_catalog(bytes: &[u8]) -> AppResult<Catalog> {
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(invalid());
    }
    let catalog: Catalog = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if catalog.schema_version != 1
        || catalog.game != "tasm1-pc-2012"
        || catalog.entries.len() > MAX_ENTRIES
        || catalog.groups.len() > MAX_ENTRIES
    {
        return Err(invalid());
    }
    let mut ids = BTreeSet::new();
    for recipe in &catalog.entries {
        if !identifier(&recipe.id)
            || !ids.insert(&recipe.id)
            || !text(&recipe.name, 160)
            || !text(&recipe.author, 160)
            || !https(&recipe.source)
            || recipe.version.as_ref().is_some_and(|v| !text(v, 64))
            || recipe
                .license
                .as_ref()
                .is_some_and(|l| !text(&l.name, 160) || !https(&l.source))
            || recipe.artifact.as_ref().is_some_and(|a| {
                !https(&a.url)
                    || !hash(&a.sha256)
                    || a.archive_bytes == 0
                    || a.archive_bytes > MAX_ARCHIVE_BYTES
                    || a.extracted_bytes == 0
                    || a.extracted_bytes > MAX_EXTRACTED_BYTES
            })
            || recipe.compatibility.len() > MAX_ENTRIES
            || recipe
                .compatibility
                .iter()
                .any(|c| !hash(&c.game_sha256) || !text(&c.evidence, 512))
            || recipe.dependencies.len() > MAX_ENTRIES
            || recipe
                .dependencies
                .iter()
                .any(|d| !identifier(&d.id) || !text(&d.version, 64) || d.id == recipe.id)
            || recipe.conflicts.len() > MAX_ENTRIES
            || recipe
                .conflicts
                .iter()
                .any(|id| !identifier(id) || id == &recipe.id)
            || recipe.files.len() > MAX_FILES
        {
            return Err(invalid());
        }
        let mut targets = BTreeSet::new();
        let mut sources = BTreeSet::new();
        let mut expanded = 0u64;
        for file in &recipe.files {
            if !relative_path(&file.source)
                || !relative_path(&file.target)
                || !sources.insert(file.source.to_ascii_lowercase())
                || !targets.insert(file.target.to_ascii_lowercase())
                || !hash(&file.sha256)
                || file.bytes == 0
                || file.bytes > MAX_EXTRACTED_BYTES
                || matches!(&file.preimage, Preimage::Sha256 { sha256 } if !hash(sha256))
            {
                return Err(invalid());
            }
            expanded = expanded.checked_add(file.bytes).ok_or_else(invalid)?;
        }
        if expanded > MAX_EXTRACTED_BYTES
            || recipe
                .artifact
                .as_ref()
                .is_some_and(|a| expanded > a.extracted_bytes)
            || parent_collision(&targets)
            || parent_collision(&sources)
        {
            return Err(invalid());
        }
    }
    let mut grouped = BTreeSet::new();
    for group in &catalog.groups {
        if !identifier(&group.id)
            || !ids.insert(&group.id)
            || !text(&group.name, 160)
            || group.members.is_empty()
            || group.members.len() > MAX_ENTRIES
        {
            return Err(invalid());
        }
        for member in &group.members {
            if !grouped.insert(member)
                || !catalog.entries.iter().any(|entry| {
                    &entry.id == member && !entry.default_enabled && entry.kind == ModKind::Wrapper
                })
            {
                return Err(invalid());
            }
        }
    }
    Ok(catalog)
}

pub fn bundled_catalog() -> AppResult<Catalog> {
    parse_catalog(include_bytes!("../../../manifests/community-catalog.json"))
}
