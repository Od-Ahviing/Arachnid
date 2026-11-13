use crate::{Image, MAX_IMAGE_BYTES};
use arachnid_core::{AppError, AppResult};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct DirectoryGuard {
    _handles: Vec<fs::File>,
}

impl DirectoryGuard {
    #[cfg(windows)]
    pub fn acquire(parent: &Path) -> AppResult<Self> {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
            FILE_SHARE_WRITE,
        };
        let absolute = std::path::absolute(parent).map_err(|_| AppError::new("invalidPath"))?;
        let mut handles = Vec::new();
        // Pin ancestors from the volume down so a parent cannot be replaced between validation and IO.
        for directory in absolute.ancestors().collect::<Vec<_>>().into_iter().rev() {
            let handle = fs::OpenOptions::new()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(directory)
                .map_err(|_| AppError::new("fileUnreadable"))?;
            let metadata = handle
                .metadata()
                .map_err(|_| AppError::new("fileUnreadable"))?;
            if !metadata.is_dir() || arachnid_detector::is_link(&metadata) {
                return Err(AppError::new("invalidPath"));
            }
            handles.push(handle);
        }
        Ok(Self { _handles: handles })
    }

    #[cfg(not(windows))]
    pub fn acquire(_: &Path) -> AppResult<Self> {
        Err(AppError::new("windowsOnly"))
    }
}

pub fn validate_relative(path: &str) -> AppResult<()> {
    if path.is_empty() || path.len() > 512 || path.contains(['\\', ':']) {
        return Err(AppError::new("invalidPath"));
    }
    for part in path.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with([' ', '.'])
            || part
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
            || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(AppError::new("invalidPath"));
        }
    }
    Ok(())
}

pub fn resolve(root: &Path, relative: &str) -> AppResult<PathBuf> {
    validate_relative(relative)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| AppError::new("invalidPath"))?;
    if !metadata.is_dir() || arachnid_detector::is_link(&metadata) {
        return Err(AppError::new("invalidPath"));
    }
    let mut path = root.to_path_buf();
    let parts: Vec<_> = relative.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) if arachnid_detector::is_link(&meta) => {
                return Err(AppError::new("invalidPath"));
            }
            Ok(meta) if index + 1 < parts.len() && !meta.is_dir() => {
                return Err(AppError::new("invalidPath"));
            }
            Err(error)
                if index + 1 == parts.len() && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(AppError::new("fileUnreadable")),
            _ => (),
        }
    }
    Ok(path)
}

pub fn read(root: &Path, relative: &str) -> AppResult<Option<Image>> {
    let path = resolve(root, relative)?;
    let _directories =
        DirectoryGuard::acquire(path.parent().ok_or_else(|| AppError::new("invalidPath"))?)?;
    resolve(root, relative)?;
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AppError::new("fileUnreadable")),
    };
    if !file.metadata().is_ok_and(|meta| {
        meta.is_file() && !arachnid_detector::is_link(&meta) && meta.len() <= MAX_IMAGE_BYTES as u64
    }) {
        return Err(AppError::new("operationLimit"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_IMAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::new("fileUnreadable"))?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(AppError::new("operationLimit"));
    }
    Ok(Some(Image {
        value_type: 0,
        bytes,
    }))
}

pub fn write(root: &Path, relative: &str, image: &Option<Image>) -> AppResult<()> {
    let path = resolve(root, relative)?;
    let _directories =
        DirectoryGuard::acquire(path.parent().ok_or_else(|| AppError::new("invalidPath"))?)?;
    resolve(root, relative)?;
    if let Some(image) = image {
        if image.value_type != 0 || image.bytes.len() > MAX_IMAGE_BYTES {
            return Err(AppError::new("invalidPlan"));
        }
        ensure_space(
            path.parent().ok_or_else(|| AppError::new("invalidPath"))?,
            image.bytes.len() as u64,
        )?;
        let mut staged = tempfile::NamedTempFile::new_in(
            path.parent().ok_or_else(|| AppError::new("invalidPath"))?,
        )
        .map_err(|_| AppError::new("writeFailed"))?;
        staged
            .write_all(&image.bytes)
            .and_then(|_| staged.as_file().sync_all())
            .map_err(|_| AppError::new("writeFailed"))?;
        resolve(root, relative)?;
        staged
            .persist(path)
            .map_err(|_| AppError::new("writeFailed"))?;
    } else {
        match fs::remove_file(path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(AppError::new("writeFailed")),
        }
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn pinned_ancestors_cannot_be_renamed_during_file_io() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        let guard = DirectoryGuard::acquire(&child).unwrap();
        assert!(fs::rename(&child, parent.join("moved")).is_err());
        assert!(fs::rename(&parent, directory.path().join("outside")).is_err());
        fs::write(child.join("fixture.txt"), b"creation remains possible").unwrap();
        drop(guard);
        fs::rename(&child, parent.join("moved")).unwrap();
    }

    #[test]
    fn quota_aware_space_check_refuses_impossible_request_and_unreadable_volume() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            ensure_space(directory.path(), u64::MAX).unwrap_err().code,
            "insufficientSpace"
        );
        assert_eq!(
            ensure_space(&directory.path().join("missing"), 0)
                .unwrap_err()
                .code,
            "spaceUnavailable"
        );
        ensure_space(directory.path(), 0).unwrap();
    }

    #[test]
    fn locked_target_refuses_replacement_without_changing_original_bytes() {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.ini");
        fs::write(&path, b"original").unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(&path)
            .unwrap();
        let replacement = Some(Image {
            value_type: 0,
            bytes: b"changed".to_vec(),
        });
        assert_eq!(
            write(directory.path(), "fixture.ini", &replacement)
                .unwrap_err()
                .code,
            "writeFailed"
        );
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        drop(lock);
        write(directory.path(), "fixture.ini", &replacement).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"changed");
    }

    #[test]
    fn existing_junction_cannot_redirect_a_read_or_write_outside_the_fixture() {
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("sentinel.ini"), b"outside").unwrap();
        let link = directory.path().join("redirect");
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:ARACHNID_TEST_LINK -Target $env:ARACHNID_TEST_TARGET | Out-Null"])
            .env("ARACHNID_TEST_LINK", &link)
            .env("ARACHNID_TEST_TARGET", outside.path())
            .status().unwrap();
        assert!(status.success());
        assert_eq!(
            read(directory.path(), "redirect/sentinel.ini")
                .unwrap_err()
                .code,
            "invalidPath"
        );
        assert_eq!(
            write(
                directory.path(),
                "redirect/sentinel.ini",
                &Some(Image {
                    value_type: 0,
                    bytes: b"changed".to_vec()
                })
            )
            .unwrap_err()
            .code,
            "invalidPath"
        );
        fs::remove_dir(&link).unwrap();
        assert_eq!(
            fs::read(outside.path().join("sentinel.ini")).unwrap(),
            b"outside"
        );
    }
}

#[cfg(windows)]
pub fn ensure_space(directory: &Path, bytes: u64) -> AppResult<()> {
    use std::os::windows::ffi::OsStrExt;
    let path: Vec<u16> = directory.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0;
    let success = unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            path.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if success == 0 {
        return Err(AppError::new("spaceUnavailable"));
    }
    if bytes
        .checked_add(4 * 1024 * 1024)
        .is_none_or(|required| available < required)
    {
        return Err(AppError::new("insufficientSpace"));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn ensure_space(_: &Path, _: u64) -> AppResult<()> {
    Err(AppError::new("windowsOnly"))
}
