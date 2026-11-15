use arachnid_core::{
    AppError, AppResult, Check, CheckStatus, FileEntry, PeInfo, SCHEMA_VERSION, ScanProgress,
    ScanReport,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_FILES: usize = 100_000;
const MAX_PE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocalReference {
    schema_version: u32,
    source: String,
    game_sha256: String,
    files: Vec<ReferenceFile>,
}

#[derive(serde::Deserialize)]
struct ReferenceFile {
    name: String,
    bytes: u64,
}

fn compare_reference(report: &mut ScanReport) -> AppResult<()> {
    let reference: LocalReference = serde_json::from_str(include_str!(
        "../../../manifests/tasm1-local-structure.json"
    ))
    .map_err(|_| AppError::new("referenceInvalid"))?;
    if reference.schema_version != 1
        || reference.source.is_empty()
        || reference.files.is_empty()
        || reference
            .files
            .iter()
            .any(|f| f.name.contains("..") || f.name.contains('\\') || f.name.starts_with('/'))
    {
        return Err(AppError::new("referenceInvalid"));
    }
    let matching_build = report
        .binaries
        .iter()
        .any(|b| b.name.eq_ignore_ascii_case("Game.exe") && b.sha256 == reference.game_sha256);
    if !matching_build {
        report.checks.push(Check::new(
            "referenceUnknown",
            CheckStatus::Unverified,
            None,
        ));
        return Ok(());
    }
    let observed: std::collections::HashMap<_, _> = report
        .files
        .iter()
        .map(|f| (f.name.to_ascii_lowercase(), f.bytes))
        .collect();
    let differences = reference
        .files
        .iter()
        .filter(|f| observed.get(&f.name.to_ascii_lowercase()) != Some(&f.bytes))
        .count();
    report.checks.push(Check::new(
        "referenceResources",
        if differences == 0 {
            CheckStatus::Passed
        } else {
            CheckStatus::Error
        },
        Some(differences.to_string()),
    ));
    Ok(())
}

pub fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn check_cancel(cancel: &AtomicBool) -> AppResult<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(AppError::new("cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn has_local_disk_prefix(path: &Path) -> bool {
    use std::path::Prefix;

    matches!(
        path.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    )
}

pub fn validate_root(path: &Path) -> AppResult<std::path::PathBuf> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AppError::new("invalidPath"));
    }
    #[cfg(windows)]
    if !has_local_disk_prefix(path)
        || path.components().any(|component| {
            matches!(component, Component::Normal(part) if part.to_string_lossy().contains([':', '/']))
        })
    {
        return Err(AppError::new("invalidPath"));
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| AppError::new("directoryUnavailable"))?;
    if !metadata.is_dir() || is_link(&metadata) {
        return Err(AppError::new("invalidPath"));
    }
    let root = path
        .canonicalize()
        .map_err(|_| AppError::new("directoryUnavailable"))?;
    #[cfg(windows)]
    if !has_local_disk_prefix(&root) {
        return Err(AppError::new("invalidPath"));
    }
    Ok(root)
}

