use arachnid_core::{AppError, AppResult};
use arachnid_repair::{
    Image,
    filesystem::{self, DirectoryGuard},
};
use std::{io::Write, path::Path};

const PRESET: &str = "ArachnidPreset.ini";
const BACKUP: &str = "ReShade.before-defaults.ini";
const DEFAULT_PRESET: &str = concat!(
    "Techniques=SMAA@SMAA.fx,ArachnidColor@ArachnidColor.fx,LumaSharpen@LumaSharpen.fx\n",
    "TechniqueSorting=SMAA@SMAA.fx,ArachnidColor@ArachnidColor.fx,LumaSharpen@LumaSharpen.fx\n",
);

pub fn prepare_defaults(directory: &Path) -> AppResult<()> {
    let _guard = DirectoryGuard::acquire(directory)?;
    let current = filesystem::read(directory, "ReShade.ini")?
        .ok_or_else(|| AppError::new("settingAbsent"))?;
    let text = std::str::from_utf8(&current.bytes).map_err(|_| AppError::new("settingOpaque"))?;
    if setting(text, "ARACHNID", "DefaultsVersion") == Some("1") {
        return Ok(());
    }
    for (name, expected) in [
        (PRESET, DEFAULT_PRESET.as_bytes()),
        (BACKUP, current.bytes.as_slice()),
    ] {
        if filesystem::read(directory, name)?.is_some_and(|image| image.bytes != expected) {
            return Err(AppError::new("modUnmanagedFiles"));
        }
    }
    publish_once(directory, BACKUP, &current.bytes)?;
    publish_once(directory, PRESET, DEFAULT_PRESET.as_bytes())?;
    let updated = set_setting(text, "GENERAL", "PresetPath", r".\ArachnidPreset.ini");
    let updated = set_setting(&updated, "GENERAL", "StartupPresetPath", "");
    let updated = set_setting(&updated, "OVERLAY", "TutorialProgress", "4");
    let updated = set_setting(&updated, "ARACHNID", "DefaultsVersion", "1");
    if filesystem::read(directory, "ReShade.ini")?.as_ref() != Some(&current) {
        return Err(AppError::new("stalePlan"));
    }
    let next = Some(Image {
        value_type: 0,
        bytes: updated.into_bytes(),
    });
    filesystem::write(directory, "ReShade.ini", &next)?;
    if filesystem::read(directory, "ReShade.ini")? != next {
        return Err(AppError::new("validationFailed"));
    }
    Ok(())
}

fn publish_once(directory: &Path, name: &str, bytes: &[u8]) -> AppResult<()> {
    if let Some(image) = filesystem::read(directory, name)? {
        return if image.bytes == bytes {
            Ok(())
        } else {
            Err(AppError::new("modUnmanagedFiles"))
        };
    }
    let path = filesystem::resolve(directory, name)?;
    let mut file =
        tempfile::NamedTempFile::new_in(directory).map_err(|_| AppError::new("writeFailed"))?;
    file.write_all(bytes)
        .map_err(|_| AppError::new("writeFailed"))?;
    file.as_file()
        .sync_all()
        .map_err(|_| AppError::new("writeFailed"))?;
    file.persist_noclobber(path)
        .map_err(|_| AppError::new("writeFailed"))?;
    if filesystem::read(directory, name)?.is_none_or(|image| image.bytes != bytes) {
        return Err(AppError::new("validationFailed"));
    }
    Ok(())
}

fn setting<'a>(text: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut current = "";
    let mut result = None;
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            current = name;
        } else if current.eq_ignore_ascii_case(section)
            && let Some((name, value)) = line.split_once('=')
            && name.trim().eq_ignore_ascii_case(key)
        {
            result = Some(value.trim());
        }
    }
    result
}

