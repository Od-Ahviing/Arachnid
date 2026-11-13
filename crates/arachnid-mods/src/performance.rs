use crate::{
    Artifact, Preimage, RecipeFile,
    archive::{extract_reshade_setup, extract_zip},
    fix_adapter,
    offline::{Payload, hash_stream, open_payload, publish_payload},
};
use arachnid_core::{AppError, AppResult};
use arachnid_repair::filesystem::DirectoryGuard;
use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

pub const GAME_SHA256: &str = "facb480a78b1d5dfd2cdbba31be89be9cf3cd01e1eadd524de35f23ea436d60d";
pub const VERSION: &str = "local-preview-3";
pub const RESOURCE_DIRECTORY: &str = "performance-preview-3";
pub const RUNTIME_QUALIFIED: bool = true;

pub fn binaries() -> Vec<Payload> {
    vec![
        descriptor("d3d9.dll", fix_adapter::ADAPTED_SHA256, 2_079_232),
        descriptor(
            "ar_dxvk.dll",
            "00bc142e98ddad1e334a4d88fa630038e092b0758b3d79e295172b181a4c772d",
            4_460_558,
        ),
    ]
}

pub fn vulkan_files() -> Vec<Payload> {
    vec![
        descriptor(
            "ReShade32.dll",
            "189e265f72ad80d7cc227f5de7c8a29cf32d6a2c90a4a03e5878e5ca0f107ee9",
            3_910_424,
        ),
        descriptor(
            "ReShade32.json",
            "dbfdbadb8c64b398b2f8bf30469841ab8c35e10c9da40ba69b966559d5fa15f6",
            526,
        ),
    ]
}

pub fn legacy_binaries() -> Vec<Payload> {
    let mut files = binaries();
    files[1] = descriptor(
        "ar_dxvk.dll",
        "00ecd422b3b12e9d3b309785f3f19431405389d80eb4a37018ebc63e2d87d900",
        4_493_326,
    );
    files
}

fn descriptor(name: &str, hash: &str, bytes: u64) -> Payload {
    Payload {
        file_name: name.into(),
        sha256: hash.into(),
        bytes,
    }
}

pub fn verified_files(root: &Path, files: &[Payload]) -> AppResult<Vec<File>> {
    let _guard = DirectoryGuard::acquire(root)?;
    files
        .iter()
        .map(|payload| {
            let mut file =
                open_payload(root, payload)?.ok_or_else(|| AppError::new("modPayloadMissing"))?;
            if hash_stream(&mut file, payload.bytes, None)? != payload.sha256 {
                return Err(AppError::new("offlineFileInvalid"));
            }
            Ok(file)
        })
        .collect()
}

pub fn available(root: &Path) -> bool {
    exact_directory(root, &["d3d9.dll", "ar_dxvk.dll", "vulkan", "profiles"]).is_ok()
        && exact_directory(&root.join("vulkan"), &["ReShade32.dll", "ReShade32.json"]).is_ok()
        && verified_files(root, &binaries()).is_ok()
        && verified_files(&root.join("vulkan"), &vulkan_files()).is_ok()
}

pub(crate) fn exact_directory(root: &Path, names: &[&str]) -> AppResult<()> {
    let _guard = DirectoryGuard::acquire(root)?;
    for entry in std::fs::read_dir(root)
        .map_err(|_| AppError::new("modPayloadMissing"))?
        .take(names.len() + 1)
    {
        let entry = entry.map_err(|_| AppError::new("modPayloadMissing"))?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|name| names.contains(&name))
        {
            return Err(AppError::new("offlineUnexpectedFile"));
        }
    }
    Ok(())
}

pub fn verify_release(root: &Path) -> AppResult<()> {
    if !RUNTIME_QUALIFIED {
        return Err(AppError::new("performancePackUnqualified"));
    }
    verify_prepared(root)
}

pub fn verify_prepared(root: &Path) -> AppResult<()> {
    if !available(root) {
        return Err(AppError::new("modPayloadMissing"));
    }
    let profiles = root.join("profiles");
    exact_directory(&profiles, &["1", "2", "3", "4", "5", "6", "7"])?;
    for bits in 1..=7 {
        let composition = crate::composition::Composition::from_bits(bits)?;
        let directory = profiles.join(bits.to_string());
        let files = composition.binaries();
        exact_directory(
            &directory,
            &files
                .iter()
                .map(|file| file.file_name.as_str())
                .collect::<Vec<_>>(),
        )?;
        verified_files(&directory, &files)?;
    }
    Ok(())
}