pub fn inspect_pe(path: &Path, name: &str, cancel: &AtomicBool) -> AppResult<PeInfo> {
    let before = fs::symlink_metadata(path).map_err(|_| AppError::new("fileUnreadable"))?;
    if !before.is_file() || is_link(&before) || before.len() > 1024 * 1024 * 1024 {
        return Err(AppError::new("invalidBinary"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Keep a writer from changing the binary while its hash and headers are read.
        options.share_mode(1);
    }
    let mut file = options
        .open(path)
        .map_err(|_| AppError::new("fileUnreadable"))?;
    let imports_inspected = before.len() <= MAX_PE_BYTES;
    let buffer_limit = if imports_inspected {
        before.len() as usize
    } else {
        64 * 1024
    };
    let mut bytes = Vec::with_capacity(buffer_limit);
    let mut total_read = 0u64;
    let mut hash = Sha256::new();
    let mut chunk = [0; 64 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = file
            .read(&mut chunk)
            .map_err(|_| AppError::new("fileUnreadable"))?;
        if count == 0 {
            break;
        }
        total_read += count as u64;
        if total_read > before.len() {
            return Err(AppError::new("fileChanged"));
        }
        hash.update(&chunk[..count]);
        let retained = count.min(buffer_limit.saturating_sub(bytes.len()));
        bytes.extend_from_slice(&chunk[..retained]);
    }
    let after = file.metadata().map_err(|_| AppError::new("fileChanged"))?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || total_read != after.len()
    {
        return Err(AppError::new("fileChanged"));
    }
    let header =
        goblin::pe::header::Header::parse(&bytes).map_err(|_| AppError::new("invalidBinary"))?;
    let optional = header
        .optional_header
        .as_ref()
        .ok_or_else(|| AppError::new("invalidBinary"))?;
    if header.coff_header.number_of_sections == 0
        || header.coff_header.number_of_sections > 96
        || (name.eq_ignore_ascii_case("Game.exe")
            && header.coff_header.characteristics & 0x2000 != 0)
    {
        return Err(AppError::new("invalidBinary"));
    }
    let architecture = match (header.coff_header.machine, optional.standard_fields.magic) {
        (0x14c, 0x10b) => "x86",
        (0x8664, 0x20b) => "x64",
        _ => "unknown",
    };
    let mut section_offset = header.dos_header.pe_pointer as usize
        + 24
        + header.coff_header.size_of_optional_header as usize;
    for _ in 0..header.coff_header.number_of_sections {
        let section =
            goblin::pe::section_table::SectionTable::parse(&bytes, &mut section_offset, 0)
                .map_err(|_| AppError::new("invalidBinary"))?;
        if u64::from(section.pointer_to_raw_data) + u64::from(section.size_of_raw_data)
            > after.len()
        {
            return Err(AppError::new("invalidBinary"));
        }
    }
    let mut imports: Vec<String> = if imports_inspected {
        goblin::pe::PE::parse(&bytes)
            .map_err(|_| AppError::new("invalidBinary"))?
            .libraries
            .iter()
            .map(|name| name.to_string())
            .collect()
    } else {
        Vec::new()
    };
    imports.sort();
    imports.dedup();
    Ok(PeInfo {
        name: name.into(),
        architecture: architecture.into(),
        sha256: hash
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        bytes: after.len(),
        imports,
        imports_inspected,
        large_address_aware: header.coff_header.characteristics & 0x20 != 0,
    })
}

pub fn scan(
    path: &Path,
    cancel: &AtomicBool,
    progress: impl Fn(ScanProgress),
) -> AppResult<ScanReport> {
    let root = validate_root(path)?;
    let mut report = ScanReport {
        schema_version: SCHEMA_VERSION,
        root: root
            .display()
            .to_string()
            .trim_start_matches(r"\\?\")
            .to_string(),
        scanned_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        recognized: false,
        verification: "INVALID".into(),
        repair_status: "UNKNOWN".into(),
        files: vec![],
        file_count: 0,
        total_bytes: 0,
        pkz_count: 0,
        binaries: vec![],
        checks: vec![],
    };
    let mut pending = vec![(root.clone(), 0)];
    let mut skipped = false;
    let mut entries_seen = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        check_cancel(cancel)?;
        if depth > 24 {
            return Err(AppError::new("scanLimit"));
        }
        let entries = fs::read_dir(&directory).map_err(|_| AppError::new("fileUnreadable"))?;
        for entry in entries {
            entries_seen += 1;
            if entries_seen > MAX_FILES {
                return Err(AppError::new("scanLimit"));
            }
            check_cancel(cancel)?;
            let entry = entry.map_err(|_| AppError::new("fileUnreadable"))?;
            let metadata =
                fs::symlink_metadata(entry.path()).map_err(|_| AppError::new("fileUnreadable"))?;
            if is_link(&metadata) {
                skipped = true;
                continue;
            }
            if metadata.is_dir() {
                pending.push((entry.path(), depth + 1));
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            if report.files.len() >= MAX_FILES {
                return Err(AppError::new("scanLimit"));
            }
            let name = entry
                .path()
                .strip_prefix(&root)
                .map_err(|_| AppError::new("invalidPath"))?
                .to_string_lossy()
                .replace('\\', "/");
            report.total_bytes = report
                .total_bytes
                .checked_add(metadata.len())
                .ok_or_else(|| AppError::new("scanLimit"))?;
            report.files.push(FileEntry {
                name,
                bytes: metadata.len(),
            });
            if report.files.len().is_multiple_of(64) {
                progress(ScanProgress {
                    stage: "inventory".into(),
                    files_scanned: report.files.len(),
                });
            }
        }
    }
    report.files.sort_by(|a, b| a.name.cmp(&b.name));
    report.pkz_count = report
        .files
        .iter()
        .filter(|f| {
            f.name.to_ascii_lowercase().starts_with("data/")
                && f.name.to_ascii_lowercase().ends_with(".pkz")
                && f.bytes > 0
        })
        .count();
    let shaders = report
        .files
        .iter()
        .any(|f| f.name.to_ascii_lowercase().starts_with("data/shaders/") && f.bytes > 0);
    let game = report
        .files
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case("Game.exe"))
        .map(|f| f.name.clone());
    let mut game_valid = false;
    for entry in report.files.iter().filter(|f| {
        f.name.to_ascii_lowercase().ends_with(".exe")
            || f.name.to_ascii_lowercase().ends_with(".dll")
    }) {
        check_cancel(cancel)?;
        progress(ScanProgress {
            stage: "binaries".into(),
            files_scanned: report.files.len(),
        });
        match inspect_pe(&root.join(&entry.name), &entry.name, cancel) {
            Ok(binary) => {
                if entry.name.eq_ignore_ascii_case("Game.exe") {
                    game_valid = binary.architecture == "x86";
                    report.checks.push(Check::new(
                        "gameBinary",
                        if game_valid {
                            CheckStatus::Passed
                        } else {
                            CheckStatus::Error
                        },
                        Some(binary.architecture.clone()),
                    ));
                } else if binary.architecture != "x86" {
                    report.checks.push(Check::new(
                        "binaryArchitecture",
                        CheckStatus::Warning,
                        Some(entry.name.clone()),
                    ));
                }
                report.binaries.push(binary);
            }
            Err(error) if error.code == "cancelled" || error.code == "fileChanged" => {
                return Err(error);
            }
            Err(_) => report.checks.push(Check::new(
                if entry.name.eq_ignore_ascii_case("Game.exe") {
                    "gameBinary"
                } else {
                    "localBinary"
                },
                if entry.name.eq_ignore_ascii_case("ASMLauncher.exe")
                    || entry.name.eq_ignore_ascii_case("VideoDetection.dll")
                    || entry
                        .name
                        .to_ascii_lowercase()
                        .ends_with("/asmlauncher.resources.dll")
                {
                    CheckStatus::Warning
                } else {
                    CheckStatus::Error
                },
                Some(entry.name.clone()),
            )),
        }
    }
    if game.is_none() {
        report
            .checks
            .push(Check::new("gameBinary", CheckStatus::Error, None));
    }
    report.checks.push(Check::new(
        "resources",
        if report.pkz_count > 0 {
            CheckStatus::Passed
        } else {
            CheckStatus::Error
        },
        Some(report.pkz_count.to_string()),
    ));
    report.checks.push(Check::new(
        "shaders",
        if shaders {
            CheckStatus::Passed
        } else {
            CheckStatus::Error
        },
        None,
    ));
    for (name, code, required) in [
        ("steam_api.dll", "gameLibrary", true),
        ("GraphicsPreset.xml", "graphicsPresets", false),
        ("VideoDetection.dll", "videoDetection", false),
    ] {
        let present = report
            .files
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case(name) && f.bytes > 0);
        report.checks.push(Check::new(
            code,
            if present {
                CheckStatus::Passed
            } else if required {
                CheckStatus::Error
            } else {
                CheckStatus::Warning
            },
            None,
        ));
    }
    if skipped {
        report
            .checks
            .push(Check::new("linksSkipped", CheckStatus::Warning, None));
    }
    let wrapper = report
        .files
        .iter()
        .any(|f| f.name.eq_ignore_ascii_case("d3d9.dll"));
    report.checks.push(Check::new(
        "wrapper",
        if wrapper {
            CheckStatus::Unverified
        } else {
            CheckStatus::Passed
        },
        None,
    ));
    report.checks.push(Check::new(
        "resourceIntegrity",
        CheckStatus::Unverified,
        None,
    ));
    compare_reference(&mut report)?;
    report.recognized = game_valid && report.pkz_count > 0 && shaders;
    report.finalize();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn minimal_pe(machine: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&machine.to_le_bytes());
        bytes[134..136].copy_from_slice(&1u16.to_le_bytes());
        bytes[148..150].copy_from_slice(&224u16.to_le_bytes());
        bytes[150..152].copy_from_slice(&0x102u16.to_le_bytes());
        bytes[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes[152 + 92..152 + 96].copy_from_slice(&16u32.to_le_bytes());
        let section = 152 + 224;
        bytes[section..section + 5].copy_from_slice(b".text");
        bytes[section + 16..section + 20].copy_from_slice(&512u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&512u32.to_le_bytes());
        bytes
    }

    #[test]
    fn rejects_text_truncation_and_hostile_offsets() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Game.exe");
        for content in [b"not a game".to_vec(), b"MZ".to_vec(), {
            let mut b = minimal_pe(0x14c);
            b[0x3c..0x40].fill(255);
            b
        }] {
            fs::write(&path, content).unwrap();
            assert!(inspect_pe(&path, "Game.exe", &AtomicBool::new(false)).is_err());
        }
    }

    #[test]
    fn recognizes_synthetic_structure_without_verifying_it() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("Data/Shaders")).unwrap();
        fs::write(dir.path().join("Game.exe"), minimal_pe(0x14c)).unwrap();
        fs::write(dir.path().join("Data/test.pkz"), b"synthetic archive").unwrap();
        fs::write(
            dir.path().join("Data/Shaders/test.dat"),
            b"synthetic shader",
        )
        .unwrap();
        let report = scan(dir.path(), &AtomicBool::new(false), |_| {}).unwrap();
        assert!(report.recognized);
        assert_eq!(report.verification, "UNVERIFIED");
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.code == "wrapper" && c.status == CheckStatus::Passed)
        );
        assert_eq!(
            fs::read(dir.path().join("Game.exe")).unwrap(),
            minimal_pe(0x14c)
        );
    }

    #[cfg(windows)]
    #[test]
    fn scans_the_canonical_root_held_by_an_installation_lease() {
        let game = tempdir().unwrap();
        let locks = tempdir().unwrap();
        let lease =
            arachnid_core::coordination::InstallationLease::acquire(game.path(), locks.path())
                .unwrap();
        let cancel = AtomicBool::new(false);
        let initial = scan(game.path(), &cancel, |_| {}).unwrap();
        let rescanned = scan(lease.root(), &cancel, |_| {}).unwrap();

        assert_eq!(rescanned.root, initial.root);
        assert_eq!(
            validate_root(Path::new(&rescanned.root)).unwrap(),
            lease.root()
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_network_device_and_alternate_stream_paths() {
        for path in [
            r"\\server\share\game",
            r"\\?\UNC\server\share\game",
            r"\\.\C:\game",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\game",
            r"\\?\C:\game:stream",
            r"\\?\C:\game\..\other",
            r"\\?\C:\game/../other",
        ] {
            assert_eq!(
                validate_root(Path::new(path)).unwrap_err().code,
                "invalidPath"
            );
        }
    }

    #[test]
    fn cancellation_and_missing_resources_are_not_success() {
        let dir = tempdir().unwrap();
        assert_eq!(
            scan(dir.path(), &AtomicBool::new(true), |_| {})
                .unwrap_err()
                .code,
            "cancelled"
        );
        fs::write(dir.path().join("Game.exe"), minimal_pe(0x14c)).unwrap();
        assert!(
            !scan(dir.path(), &AtomicBool::new(false), |_| {})
                .unwrap()
                .recognized
        );
        assert!(validate_root(Path::new("../relative")).is_err());
    }

    #[test]
    fn large_binary_hash_includes_bytes_beyond_the_header_buffer() {
        use std::io::{Seek, SeekFrom, Write};
        let directory = tempdir().unwrap();
        let path = directory.path().join("large-installer.exe");
        fs::write(&path, minimal_pe(0x14c)).unwrap();
        let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(MAX_PE_BYTES + 1024).unwrap();
        drop(file);
        let before = inspect_pe(&path, "large-installer.exe", &AtomicBool::new(false)).unwrap();
        assert_eq!(before.architecture, "x86");
        assert!(!before.imports_inspected);
        file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(SeekFrom::End(-1)).unwrap();
        file.write_all(&[1]).unwrap();
        drop(file);
        let after = inspect_pe(&path, "large-installer.exe", &AtomicBool::new(false)).unwrap();
        assert_ne!(before.sha256, after.sha256);
    }
}
