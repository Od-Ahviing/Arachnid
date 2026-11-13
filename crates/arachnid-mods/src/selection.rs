use crate::{Catalog, Recipe, parse_catalog};
use arachnid_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub schema_version: u32,
    pub enabled: BTreeSet<String>,
}

impl Selection {
    pub fn load(directory: &Path, catalog: &Catalog) -> AppResult<Self> {
        let _guard = arachnid_repair::filesystem::DirectoryGuard::acquire(directory)?;
        let path = arachnid_repair::filesystem::resolve(directory, "mod-selection.json")?;
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
            };
            options
                .share_mode(FILE_SHARE_READ)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        }
        let file = match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::defaults(catalog));
            }
            Err(_) => return Err(AppError::new("modSelectionUnavailable")),
        };
        let metadata = file
            .metadata()
            .map_err(|_| AppError::new("modSelectionUnavailable"))?;
        if !metadata.is_file() || arachnid_detector::is_link(&metadata) || metadata.len() > 8192 {
            return Err(AppError::new("invalidModSelection"));
        }
        let mut bytes = Vec::new();
        file.take(8193)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::new("modSelectionUnavailable"))?;
        if bytes.len() > 8192 {
            return Err(AppError::new("invalidModSelection"));
        }
        let selection: Self =
            serde_json::from_slice(&bytes).map_err(|_| AppError::new("invalidModSelection"))?;
        selection.validate(catalog)?;
        Ok(selection)
    }

    pub fn save(&self, directory: &Path, catalog: &Catalog) -> AppResult<()> {
        self.validate(catalog)?;
        let _guard = arachnid_repair::filesystem::DirectoryGuard::acquire(directory)?;
        Self::load(directory, catalog)?;
        let destination = arachnid_repair::filesystem::resolve(directory, "mod-selection.json")?;
        let mut staging = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| AppError::new("modSelectionUnavailable"))?;
        serde_json::to_writer(&mut staging, self)
            .map_err(|_| AppError::new("modSelectionUnavailable"))?;
        staging
            .flush()
            .and_then(|_| staging.as_file().sync_all())
            .map_err(|_| AppError::new("modSelectionUnavailable"))?;
        staging
            .persist(destination)
            .map_err(|_| AppError::new("modSelectionUnavailable"))?;
        Ok(())
    }

    pub fn defaults(catalog: &Catalog) -> Self {
        let members: BTreeSet<_> = catalog.groups.iter().flat_map(|g| &g.members).collect();
        Self {
            schema_version: 1,
            enabled: catalog
                .groups
                .iter()
                .filter(|g| g.default_enabled)
                .map(|g| g.id.clone())
                .chain(
                    catalog
                        .entries
                        .iter()
                        .filter(|r| r.default_enabled && !members.contains(&r.id))
                        .map(|r| r.id.clone()),
                )
                .collect(),
        }
    }

    pub fn validate(&self, catalog: &Catalog) -> AppResult<()> {
        let members: BTreeSet<_> = catalog.groups.iter().flat_map(|g| &g.members).collect();
        if self.schema_version != 1
            || self.enabled.len() > 32
            || self.enabled.iter().any(|id| {
                members.contains(id)
                    || !(catalog.groups.iter().any(|g| &g.id == id)
                        || catalog.entries.iter().any(|r| &r.id == id))
            })
        {
            return Err(AppError::new("invalidModSelection"));
        }
        Ok(())
    }

    pub fn set(&mut self, catalog: &Catalog, id: &str, enabled: bool) -> AppResult<()> {
        let mut next = self.clone();
        // Validate the identifier even when removing an entry that is not selected.
        next.enabled.insert(id.into());
        next.validate(catalog)?;
        if !enabled {
            next.enabled.remove(id);
        }
        *self = next;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub order: Vec<String>,
}

pub fn resolve(catalog: &Catalog, selection: &Selection) -> AppResult<Resolution> {
    // Public structures may also be constructed by Rust callers rather than deserialized.
    let bytes = serde_json::to_vec(catalog).map_err(|_| AppError::new("invalidModCatalog"))?;
    parse_catalog(&bytes)?;
    selection.validate(catalog)?;
    let entries: BTreeMap<_, _> = catalog.entries.iter().map(|r| (r.id.as_str(), r)).collect();
    let mut selected = BTreeSet::new();
    for id in &selection.enabled {
        if let Some(group) = catalog.groups.iter().find(|g| &g.id == id) {
            selected.extend(group.members.iter().map(String::as_str));
        } else {
            selected.insert(id.as_str());
        }
    }
    let mut visiting = BTreeSet::new();
    let mut complete = BTreeSet::new();
    let mut order = Vec::new();
    for id in selected {
        visit(id, &entries, &mut visiting, &mut complete, &mut order)?;
    }
    // A dependency cannot turn on only one component of a disabled group.
    for group in &catalog.groups {
        if group.members.iter().any(|m| complete.contains(m.as_str()))
            && !selection.enabled.contains(&group.id)
        {
            return Err(AppError::new("modGroupRequired"));
        }
    }
    let mut paths: BTreeSet<String> = BTreeSet::new();
    for id in &order {
        let recipe = entries[id.as_str()];
        if recipe
            .conflicts
            .iter()
            .any(|c| complete.contains(c.as_str()))
        {
            return Err(AppError::new("modConflict"));
        }
        for file in &recipe.files {
            let path = file.target.to_ascii_lowercase();
            if paths.iter().any(|existing| {
                existing == &path
                    || path.starts_with(&format!("{existing}/"))
                    || existing.starts_with(&format!("{path}/"))
            }) {
                return Err(AppError::new("modPathConflict"));
            }
            paths.insert(path);
        }
    }
    Ok(Resolution { order })
}

fn visit<'a>(
    id: &'a str,
    entries: &BTreeMap<&'a str, &'a Recipe>,
    visiting: &mut BTreeSet<&'a str>,
    complete: &mut BTreeSet<&'a str>,
    order: &mut Vec<String>,
) -> AppResult<()> {
    if complete.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(AppError::new("modDependencyCycle"));
    }
    let recipe = entries
        .get(id)
        .ok_or_else(|| AppError::new("modDependencyMissing"))?;
    let mut dependencies: Vec<_> = recipe.dependencies.iter().collect();
    dependencies.sort_by_key(|d| &d.id);
    for dependency in dependencies {
        let entry = entries
            .get(dependency.id.as_str())
            .ok_or_else(|| AppError::new("modDependencyMissing"))?;
        if entry.version.as_deref() != Some(&dependency.version) {
            return Err(AppError::new("modDependencyVersion"));
        }
        visit(&entry.id, entries, visiting, complete, order)?;
    }
    visiting.remove(id);
    complete.insert(id);
    order.push(id.into());
    Ok(())
}