pub fn check_conflicts(game: &Path) -> AppResult<()> {
    let _guard = DirectoryGuard::acquire(game)?;
    for name in [
        "dxgi.dll",
        "d3d11.dll",
        "dinput8.dll",
        "winmm.dll",
        "dsound.dll",
        "opengl32.dll",
        "ReShade.ini",
        "ReShade32.dll",
        "ReShade64.dll",
    ] {
        match std::fs::symlink_metadata(game.join(name)) {
            Ok(_) if name == "dinput8.dll" && crate::skins::managed_loader(game) => (),
            Ok(_) => return Err(AppError::new("modUnmanagedFiles")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(AppError::new("fileUnreadable")),
        }
    }
    Ok(())
}

/// Private maintainer preparation only. No installation, registration or third-party execution.
pub fn prepare_preview(
    fix: &Path,
    dxvk: &Path,
    reshade: &Path,
    destination: &Path,
) -> AppResult<()> {
    let _guard = DirectoryGuard::acquire(destination)?;
    let staging = tempfile::tempdir().map_err(|_| AppError::new("offlineWriteFailed"))?;
    let fix_artifact = Artifact {
        url: "https://fixenhancers.wixsite.com/fix-enhancers/pc-fixes".into(),
        sha256: "d672dc66bf25e6bee17736f5764e80517d73c665326f18b31d44c3817b2f382f".into(),
        archive_bytes: 3_081_564,
        extracted_bytes: 10_000_000,
    };
    let fix_file = RecipeFile {
        source: "The Amazing Spider-Man Fix/d3d9.dll".into(),
        target: "d3d9.dll".into(),
        sha256: fix_adapter::ORIGINAL_SHA256.into(),
        bytes: fix_adapter::ORIGINAL_BYTES as u64,
        preimage: Preimage::Absent,
    };
    let prepared = extract_zip(fix, &fix_artifact, &[fix_file], staging.path())?;
    let mut original = Vec::new();
    File::open(
        prepared
            .file("d3d9.dll")
            .ok_or_else(|| AppError::new("modPayloadMissing"))?,
    )
    .map_err(|_| AppError::new("offlineFileUnavailable"))?
    .take(fix_adapter::ORIGINAL_BYTES as u64 + 1)
    .read_to_end(&mut original)
    .map_err(|_| AppError::new("offlineFileUnavailable"))?;
    let candidate = fix_adapter::adapt(&original)?;
    publish_payload(destination, &binaries()[0], &mut Cursor::new(candidate))?;
    let _dxvk_guard =
        DirectoryGuard::acquire(dxvk.parent().ok_or_else(|| AppError::new("invalidPath"))?)?;
    let dxvk_descriptor = Payload {
        file_name: dxvk
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::new("invalidPath"))?
            .into(),
        ..binaries()[1].clone()
    };
    let mut dxvk_file = open_payload(
        dxvk.parent().ok_or_else(|| AppError::new("invalidPath"))?,
        &dxvk_descriptor,
    )?
    .ok_or_else(|| AppError::new("modPayloadMissing"))?;
    publish_payload(destination, &binaries()[1], &mut dxvk_file)?;
    let reshade_artifact = Artifact {
        url: "https://reshade.me/downloads/ReShade_Setup_6.6.2.exe".into(),
        sha256: "807e145eab98903b20f1fc743598a74e3a3be7394ab015b7f64231228168f04e".into(),
        archive_bytes: 3_940_880,
        extracted_bytes: 16_000_000,
    };
    let files: Vec<_> = vulkan_files()
        .into_iter()
        .map(|p| RecipeFile {
            source: p.file_name.clone(),
            target: p.file_name,
            sha256: p.sha256,
            bytes: p.bytes,
            preimage: Preimage::Absent,
        })
        .collect();
    let prepared = extract_reshade_setup(reshade, &reshade_artifact, &files, staging.path())?;
    let vulkan = destination.join("vulkan");
    match std::fs::create_dir(&vulkan) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return Err(AppError::new("offlineWriteFailed")),
    }
    let _vulkan_guard = DirectoryGuard::acquire(&vulkan)?;
    for payload in vulkan_files() {
        let source = prepared
            .file(&payload.file_name)
            .ok_or_else(|| AppError::new("modPayloadMissing"))?;
        publish_payload(
            &vulkan,
            &payload,
            &mut File::open(source).map_err(|_| AppError::new("offlineFileUnavailable"))?,
        )?;
    }
    verified_files(destination, &binaries())?;
    verified_files(&vulkan, &vulkan_files())?;
    let profiles = destination.join("profiles");
    std::fs::create_dir_all(&profiles).map_err(|_| AppError::new("offlineWriteFailed"))?;
    for bits in 1..=7 {
        let profile = crate::composition::Composition::from_bits(bits)?;
        let directory = profiles.join(bits.to_string());
        std::fs::create_dir_all(&directory).map_err(|_| AppError::new("offlineWriteFailed"))?;
        for payload in profile.binaries() {
            let bytes = if payload.sha256 == fix_adapter::ORIGINAL_SHA256 {
                original.clone()
            } else if payload.sha256 == fix_adapter::ADAPTED_SHA256 {
                std::fs::read(destination.join("d3d9.dll"))
                    .map_err(|_| AppError::new("offlineFileUnavailable"))?
            } else if payload.sha256 == binaries()[1].sha256 {
                std::fs::read(destination.join("ar_dxvk.dll"))
                    .map_err(|_| AppError::new("offlineFileUnavailable"))?
            } else {
                std::fs::read(vulkan.join("ReShade32.dll"))
                    .map_err(|_| AppError::new("offlineFileUnavailable"))?
            };
            publish_payload(&directory, &payload, &mut Cursor::new(bytes))?;
        }
        verified_files(&directory, &profile.binaries())?;
    }
    Ok(())
}
