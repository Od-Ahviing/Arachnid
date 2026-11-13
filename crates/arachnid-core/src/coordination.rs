use crate::{AppError, AppResult};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub struct InstallationLease {
    _file: File,
    root: PathBuf,
}

impl InstallationLease {
    pub fn id(&self) -> String {
        use sha2::{Digest, Sha256};
        Sha256::digest(self.root.to_string_lossy().to_lowercase().as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
    pub fn acquire(root: &Path, locks: &Path) -> AppResult<Self> {
        use sha2::{Digest, Sha256};
        let root = root
            .canonicalize()
            .map_err(|_| AppError::new("directoryUnavailable"))?;
        let identity = root.to_string_lossy().to_lowercase();
        fs::create_dir_all(locks).map_err(|_| AppError::new("lockUnavailable"))?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(locks.join(format!(
                    "{}.lock",
                    Sha256::digest(identity.as_bytes())
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                )))
            .map_err(|_| AppError::new("lockUnavailable"))?;
        file.try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        Ok(Self { _file: file, root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclusive_lease_is_released_on_drop_and_scoped_by_installation() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let locks = tempfile::tempdir().unwrap();
        let lease = InstallationLease::acquire(root.path(), locks.path()).unwrap();
        assert!(InstallationLease::acquire(root.path(), locks.path()).is_err());
        assert!(InstallationLease::acquire(other.path(), locks.path()).is_ok());
        drop(lease);
        assert!(InstallationLease::acquire(root.path(), locks.path()).is_ok());
    }
}
