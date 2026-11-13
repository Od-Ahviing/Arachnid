use crate::{SourceProfile, source};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::filesystem::DirectoryGuard;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const SAVE_NAME: &str = "asm_save.fpop";
const APP_ID: &str = "212580";

pub struct Discovery {
    pub installation: Option<PathBuf>,
    candidate: PathBuf,
}

impl Discovery {
    pub fn native(documents: &Path) -> Self {
        Self {
            installation: None,
            candidate: documents.join("Activision/ASM/Save"),
        }
    }

    pub fn new(documents: &Path, roaming: &Path, installation: Option<&Path>) -> AppResult<Self> {
        let mut detection = Self::native(documents);
        detection.installation = installation.map(Path::to_path_buf);
        if let Some(root) = installation {
            let app_id = read_optional(root, "steam_settings/steam_appid.txt", 64)?;
            if app_id.is_some_and(|bytes| {
                String::from_utf8_lossy(&bytes)
                    .trim()
                    .trim_start_matches('\u{feff}')
                    == APP_ID
            }) && uses_gse_storage(root)?
            {
                detection.candidate = roaming.join("GSE Saves").join(APP_ID).join("remote");
            }
        }
        Ok(detection)
    }

    pub fn matches_source(&self, path: &Path) -> bool {
        source::safe_directory(&self.candidate).is_ok_and(|candidate| {
            candidate
                .to_string_lossy()
                .eq_ignore_ascii_case(&path.to_string_lossy())
        })
    }

    pub fn find(&self, backup_root: &Path) -> AppResult<Option<SourceProfile>> {
        let file = self.candidate.join(SAVE_NAME);
        match fs::symlink_metadata(&file) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(AppError::new("saveSourceUnavailable")),
            Ok(metadata) => {
                if !metadata.is_file()
                    || arachnid_detector::is_link(&metadata)
                    || metadata.len() == 0
                {
                    return Ok(None);
                }
                let mut profile = SourceProfile::new(&self.candidate, backup_root)?;
                profile.enabled = true;
                profile.automatic = true;
                Ok(Some(profile))
            }
        }
    }
}

fn open_optional(root: &Path, relative: &str) -> AppResult<Option<fs::File>> {
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AppError::new("fileUnreadable")),
        Ok(_) => (),
    }
    let _guard =
        DirectoryGuard::acquire(path.parent().ok_or_else(|| AppError::new("invalidPath"))?)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        };
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE);
    }
    let file = options
        .open(&path)
        .map_err(|_| AppError::new("fileUnreadable"))?;
    if !file
        .metadata()
        .is_ok_and(|m| m.is_file() && !arachnid_detector::is_link(&m))
    {
        return Err(AppError::new("invalidPath"));
    }
    Ok(Some(file))
}

fn read_optional(root: &Path, relative: &str, limit: usize) -> AppResult<Option<Vec<u8>>> {
    let Some(file) = open_optional(root, relative)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::new("fileUnreadable"))?;
    if bytes.len() > limit {
        return Err(AppError::new("operationLimit"));
    }
    Ok(Some(bytes))
}

fn uses_gse_storage(root: &Path) -> AppResult<bool> {
    let Some(mut file) = open_optional(root, "steam_api.dll")? else {
        return Ok(false);
    };
    const LIMIT: u64 = 32 * 1024 * 1024;
    if file
        .metadata()
        .map_err(|_| AppError::new("fileUnreadable"))?
        .len()
        > LIMIT
    {
        return Err(AppError::new("operationLimit"));
    }
    let marker = b"GSE Saves";
    let mut buffer = [0_u8; 8192 + 8];
    let mut retained = 0;
    let mut total = 0;
    loop {
        let count = file
            .read(&mut buffer[retained..])
            .map_err(|_| AppError::new("fileUnreadable"))?;
        if count == 0 {
            return Ok(false);
        }
        total += count as u64;
        if total > LIMIT {
            return Err(AppError::new("operationLimit"));
        }
        let end = retained + count;
        if buffer[..end]
            .windows(marker.len())
            .any(|bytes| bytes == marker)
        {
            return Ok(true);
        }
        retained = end.min(marker.len() - 1);
        buffer.copy_within(end - retained..end, 0);
    }
}
