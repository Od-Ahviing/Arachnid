use arachnid_core::{AppError, AppResult, Preferences, SCHEMA_VERSION};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn record(&self, event: &'static str) {
        let Some(directory) = self.path.parent() else {
            return;
        };
        let path = directory.join("launcher.log");
        if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 256 * 1024)
            && fs::rename(&path, directory.join("launcher.previous.log")).is_err()
        {
            return;
        }
        if let Ok(mut log) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let _ = writeln!(log, "{timestamp} {event}");
        }
    }

    pub fn new(directory: &Path) -> AppResult<Self> {
        fs::create_dir_all(directory).map_err(|_| AppError::new("preferencesUnavailable"))?;
        Ok(Self {
            path: directory.join("preferences.json"),
        })
    }

    pub fn load(&self) -> AppResult<Preferences> {
        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Preferences::default());
            }
            Err(_) => return Err(AppError::new("preferencesUnavailable")),
        };
        let mut bytes = Vec::new();
        file.take(64 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::new("preferencesUnavailable"))?;
        if bytes.len() > 64 * 1024 {
            return Err(AppError::new("preferencesInvalid"));
        }
        let prefs: Preferences =
            serde_json::from_slice(&bytes).map_err(|_| AppError::new("preferencesInvalid"))?;
        if prefs.schema_version != SCHEMA_VERSION {
            return Err(AppError::new("preferencesVersion"));
        }
        Ok(prefs)
    }

    pub fn save(&self, preferences: &Preferences) -> AppResult<()> {
        // Refuse to replace a corrupt or newer configuration with defaults.
        self.load()?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| AppError::new("preferencesUnavailable"))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| AppError::new("preferencesUnavailable"))?;
        serde_json::to_writer_pretty(&mut file, preferences)
            .map_err(|_| AppError::new("preferencesUnavailable"))?;
        file.write_all(b"\n")
            .map_err(|_| AppError::new("preferencesUnavailable"))?;
        file.as_file()
            .sync_all()
            .map_err(|_| AppError::new("preferencesUnavailable"))?;
        file.persist(&self.path)
            .map_err(|_| AppError::new("preferencesUnavailable"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_future_schema_do_not_overwrite_data() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new(dir.path()).unwrap();
        assert_eq!(storage.load().unwrap(), Preferences::default());
        let prefs = Preferences {
            installation_path: Some("C:\\Test".into()),
            ..Preferences::default()
        };
        storage.save(&prefs).unwrap();
        assert_eq!(storage.load().unwrap(), prefs);
        fs::write(
            &storage.path,
            b"{\"schemaVersion\":99,\"language\":\"auto\",\"installationPath\":null}",
        )
        .unwrap();
        let before = fs::read(&storage.path).unwrap();
        assert!(storage.save(&Preferences::default()).is_err());
        assert_eq!(fs::read(&storage.path).unwrap(), before);
    }
    #[test]
    fn corrupt_preferences_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::new(dir.path()).unwrap();
        fs::write(&storage.path, b"{broken").unwrap();
        assert_eq!(storage.load().unwrap_err().code, "preferencesInvalid");
        assert!(storage.save(&Preferences::default()).is_err());
        assert_eq!(fs::read(&storage.path).unwrap(), b"{broken");
    }
}
