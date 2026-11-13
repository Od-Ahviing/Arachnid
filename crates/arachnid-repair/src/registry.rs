use crate::{Image, Resources, Target, filesystem};
use arachnid_core::{AppError, AppResult, coordination::InstallationLease};

pub struct InstallationResources<'a> {
    pub lease: &'a InstallationLease,
}

impl Resources for InstallationResources<'_> {
    fn read(&self, target: &Target) -> AppResult<Option<Image>> {
        match target {
            Target::File { path } => filesystem::read(self.lease.root(), path),
            Target::Registry { section, name } => read(section, name),
        }
    }
    fn write(&mut self, target: &Target, image: &Option<Image>) -> AppResult<()> {
        match target {
            Target::File { path } => filesystem::write(self.lease.root(), path, image),
            Target::Registry { section, name } => write(section, name, image),
        }
    }
}

fn key_path(section: &str, name: &str) -> AppResult<String> {
    let allowed = match section {
        "Display" => matches!(
            name,
            "Width"
                | "Height"
                | "Fullscreen"
                | "VSync"
                | "RefreshRate"
                | "QualitySettings"
                | "RenderingFlags"
                | "MaxAniso"
                | "AspectRatio"
                | "AntiAliasing"
                | "MotionBlur"
        ),
        "GameConfig" => matches!(
            name,
            "GraphicsPreset" | "FxPreset" | "LightningPreset" | "ShadowsPreset" | "CityLifeDensity"
        ),
        "Install" => name == "Language",
        _ => false,
    };
    if !allowed {
        return Err(AppError::new("unsupportedSetting"));
    }
    let path = if section == "Display" {
        "Settings\\Display"
    } else {
        section
    };
    Ok(format!("Software\\Activision\\ASM\\{path}"))
}

#[cfg(windows)]
pub fn section_exists(section: &str) -> AppResult<bool> {
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_32KEY},
    };
    let name = match section {
        "Display" => "Width",
        "GameConfig" => "GraphicsPreset",
        "Install" => "Language",
        _ => return Err(AppError::new("unsupportedSetting")),
    };
    match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(key_path(section, name)?, KEY_READ | KEY_WOW64_32KEY)
    {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(AppError::new("registryUnreadable")),
    }
}

#[cfg(not(windows))]
pub fn section_exists(_: &str) -> AppResult<bool> {
    Err(AppError::new("windowsOnly"))
}

#[cfg(windows)]
pub fn read(section: &str, name: &str) -> AppResult<Option<Image>> {
    read_at(&key_path(section, name)?, name)
}

#[cfg(windows)]
fn read_at(path: &str, name: &str) -> AppResult<Option<Image>> {
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_32KEY},
    };
    let key = match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(path, KEY_READ | KEY_WOW64_32KEY)
    {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(AppError::new("registryUnreadable")),
    };
    match key.get_raw_value(name) {
        Ok(value) if value.bytes.len() <= crate::MAX_IMAGE_BYTES => Ok(Some(Image {
            value_type: value.vtype as u32,
            bytes: value.bytes.to_vec(),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        _ => Err(AppError::new("registryUnreadable")),
    }
}

#[cfg(windows)]
fn write(section: &str, name: &str, image: &Option<Image>) -> AppResult<()> {
    write_at(&key_path(section, name)?, name, image)
}

#[cfg(windows)]
fn write_at(path: &str, name: &str, image: &Option<Image>) -> AppResult<()> {
    use winreg::{
        RegKey, RegValue,
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_32KEY, KEY_WRITE, RegType},
    };
    // Never create a missing game key: its original absence must survive a failed operation.
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(path, KEY_READ | KEY_WRITE | KEY_WOW64_32KEY)
        .map_err(|_| AppError::new("writeFailed"))?;
    if let Some(image) = image {
        let kind = match image.value_type {
            0 => RegType::REG_NONE,
            1 => RegType::REG_SZ,
            2 => RegType::REG_EXPAND_SZ,
            3 => RegType::REG_BINARY,
            4 => RegType::REG_DWORD,
            5 => RegType::REG_DWORD_BIG_ENDIAN,
            7 => RegType::REG_MULTI_SZ,
            11 => RegType::REG_QWORD,
            _ => return Err(AppError::new("unsupportedSetting")),
        };
        key.set_raw_value(
            name,
            &RegValue {
                bytes: image.bytes.clone().into(),
                vtype: kind,
            },
        )
        .map_err(|_| AppError::new("writeFailed"))?;
    } else {
        match key.delete_value(name) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(AppError::new("writeFailed")),
        }
    }
    if unsafe { windows_sys::Win32::System::Registry::RegFlushKey(key.raw_handle()) } != 0 {
        return Err(AppError::new("writeFailed"));
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WOW64_32KEY, KEY_WRITE},
    };

    #[test]
    fn windows_adapter_round_trips_raw_types_and_absence_in_an_isolated_test_key() {
        let path = format!(
            "Software\\ArachnidTests\\repair-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = root
            .create_subkey_with_flags(&path, KEY_READ | KEY_WRITE | KEY_WOW64_32KEY)
            .unwrap();
        for (kind, bytes) in [
            (3, vec![1, 2, 255]),
            (4, vec![0, 1, 0, 0]),
            (1, vec![65, 0, 0, 0]),
            (11, vec![1, 0, 0, 0, 0, 0, 0, 0]),
        ] {
            let value = Some(Image {
                value_type: kind,
                bytes,
            });
            assert!(read_at(&path, "fixture").unwrap().is_none());
            write_at(&path, "fixture", &value).unwrap();
            assert_eq!(read_at(&path, "fixture").unwrap(), value);
            write_at(&path, "fixture", &None).unwrap();
            assert!(read_at(&path, "fixture").unwrap().is_none());
        }
        drop(key);
        root.delete_subkey(&path).unwrap();
        assert!(read_at(&path, "fixture").unwrap().is_none());
        assert!(
            write_at(
                &path,
                "fixture",
                &Some(Image {
                    value_type: 4,
                    bytes: vec![0; 4]
                })
            )
            .is_err()
        );
    }
}

#[cfg(not(windows))]
pub fn read(section: &str, name: &str) -> AppResult<Option<Image>> {
    key_path(section, name)?;
    Err(AppError::new("windowsOnly"))
}
#[cfg(not(windows))]
fn write(_: &str, _: &str, _: &Option<Image>) -> AppResult<()> {
    Err(AppError::new("windowsOnly"))
}
