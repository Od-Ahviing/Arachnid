use crate::{Artifact, RecipeFile, hash, parent_collision, relative_path};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::filesystem::DirectoryGuard;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

const MAX_ARCHIVE: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: usize = 256;

pub struct PreparedArchive {
    directory: tempfile::TempDir,
    files: BTreeMap<String, String>,
}

impl PreparedArchive {
    pub fn file(&self, target: &str) -> Option<std::path::PathBuf> {
        self.files
            .get(target)
            .map(|name| self.directory.path().join(name))
    }
}

fn invalid() -> AppError {
    AppError::new("invalidModArchive")
}

/// Produces only verified private blobs. It never writes into the game installation.
pub fn extract_zip(
    source: &Path,
    artifact: &Artifact,
    expected: &[RecipeFile],
    staging_parent: &Path,
) -> AppResult<PreparedArchive> {
    extract(source, artifact, expected, staging_parent, false)
}

/// Maintainer-only extraction of the aligned ZIP appended by the official ReShade setup.
pub fn extract_reshade_setup(
    source: &Path,
    artifact: &Artifact,
    expected: &[RecipeFile],
    staging_parent: &Path,
) -> AppResult<PreparedArchive> {
    extract(source, artifact, expected, staging_parent, true)
}

fn extract(
    source: &Path,
    artifact: &Artifact,
    expected: &[RecipeFile],
    staging_parent: &Path,
    reshade_setup: bool,
) -> AppResult<PreparedArchive> {
    if artifact.archive_bytes == 0
        || artifact.archive_bytes > MAX_ARCHIVE
        || artifact.extracted_bytes == 0
        || artifact.extracted_bytes > MAX_EXPANDED
        || !hash(&artifact.sha256)
        || expected.is_empty()
        || expected.len() > 32
    {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for file in expected {
        if !relative_path(&file.source)
            || !relative_path(&file.target)
            || !hash(&file.sha256)
            || file.bytes == 0
            || file.bytes > MAX_EXPANDED
            || !names.insert(file.source.to_ascii_lowercase())
            || !targets.insert(file.target.to_ascii_lowercase())
        {
            return Err(invalid());
        }
    }
    if parent_collision(&names) || parent_collision(&targets) {
        return Err(invalid());
    }
    let _source_guard = DirectoryGuard::acquire(source.parent().ok_or_else(invalid)?)?;
    let _staging_guard = DirectoryGuard::acquire(staging_parent)?;
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
    let mut input = options.open(source).map_err(|_| invalid())?;
    let metadata = input.metadata().map_err(|_| invalid())?;
    if !metadata.is_file()
        || arachnid_detector::is_link(&metadata)
        || metadata.len() != artifact.archive_bytes
    {
        return Err(invalid());
    }
    if digest(&mut input, artifact.archive_bytes)? != artifact.sha256.to_ascii_lowercase() {
        return Err(invalid());
    }
    input.seek(SeekFrom::Start(0)).map_err(|_| invalid())?;
    let mut base = 0;
    if reshade_setup {
        let mut block = [0u8; 512];
        loop {
            let count = input.read(&mut block).map_err(|_| invalid())?;
            if count < 30 {
                return Err(invalid());
            }
            if block[..4] == [0x50, 0x4b, 0x03, 0x04] && block[4..30].iter().any(|b| *b != 0) {
                break;
            }
            base += count as u64;
        }
    }
    input.seek(SeekFrom::Start(base)).map_err(|_| invalid())?;
    let reader = ArchiveReader { file: input, base };
    let mut archive = zip::ZipArchive::new(reader).map_err(|_| invalid())?;
    if archive.len() > MAX_ENTRIES
        || archive.has_duplicate_names()
        || archive.has_overlapping_entries().map_err(|_| invalid())?
    {
        return Err(invalid());
    }
    archive
        .validate_header_consistency()
        .map_err(|_| invalid())?;
    let mut paths = BTreeMap::new();
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|_| invalid())?;
        let name = entry.name().map_err(|_| invalid())?.into_owned();
        let path = if entry.is_dir() {
            name.strip_suffix('/').ok_or_else(invalid)?
        } else {
            &name
        };
        let kind = entry.unix_mode().unwrap_or(0) & 0o170000;
        if !relative_path(path)
            || path.split('/').count() > 8
            || entry.encrypted()
            || entry.is_symlink()
            || !matches!(kind, 0 | 0o100000 | 0o040000)
            || (entry.is_dir() && entry.size() != 0)
            || paths
                .insert(path.to_ascii_lowercase(), entry.is_dir())
                .is_some()
        {
            return Err(invalid());
        }
        expanded = expanded.checked_add(entry.size()).ok_or_else(invalid)?;
        if expanded > artifact.extracted_bytes {
            return Err(invalid());
        }
    }
    for path in paths.keys() {
        if path
            .match_indices('/')
            .any(|(position, _)| paths.get(&path[..position]) == Some(&false))
        {
            return Err(invalid());
        }
    }
    let directory = tempfile::Builder::new()
        .prefix("mod-staging-")
        .tempdir_in(staging_parent)
        .map_err(|_| invalid())?;
    let _private_guard = DirectoryGuard::acquire(directory.path())?;
    let mut files = BTreeMap::new();
    for (index, file) in expected.iter().enumerate() {
        let mut entry = archive.by_name(&file.source).map_err(|_| invalid())?;
        if entry.is_dir() || entry.size() != file.bytes {
            return Err(invalid());
        }
        // Output names are generated; no archive-controlled path is ever created.
        let name = format!("{index}.blob");
        let path = directory.path().join(&name);
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| invalid())?;
        let mut buffer = [0u8; 64 * 1024];
        let mut copied = 0u64;
        let mut hasher = Sha256::new();
        loop {
            let count = entry.read(&mut buffer).map_err(|_| invalid())?;
            if count == 0 {
                break;
            }
            copied = copied.checked_add(count as u64).ok_or_else(invalid)?;
            if copied > file.bytes {
                return Err(invalid());
            }
            hasher.update(&buffer[..count]);
            output.write_all(&buffer[..count]).map_err(|_| invalid())?;
        }
        if copied != file.bytes || hex(&hasher.finalize()) != file.sha256.to_ascii_lowercase() {
            return Err(invalid());
        }
        output.sync_all().map_err(|_| invalid())?;
        drop(output);
        if digest(&mut File::open(path).map_err(|_| invalid())?, file.bytes)?
            != file.sha256.to_ascii_lowercase()
        {
            return Err(invalid());
        }
        files.insert(file.target.clone(), name);
    }
    Ok(PreparedArchive { directory, files })
}

struct ArchiveReader {
    file: File,
    base: u64,
}

impl Read for ArchiveReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buffer)
    }
}

impl Seek for ArchiveReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let position = match position {
            SeekFrom::Start(offset) => SeekFrom::Start(
                self.base
                    .checked_add(offset)
                    .ok_or(std::io::ErrorKind::InvalidInput)?,
            ),
            other => other,
        };
        self.file
            .seek(position)?
            .checked_sub(self.base)
            .ok_or(std::io::ErrorKind::InvalidInput.into())
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest(input: &mut impl Read, expected: u64) -> AppResult<String> {
    let mut buffer = [0u8; 64 * 1024];
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    loop {
        let count = input.read(&mut buffer).map_err(|_| invalid())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > expected {
            return Err(invalid());
        }
        hasher.update(&buffer[..count]);
    }
    if total != expected {
        return Err(invalid());
    }
    Ok(hex(&hasher.finalize()))
}
