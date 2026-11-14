use arachnid_core::{AppError, AppResult};
use arachnid_repair::{
    Image, OperationPlan, Resources, Target,
    filesystem::{self, DirectoryGuard},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_PAYLOAD_BYTES: u64 = 16 * 1024 * 1024;
const MAX_STORE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payload {
    pub sha256: String,
    pub bytes: u64,
}

impl Payload {
    pub fn image(&self) -> AppResult<Image> {
        self.validate()?;
        Ok(Image {
            value_type: 0,
            bytes: serde_json::to_vec(self).map_err(|_| invalid())?,
        })
    }
    fn validate(&self) -> AppResult<()> {
        if !crate::hash(&self.sha256)
            || self.sha256 != self.sha256.to_ascii_lowercase()
            || self.bytes > MAX_PAYLOAD_BYTES
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(crate) fn from_image(image: &Image) -> AppResult<Self> {
        if image.value_type != 0 || image.bytes.len() > 256 {
            return Err(invalid());
        }
        let payload: Self = serde_json::from_slice(&image.bytes).map_err(|_| invalid())?;
        payload.validate()?;
        if payload.image()? != *image {
            return Err(invalid());
        }
        Ok(payload)
    }
}

fn invalid() -> AppError {
    AppError::new("invalidModPayload")
}

/// Adapter for the Repair journal: images describe verified blobs, not inline file contents.
/// Its journals must remain separate from native settings and SaveGuard transactions.
pub struct PayloadResources {
    root: PathBuf,
    store: PathBuf,
    _root_guard: DirectoryGuard,
    _store_guard: DirectoryGuard,
}

impl PayloadResources {
    pub fn observe(&self, target: &Target) -> AppResult<Option<Image>> {
        let Target::File { path } = target else {
            return Err(invalid());
        };
        let resolved = filesystem::resolve(&self.root, path)?;
        let _guard = DirectoryGuard::acquire(resolved.parent().ok_or_else(invalid)?)?;
        match open(&resolved) {
            Ok(mut file) => {
                let bytes = file.metadata().map_err(|_| invalid())?.len();
                if bytes > MAX_PAYLOAD_BYTES {
                    return Err(AppError::new("operationLimit"));
                }
                inspect(&mut file, bytes)?.image().map(Some)
            }
            Err(error) if error.code == "modFileAbsent" => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub fn new(root: &Path, store: &Path) -> AppResult<Self> {
        let root_guard = DirectoryGuard::acquire(root)?;
        let store_guard = DirectoryGuard::acquire(store)?;
        let root = root.canonicalize().map_err(|_| invalid())?;
        let store = store.canonicalize().map_err(|_| invalid())?;
        if root.starts_with(&store) || store.starts_with(&root) {
            return Err(invalid());
        }
        Ok(Self {
            root,
            store,
            _root_guard: root_guard,
            _store_guard: store_guard,
        })
    }

    pub fn import(&self, source: &Path, expected: &Payload) -> AppResult<Image> {
        expected.validate()?;
        let _guard = DirectoryGuard::acquire(source.parent().ok_or_else(invalid)?)?;
        let input = open(source)?;
        self.capture(input, Some(expected))?;
        expected.image()
    }

    pub fn validate_plan(plan: &OperationPlan) -> AppResult<()> {
        plan.validate()?;
        let mut total = 0u64;
        for action in &plan.actions {
            if !matches!(action.target, Target::File { .. }) {
                return Err(invalid());
            }
            for image in [&action.before, &action.after].into_iter().flatten() {
                total += Payload::from_image(image)?.bytes;
            }
        }
        if total > 128 * 1024 * 1024 {
            return Err(AppError::new("operationLimit"));
        }
        Ok(())
    }

    fn blob(&self, payload: &Payload) -> PathBuf {
        self.store.join(format!("{}.blob", payload.sha256))
    }

    pub(crate) fn verify_backup(&self, image: &Image) -> AppResult<()> {
        let payload = Payload::from_image(image)?;
        if inspect(&mut open(&self.blob(&payload))?, payload.bytes)? != payload {
            return Err(invalid());
        }
        Ok(())
    }

    pub(crate) fn read_config_backup(&self, image: &Image) -> AppResult<Vec<u8>> {
        let payload = Payload::from_image(image)?;
        if payload.bytes > 64 * 1024 {
            return Err(invalid());
        }
        let mut file = open(&self.blob(&payload))?;
        let mut bytes = Vec::new();
        if copy(&mut file, &mut bytes, payload.bytes)? != payload {
            return Err(invalid());
        }
        Ok(bytes)
    }

    fn capture(&self, mut source: File, expected: Option<&Payload>) -> AppResult<Payload> {
        let bytes = source.metadata().map_err(|_| invalid())?.len();
        if bytes > MAX_PAYLOAD_BYTES {
            return Err(AppError::new("operationLimit"));
        }
        let mut used = 0u64;
        let entries = fs::read_dir(&self.store).map_err(|_| invalid())?;
        let mut count = 0;
        for entry in entries {
            let entry = entry.map_err(|_| invalid())?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|_| invalid())?;
            if !metadata.is_file() || arachnid_detector::is_link(&metadata) {
                return Err(invalid());
            }
            used = used.checked_add(metadata.len()).ok_or_else(invalid)?;
            count += 1;
            if count > 128 {
                return Err(AppError::new("operationLimit"));
            }
        }
        if used.saturating_add(bytes) > MAX_STORE_BYTES {
            return Err(AppError::new("operationLimit"));
        }
        filesystem::ensure_space(&self.store, bytes.saturating_mul(2))?;
        let mut staging = tempfile::NamedTempFile::new_in(&self.store).map_err(|_| invalid())?;
        let payload = copy(&mut source, &mut staging, bytes)?;
        if expected.is_some_and(|expected| expected != &payload) {
            return Err(invalid());
        }
        staging.as_file().sync_all().map_err(|_| invalid())?;
        if inspect(&mut staging.reopen().map_err(|_| invalid())?, bytes)? != payload {
            return Err(invalid());
        }
        let destination = self.blob(&payload);
        match staging.persist_noclobber(&destination) {
            Ok(_) => (),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                if inspect(&mut open(&destination)?, bytes)? != payload {
                    return Err(invalid());
                }
            }
            Err(_) => return Err(invalid()),
        }
        Ok(payload)
    }
}

impl Resources for PayloadResources {
    fn read(&self, target: &Target) -> AppResult<Option<Image>> {
        let Target::File { path } = target else {
            return Err(invalid());
        };
        let resolved = filesystem::resolve(&self.root, path)?;
        let _guard = DirectoryGuard::acquire(resolved.parent().ok_or_else(invalid)?)?;
        match open(&resolved) {
            Ok(file) => self.capture(file, None)?.image().map(Some),
            Err(error) if error.code == "modFileAbsent" => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
        let Target::File { path } = target else {
            return Err(invalid());
        };
        let destination = filesystem::resolve(&self.root, path)?;
        let parent = destination.parent().ok_or_else(invalid)?;
        let _guard = DirectoryGuard::acquire(parent)?;
        if let Some(image) = image {
            let payload = Payload::from_image(image)?;
            filesystem::ensure_space(parent, payload.bytes.saturating_mul(2))?;
            let mut source = open(&self.blob(&payload))?;
            let mut staging = tempfile::NamedTempFile::new_in(parent)
                .map_err(|_| AppError::new("writeFailed"))?;
            if copy(&mut source, &mut staging, payload.bytes)? != payload {
                return Err(invalid());
            }
            staging.as_file().sync_all().map_err(|_| invalid())?;
            if inspect(&mut staging.reopen().map_err(|_| invalid())?, payload.bytes)? != payload {
                return Err(invalid());
            }
            staging
                .persist(&destination)
                .map_err(|_| AppError::new("writeFailed"))?;
        } else {
            match fs::remove_file(&destination) {
                Ok(()) => (),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err(AppError::new("writeFailed")),
            }
        }
        Ok(())
    }
}

fn open(path: &Path) -> AppResult<File> {
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
    let file = options.open(path).map_err(|error| {
        AppError::new(if error.kind() == std::io::ErrorKind::NotFound {
            "modFileAbsent"
        } else {
            "modFileUnreadable"
        })
    })?;
    let metadata = file.metadata().map_err(|_| invalid())?;
    if !metadata.is_file() || arachnid_detector::is_link(&metadata) {
        return Err(invalid());
    }
    Ok(file)
}

fn inspect(input: &mut impl Read, bytes: u64) -> AppResult<Payload> {
    copy(input, &mut std::io::sink(), bytes)
}

fn copy(input: &mut impl Read, output: &mut impl Write, bytes: u64) -> AppResult<Payload> {
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    let mut digest = Sha256::new();
    loop {
        let count = input.read(&mut buffer).map_err(|_| invalid())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > bytes || total > MAX_PAYLOAD_BYTES {
            return Err(invalid());
        }
        digest.update(&buffer[..count]);
        output.write_all(&buffer[..count]).map_err(|_| invalid())?;
    }
    if total != bytes {
        return Err(invalid());
    }
    Ok(Payload {
        sha256: digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        bytes,
    })
}
