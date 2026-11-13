use crate::{Catalog, MAX_ARCHIVE_BYTES, hash, identifier, relative_path};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::filesystem::DirectoryGuard;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageIndex {
    schema_version: u32,
    packages: Vec<Package>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawIndex {
    schema_version: u32,
    packages: Vec<Package>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Package {
    pub id: String,
    pub payload: Option<Payload>,
    pub notice: Option<Payload>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    pub file_name: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Presence {
    NotPrepared,
    Missing,
    Invalid,
    Verified,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageStatus {
    pub id: String,
    pub presence: Presence,
    pub notice_presence: Presence,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineReport {
    pub packages: Vec<PackageStatus>,
    pub all_present: bool,
}

fn invalid() -> AppError {
    AppError::new("invalidOfflinePackages")
}

pub fn parse_index(bytes: &[u8], catalog: &Catalog) -> AppResult<PackageIndex> {
    if bytes.len() > 64 * 1024 {
        return Err(invalid());
    }
    let index: RawIndex = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if index.schema_version != 1 || index.packages.len() != catalog.entries.len() {
        return Err(invalid());
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for package in &index.packages {
        if !identifier(&package.id)
            || !ids.insert(&package.id)
            || !catalog.entries.iter().any(|entry| entry.id == package.id)
        {
            return Err(invalid());
        }
        for payload in [&package.payload, &package.notice].into_iter().flatten() {
            if !relative_path(&payload.file_name)
                || payload.file_name.contains('/')
                || !paths.insert(payload.file_name.to_ascii_lowercase())
                || !hash(&payload.sha256)
                || payload.bytes == 0
                || payload.bytes > MAX_ARCHIVE_BYTES
            {
                return Err(invalid());
            }
        }
    }
    Ok(PackageIndex {
        schema_version: index.schema_version,
        packages: index.packages,
    })
}

pub fn bundled_index(catalog: &Catalog) -> AppResult<PackageIndex> {
    parse_index(
        include_bytes!("../../../manifests/offline-packages.json"),
        catalog,
    )
}

pub(crate) fn open_payload(root: &Path, payload: &Payload) -> AppResult<Option<File>> {
    if !relative_path(&payload.file_name) || payload.file_name.contains('/') {
        return Err(invalid());
    }
    let path = root.join(&payload.file_name);
    let mut options = OpenOptions::new();
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
    let file = match options.open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AppError::new("offlineFileUnavailable")),
    };
    let metadata = file
        .metadata()
        .map_err(|_| AppError::new("offlineFileUnavailable"))?;
    if !metadata.is_file()
        || arachnid_detector::is_link(&metadata)
        || metadata.len() != payload.bytes
    {
        return Err(AppError::new("offlineFileInvalid"));
    }
    Ok(Some(file))
}

pub(crate) fn hash_stream(
    reader: &mut impl Read,
    expected: u64,
    mut output: Option<&mut File>,
) -> AppResult<String> {
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| AppError::new("offlineFileUnavailable"))?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or_else(invalid)?;
        if total > expected {
            return Err(AppError::new("offlineFileInvalid"));
        }
        digest.update(&buffer[..count]);
        if let Some(file) = output.as_deref_mut() {
            file.write_all(&buffer[..count])
                .map_err(|_| AppError::new("offlineWriteFailed"))?;
        }
    }
    if total != expected {
        return Err(AppError::new("offlineFileInvalid"));
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(crate) fn publish_payload(
    root: &Path,
    payload: &Payload,
    reader: &mut impl Read,
) -> AppResult<()> {
    if !relative_path(&payload.file_name)
        || payload.file_name.contains('/')
        || !hash(&payload.sha256)
        || payload.bytes == 0
        || payload.bytes > MAX_ARCHIVE_BYTES
    {
        return Err(invalid());
    }
    let _guard = DirectoryGuard::acquire(root)?;
    let mut staged =
        tempfile::NamedTempFile::new_in(root).map_err(|_| AppError::new("offlineWriteFailed"))?;
    if hash_stream(reader, payload.bytes, Some(staged.as_file_mut()))?
        != payload.sha256.to_ascii_lowercase()
    {
        return Err(AppError::new("offlineFileInvalid"));
    }
    staged
        .as_file()
        .sync_all()
        .map_err(|_| AppError::new("offlineWriteFailed"))?;
    if hash_stream(
        &mut staged
            .reopen()
            .map_err(|_| AppError::new("offlineWriteFailed"))?,
        payload.bytes,
        None,
    )? != payload.sha256.to_ascii_lowercase()
    {
        return Err(AppError::new("offlineFileInvalid"));
    }
    if let Some(mut existing) = open_payload(root, payload)? {
        if hash_stream(&mut existing, payload.bytes, None)? != payload.sha256.to_ascii_lowercase() {
            return Err(AppError::new("offlineFileInvalid"));
        }
    } else {
        staged
            .persist_noclobber(root.join(&payload.file_name))
            .map_err(|_| AppError::new("offlineWriteFailed"))?;
    }
    Ok(())
}

impl PackageIndex {
    pub fn inspect(&self, root: &Path) -> AppResult<OfflineReport> {
        let _guard = DirectoryGuard::acquire(root)?;
        let packages = self
            .packages
            .iter()
            .map(|package| {
                let inspect_payload = |payload: &Option<Payload>| match payload {
                    None => Presence::NotPrepared,
                    Some(payload) => match open_payload(root, payload) {
                        Ok(None) => Presence::Missing,
                        Ok(Some(mut file)) => match hash_stream(&mut file, payload.bytes, None) {
                            Ok(hash) if hash.eq_ignore_ascii_case(&payload.sha256) => {
                                Presence::Verified
                            }
                            _ => Presence::Invalid,
                        },
                        Err(_) => Presence::Invalid,
                    },
                };
                PackageStatus {
                    id: package.id.clone(),
                    presence: inspect_payload(&package.payload),
                    notice_presence: inspect_payload(&package.notice),
                }
            })
            .collect::<Vec<_>>();
        Ok(OfflineReport {
            all_present: !packages.is_empty()
                && packages
                    .iter()
                    .all(|package| package.presence == Presence::Verified),
            packages,
        })
    }

    /// Local preparation is separate from the release gate; no payload is executed or installed.
    pub fn stage_available(&self, source: &Path, destination: &Path) -> AppResult<OfflineReport> {
        let _source_guard = DirectoryGuard::acquire(source)?;
        let _destination_guard = DirectoryGuard::acquire(destination)?;
        for payload in self
            .packages
            .iter()
            .flat_map(|package| [&package.payload, &package.notice].into_iter().flatten())
        {
            let Some(mut input) = open_payload(source, payload)? else {
                continue;
            };
            if fs::symlink_metadata(destination.join(&payload.file_name)).is_ok() {
                let Some(mut existing) = open_payload(destination, payload)? else {
                    return Err(invalid());
                };
                if !hash_stream(&mut existing, payload.bytes, None)?
                    .eq_ignore_ascii_case(&payload.sha256)
                {
                    return Err(AppError::new("offlineFileInvalid"));
                }
                continue;
            }
            let mut staged = tempfile::NamedTempFile::new_in(destination)
                .map_err(|_| AppError::new("offlineWriteFailed"))?;
            if !hash_stream(&mut input, payload.bytes, Some(staged.as_file_mut()))?
                .eq_ignore_ascii_case(&payload.sha256)
            {
                return Err(AppError::new("offlineFileInvalid"));
            }
            staged
                .as_file_mut()
                .sync_all()
                .map_err(|_| AppError::new("offlineWriteFailed"))?;
            let mut readback = staged
                .reopen()
                .map_err(|_| AppError::new("offlineWriteFailed"))?;
            if !hash_stream(&mut readback, payload.bytes, None)?
                .eq_ignore_ascii_case(&payload.sha256)
            {
                return Err(AppError::new("offlineFileInvalid"));
            }
            staged
                .persist_noclobber(destination.join(&payload.file_name))
                .map_err(|_| AppError::new("offlineWriteFailed"))?;
        }
        self.inspect(destination)
    }

    pub fn verify_release(&self, catalog: &Catalog, root: &Path) -> AppResult<()> {
        let _guard = DirectoryGuard::acquire(root)?;
        let expected: BTreeSet<_> = self
            .packages
            .iter()
            .flat_map(|package| {
                [&package.payload, &package.notice]
                    .into_iter()
                    .flatten()
                    .map(|payload| payload.file_name.as_str())
            })
            .collect();
        for entry in fs::read_dir(root)
            .map_err(|_| AppError::new("offlineFileUnavailable"))?
            .take(expected.len() + 1)
        {
            let entry = entry.map_err(|_| AppError::new("offlineFileUnavailable"))?;
            if !entry
                .file_name()
                .to_str()
                .is_some_and(|name| expected.contains(name))
            {
                return Err(AppError::new("offlineUnexpectedFile"));
            }
        }
        let report = self.inspect(root)?;
        if !report.all_present {
            return Err(AppError::new("offlinePackagesIncomplete"));
        }
        if report
            .packages
            .iter()
            .any(|package| package.notice_presence != Presence::Verified)
        {
            return Err(AppError::new("offlineNoticesIncomplete"));
        }
        if catalog
            .entries
            .iter()
            .any(|entry| !entry.missing_evidence().is_empty())
        {
            return Err(AppError::new("offlineRecipesUnqualified"));
        }
        Ok(())
    }
}
