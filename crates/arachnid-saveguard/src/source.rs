use arachnid_core::{AppError, AppResult};
use arachnid_repair::{
    Image,
    filesystem::{self, DirectoryGuard},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const MAX_FILES: usize = 128;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub type Generation = BTreeMap<String, Image>;

pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn safe_directory(path: &Path) -> AppResult<PathBuf> {
    let _guard = DirectoryGuard::acquire(path)?;
    fs::canonicalize(path).map_err(|_| AppError::new("invalidPath"))
}

pub fn separate(source: &Path, destination: &Path) -> AppResult<()> {
    let source = safe_directory(source)?;
    let destination = safe_directory(destination)?;
    let source_key = PathBuf::from(source.to_string_lossy().to_lowercase());
    let destination_key = PathBuf::from(destination.to_string_lossy().to_lowercase());
    if source_key.starts_with(&destination_key) || destination_key.starts_with(&source_key) {
        return Err(AppError::new("savePathsOverlap"));
    }
    Ok(())
}

fn inventory(
    root: &Path,
    directory: &Path,
    files: &mut Vec<String>,
    depth: usize,
    visited: &mut usize,
) -> AppResult<()> {
    if depth > 8 {
        return Err(AppError::new("operationLimit"));
    }
    for entry in fs::read_dir(directory).map_err(|_| AppError::new("saveSourceUnavailable"))? {
        *visited += 1;
        if *visited > 256 {
            return Err(AppError::new("operationLimit"));
        }
        let entry = entry.map_err(|_| AppError::new("saveSourceUnavailable"))?;
        let metadata =
            fs::symlink_metadata(entry.path()).map_err(|_| AppError::new("saveUnstable"))?;
        if arachnid_detector::is_link(&metadata) {
            return Err(AppError::new("invalidPath"));
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| AppError::new("invalidPath"))?
            .to_str()
            .ok_or_else(|| AppError::new("invalidPath"))?
            .replace('\\', "/");
        filesystem::validate_relative(&relative)?;
        if metadata.is_dir() {
            // Empty directories have no restore semantics; only ordinary bounded file groups are copied.
            let _guard = DirectoryGuard::acquire(&path)?;
            inventory(root, &path, files, depth + 1, visited)?;
        } else if metadata.is_file() {
            files.push(relative);
            if files.len() > MAX_FILES {
                return Err(AppError::new("operationLimit"));
            }
        } else {
            return Err(AppError::new("invalidPath"));
        }
    }
    Ok(())
}

pub fn capture(root: &Path) -> AppResult<Generation> {
    let _root_guard = DirectoryGuard::acquire(root)?;
    let mut names = Vec::new();
    inventory(root, root, &mut names, 0, &mut 0)?;
    names.sort();
    let mut identities = std::collections::BTreeSet::new();
    let mut handles = Vec::new();
    let mut guards = Vec::new();
    let mut total = 0;
    for name in &names {
        if !identities.insert(name.to_lowercase()) {
            return Err(AppError::new("invalidPath"));
        }
        let path = filesystem::resolve(root, name)?;
        guards.push(DirectoryGuard::acquire(
            path.parent().ok_or_else(|| AppError::new("invalidPath"))?,
        )?);
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
            };
            options
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        }
        let handle = options
            .open(&path)
            .map_err(|_| AppError::new("saveUnstable"))?;
        let metadata = handle
            .metadata()
            .map_err(|_| AppError::new("saveUnstable"))?;
        if !metadata.is_file()
            || arachnid_detector::is_link(&metadata)
            || metadata.len() > arachnid_repair::MAX_IMAGE_BYTES as u64
        {
            return Err(AppError::new("operationLimit"));
        }
        total += metadata.len() as usize;
        if total > MAX_BYTES {
            return Err(AppError::new("operationLimit"));
        }
        handles.push((name, handle));
    }
    let mut generation = Generation::new();
    let mut copied_bytes = 0;
    for (name, handle) in handles.iter_mut() {
        let mut bytes = Vec::new();
        handle
            .take(arachnid_repair::MAX_IMAGE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::new("saveUnstable"))?;
        if bytes.len() > arachnid_repair::MAX_IMAGE_BYTES {
            return Err(AppError::new("operationLimit"));
        }
        copied_bytes += bytes.len();
        if copied_bytes > MAX_BYTES {
            return Err(AppError::new("operationLimit"));
        }
        generation.insert(
            (*name).clone(),
            Image {
                value_type: 0,
                bytes,
            },
        );
    }
    let mut after = Vec::new();
    inventory(root, root, &mut after, 0, &mut 0)?;
    after.sort();
    if names != after {
        return Err(AppError::new("saveUnstable"));
    }
    Ok(generation)
}

pub fn stable(root: &Path, pause: std::time::Duration) -> AppResult<Generation> {
    let before = capture(root)?;
    std::thread::sleep(pause);
    if capture(root)? != before {
        return Err(AppError::new("saveUnstable"));
    }
    Ok(before)
}