fn set_setting(text: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut current = false;
    let mut found = false;
    let mut insertion = None;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches('\u{feff}');
        if trimmed.starts_with('[') {
            current = trimmed.eq_ignore_ascii_case(&format!("[{section}]"));
            if current {
                insertion = Some(lines.len() + 1);
            }
        }
        if current
            && line
                .split_once('=')
                .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case(key))
        {
            if !found {
                lines.push(format!("{key}={value}"));
            }
            found = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !found {
        if let Some(index) = insertion {
            lines.insert(index, format!("{key}={value}"));
        } else {
            lines.extend([format!("[{section}]"), format!("{key}={value}")]);
        }
    }
    lines.join(newline) + newline
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn defaults_enable_only_the_requested_effects_and_skip_the_tutorial() {
        let dir = tempfile::tempdir().unwrap();
        let original = "[GENERAL]\r\nEffectSearchPaths=.\\shaders,.\\community-effects\\shaders\r\n[INPUT]\r\nKeyOverlay=36,0,0,0\r\n";
        fs::write(dir.path().join("ReShade.ini"), original).unwrap();
        prepare_defaults(dir.path()).unwrap();
        let config = fs::read_to_string(dir.path().join("ReShade.ini")).unwrap();
        assert_eq!(
            setting(&config, "GENERAL", "PresetPath"),
            Some(r".\ArachnidPreset.ini")
        );
        assert_eq!(setting(&config, "OVERLAY", "TutorialProgress"), Some("4"));
        assert_eq!(setting(&config, "INPUT", "KeyOverlay"), Some("36,0,0,0"));
        assert!(config.contains("EffectSearchPaths=.\\shaders,.\\community-effects\\shaders\r\n"));
        assert_eq!(
            fs::read_to_string(dir.path().join(BACKUP)).unwrap(),
            original
        );
        let preset = fs::read_to_string(dir.path().join(PRESET)).unwrap();
        let techniques = setting(&preset, "", "Techniques").unwrap();
        assert_eq!(techniques.split(',').count(), 3);
        for effect in [
            "SMAA@SMAA.fx",
            "ArachnidColor@ArachnidColor.fx",
            "LumaSharpen@LumaSharpen.fx",
        ] {
            assert!(techniques.split(',').any(|name| name == effect));
        }
        assert!(!preset.contains("MXAO"));
        assert!(
            include_str!("../resources/shaders/ArachnidColor.fx")
                .contains("technique ArachnidColor")
        );
    }

    #[test]
    fn later_user_choices_and_external_presets_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let original = "[GENERAL]\nPresetPath=C:\\Personal\\MyPreset.ini\nStartupPresetPath=C:\\Personal\\Startup.ini\n";
        fs::write(dir.path().join("ReShade.ini"), original).unwrap();
        prepare_defaults(dir.path()).unwrap();
        let mut config = fs::read_to_string(dir.path().join("ReShade.ini")).unwrap();
        assert_eq!(setting(&config, "GENERAL", "StartupPresetPath"), Some(""));
        config = set_setting(
            &config,
            "GENERAL",
            "PresetPath",
            r"C:\Personal\MyPreset.ini",
        );
        fs::write(dir.path().join("ReShade.ini"), &config).unwrap();
        fs::write(dir.path().join(PRESET), "Techniques=SMAA@SMAA.fx\n").unwrap();
        prepare_defaults(dir.path()).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("ReShade.ini")).unwrap(),
            config
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(PRESET)).unwrap(),
            "Techniques=SMAA@SMAA.fx\n"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join(BACKUP)).unwrap(),
            original
        );
    }

    #[test]
    fn conflicts_and_interrupted_preparation_do_not_overwrite_user_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ReShade.ini");
        let original = "[GENERAL]\nEffectSearchPaths=.\\shaders\n";
        fs::write(&path, original).unwrap();
        fs::write(dir.path().join(PRESET), "personal preset").unwrap();
        assert!(prepare_defaults(dir.path()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(
            fs::read_to_string(dir.path().join(PRESET)).unwrap(),
            "personal preset"
        );
        fs::write(dir.path().join(PRESET), DEFAULT_PRESET).unwrap();
        fs::write(dir.path().join(BACKUP), original).unwrap();
        prepare_defaults(dir.path()).unwrap();
        assert_eq!(
            setting(
                &fs::read_to_string(path).unwrap(),
                "ARACHNID",
                "DefaultsVersion"
            ),
            Some("1")
        );
    }

    #[test]
    fn invalid_config_and_conflicting_backup_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ReShade.ini");
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert!(prepare_defaults(dir.path()).is_err());
        assert_eq!(fs::read(&path).unwrap(), [0xff, 0xfe]);
        let original = "[GENERAL]\n";
        fs::write(&path, original).unwrap();
        fs::write(dir.path().join(BACKUP), "personal backup").unwrap();
        assert!(prepare_defaults(dir.path()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(
            fs::read_to_string(dir.path().join(BACKUP)).unwrap(),
            "personal backup"
        );
        assert!(!dir.path().join(PRESET).exists());
    }
}
