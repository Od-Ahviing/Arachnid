use crate::{AppState, authorize};
use arachnid_core::{
    AppError, AppResult, CheckStatus, ScanReport, coordination::InstallationLease,
};
use arachnid_mods::{
    Catalog, bundled_catalog,
    composition::Composition,
    fix_settings::DisplaySettings,
    manager::{PackManager, PackOperation, PackPreview, PackSpec, PackStatus},
    performance,
    selection::Selection,
};
use arachnid_repair::{
    Phase,
    filesystem::{self, DirectoryGuard},
};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, atomic::AtomicBool},
};
use tauri::{Manager, State, WebviewWindow};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModStatus {
    components: Composition,
    default_components: Composition,
    catalog: Catalog,
    selection: Selection,
    activation_available: bool,
    pack: PackStatus,
    preview: Option<PackPreview>,
}

fn status(state: &AppState, app: &tauri::AppHandle) -> AppResult<ModStatus> {
    let catalog = bundled_catalog()?;
    let selection = Selection::load(&state.data_directory, &catalog)?;
    let report = crate::commands::report(state)?;
    let lease = crate::commands::lease(state, &report)?;
    let manager = manager(state, &lease)?;
    let components = applied_components(&manager)?;
    let mut pack = pack_status(&manager, components)?;
    pack.requires_update = manager
        .applied_version()?
        .is_some_and(|version| matches!(version.as_str(), "local-preview-1" | "local-preview-2"));
    Ok(ModStatus {
        components,
        default_components: Composition::DEFAULT,
        catalog,
        selection,
        activation_available: (cfg!(debug_assertions) || performance::RUNTIME_QUALIFIED)
            && performance::available(&asset_root(app)?),
        pack,
        preview: manager.preview()?,
    })
}

#[tauri::command]
pub async fn mod_status(window: WebviewWindow, app: tauri::AppHandle) -> AppResult<ModStatus> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        status(&state, &app)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub fn select_mod(
    window: WebviewWindow,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
) -> AppResult<ModStatus> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    crate::commands::ensure_no_session(&state)?;
    let mut current = status(&state, &app)?;
    current.selection.set(&current.catalog, &id, enabled)?;
    current
        .selection
        .save(&state.data_directory, &current.catalog)?;
    state.storage.record("mod_preference_saved");
    Ok(current)
}

fn asset_root(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    if cfg!(debug_assertions) {
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(performance::RESOURCE_DIRECTORY))
    } else {
        app.path()
            .resource_dir()
            .map(|p| p.join("performance-pack"))
            .map_err(|_| AppError::new("modPayloadMissing"))
    }
}

pub(crate) fn child_directory(parent: &Path, name: &str) -> AppResult<PathBuf> {
    let _guard = DirectoryGuard::acquire(parent)?;
    let path = filesystem::resolve(parent, name)?;
    match std::fs::create_dir(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return Err(AppError::new("journalUnavailable")),
    }
    DirectoryGuard::acquire(&path)?;
    Ok(path)
}

pub(crate) fn pack_directory(state: &AppState, lease: &InstallationLease) -> AppResult<PathBuf> {
    child_directory(
        &child_directory(&state.data_directory, "mods")?,
        &lease.id(),
    )
}

fn manager(state: &AppState, lease: &InstallationLease) -> AppResult<PackManager> {
    manager_for(state, lease, Composition::from_bits(7)?)
}

fn manager_for(
    state: &AppState,
    lease: &InstallationLease,
    components: Composition,
) -> AppResult<PackManager> {
    PackManager::new(
        lease.root(),
        &pack_directory(state, lease)?,
        &lease.id(),
        PackSpec::for_composition(components),
    )
}

fn applied_components(manager: &PackManager) -> AppResult<Composition> {
    match manager.applied_version()? {
        Some(version) => Composition::from_version(&version),
        None => Composition::from_bits(0),
    }
}

fn pack_status(manager: &PackManager, components: Composition) -> AppResult<PackStatus> {
    if components.fix {
        return manager.status(&display()?);
    }
    // The manager only compares display values when the installed composition uses Fix.
    manager.status(&DisplaySettings {
        width: 0,
        height: 0,
        windowed: false,
    })
}

fn prepare_shader(config: &Path, config_path: &Path) -> AppResult<()> {
    let directory = child_directory(config, "shaders")?;
    let shader = include_bytes!("../resources/shaders/ArachnidColor.fx");
    let path = filesystem::resolve(&directory, "ArachnidColor.fx")?;
    let existing = filesystem::read(&directory, "ArachnidColor.fx")?;
    let replace = existing
        .as_ref()
        .is_some_and(|image| is_legacy_shader(&image.bytes));
    if existing.is_none() || replace {
        let mut file = tempfile::NamedTempFile::new_in(&directory)
            .map_err(|_| AppError::new("journalUnavailable"))?;
        file.write_all(shader)
            .map_err(|_| AppError::new("journalUnavailable"))?;
        file.as_file()
            .sync_all()
            .map_err(|_| AppError::new("journalUnavailable"))?;
        if replace {
            let previous = existing
                .as_ref()
                .ok_or_else(|| AppError::new("stalePlan"))?;
            let backup = filesystem::resolve(&directory, "ArachnidColor.v1.fx.bak")?;
            if let Some(saved) = filesystem::read(&directory, "ArachnidColor.v1.fx.bak")? {
                if saved.bytes != previous.bytes {
                    return Err(AppError::new("modUnmanagedFiles"));
                }
            } else {
                let mut saved = tempfile::NamedTempFile::new_in(&directory)
                    .map_err(|_| AppError::new("journalUnavailable"))?;
                saved
                    .write_all(&previous.bytes)
                    .map_err(|_| AppError::new("journalUnavailable"))?;
                saved
                    .as_file()
                    .sync_all()
                    .map_err(|_| AppError::new("journalUnavailable"))?;
                saved
                    .persist_noclobber(backup)
                    .map_err(|_| AppError::new("journalUnavailable"))?;
            }
            if filesystem::read(&directory, "ArachnidColor.fx")?.as_ref() != Some(previous) {
                return Err(AppError::new("stalePlan"));
            }
            file.persist(path)
                .map_err(|_| AppError::new("journalUnavailable"))?;
        } else {
            file.persist_noclobber(path)
                .map_err(|_| AppError::new("journalUnavailable"))?;
        }
    }
    let current =
        filesystem::read(config, "ReShade.ini")?.ok_or_else(|| AppError::new("settingAbsent"))?;
    let text = std::str::from_utf8(&current.bytes).map_err(|_| AppError::new("settingOpaque"))?;
    let updated = shader_search_path(text);
    if text != updated {
        let mut file = tempfile::NamedTempFile::new_in(config)
            .map_err(|_| AppError::new("journalUnavailable"))?;
        file.write_all(updated.as_bytes())
            .map_err(|_| AppError::new("journalUnavailable"))?;
        file.as_file()
            .sync_all()
            .map_err(|_| AppError::new("journalUnavailable"))?;
        file.persist(config_path)
            .map_err(|_| AppError::new("journalUnavailable"))?;
    }
    Ok(())
}

fn is_legacy_shader(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    arachnid_mods::fix_adapter::digest(text.replace("\r\n", "\n").as_bytes())
        == "0f5494ef337abb98d2cdbdc2a154a8a0d9f24759ea01e1eaf354eb6c246ce556"
}

fn shader_search_path(text: &str) -> String {
    general_search_path(text, "EffectSearchPaths", r".\shaders")
}

fn general_search_path(text: &str, setting: &str, search_path: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut general = false;
    let mut found = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            general = trimmed.eq_ignore_ascii_case("[GENERAL]");
        }
        if general
            && let Some((key, value)) = line.split_once('=')
            && key.trim().eq_ignore_ascii_case(setting)
        {
            found = true;
            let mut repaired = String::new();
            for (index, part) in value.split(';').enumerate() {
                if index != 0 {
                    repaired.push(
                        if [
                            r".\shaders",
                            r".\community-effects\shaders",
                            r".\community-effects\textures",
                        ]
                        .contains(&part.trim())
                        {
                            ','
                        } else {
                            ';'
                        },
                    );
                }
                repaired.push_str(part);
            }
            let mut paths = reshade_paths(&repaired);
            if !paths.iter().any(|path| path == search_path) {
                paths.push(search_path.into());
            }
            let encoded = paths
                .iter()
                .map(|path| path.replace(',', ",,"))
                .collect::<Vec<_>>()
                .join(",");
            lines.push(format!("{key}={encoded}"));
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim().eq_ignore_ascii_case("[GENERAL]"))
        {
            lines.insert(index + 1, format!("{setting}={search_path}"));
        } else {
            lines.extend(["[GENERAL]".into(), format!("{setting}={search_path}")]);
        }
    }
    lines.join(newline) + newline
}

fn reshade_paths(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut path = String::new();
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == ',' && characters.peek() == Some(&',') {
            characters.next();
            path.push(',');
        } else if character == ',' {
            if !path.trim().is_empty() {
                result.push(path.trim().into());
            }
            path.clear();
        } else {
            path.push(character);
        }
    }
    if !path.trim().is_empty() {
        result.push(path.trim().into());
    }
    result
}

fn display() -> AppResult<DisplaySettings> {
    let value = |name: &str| -> AppResult<Option<u32>> {
        let Some(image) = arachnid_repair::registry::read("Display", name)? else {
            return Ok(None);
        };
        if image.value_type != 4 || image.bytes.len() != 4 {
            return Err(AppError::new("settingOpaque"));
        }
        Ok(Some(u32::from_le_bytes(
            image
                .bytes
                .try_into()
                .map_err(|_| AppError::new("settingOpaque"))?,
        )))
    };
    resolve_display(value("Width")?, value("Height")?, value("Fullscreen")?)
}

fn resolve_display(
    width: Option<u32>,
    height: Option<u32>,
    fullscreen: Option<u32>,
) -> AppResult<DisplaySettings> {
    let (width, height, fullscreen) = match (width, height, fullscreen) {
        (None, None, None) => (
            arachnid_settings::INITIAL_WIDTH,
            arachnid_settings::INITIAL_HEIGHT,
            arachnid_settings::INITIAL_FULLSCREEN,
        ),
        (Some(width), Some(height), Some(fullscreen)) => (width, height, fullscreen),
        _ => return Err(AppError::new("settingAbsent")),
    };
    if fullscreen > 1 {
        return Err(AppError::new("settingOpaque"));
    }
    Ok(DisplaySettings {
        width,
        height,
        windowed: fullscreen == 0,
    })
}

pub(crate) fn check_game(lease: &InstallationLease) -> AppResult<()> {
    arachnid_launch::ensure_game_stopped(lease.root())?;
    let pe = arachnid_detector::inspect_pe(
        &lease.root().join("Game.exe"),
        "Game.exe",
        &AtomicBool::new(false),
    )?;
    if !arachnid_core::game_build::supported(&pe.sha256) {
        return Err(AppError::new("modBuildUnsupported"));
    }
    Ok(())
}

fn refresh_report(
    cache: &Mutex<Option<ScanReport>>,
    root: &Path,
    qualify: impl FnOnce(&mut ScanReport) -> AppResult<()>,
) -> AppResult<ScanReport> {
    *cache.lock().map_err(|_| AppError::new("internal"))? = None;
    let mut report = arachnid_detector::scan(root, &AtomicBool::new(false), |_| {})?;
    arachnid_diagnostics::evaluate(&mut report);
    qualify(&mut report)?;
    *cache.lock().map_err(|_| AppError::new("internal"))? = Some(report.clone());
    Ok(report)
}

fn verify_pack_change(state: &AppState, lease: &InstallationLease) -> AppResult<ScanReport> {
    refresh_report(&state.report, lease.root(), |report| {
        qualify_report(state, lease, report).map(|_| ())
    })
}

pub(crate) fn pending(state: &AppState, lease: &InstallationLease) -> bool {
    manager(state, lease)
        .and_then(|m| m.operation())
        .map(|j| {
            j.is_some_and(|j| {
                !matches!(
                    j.phase,
                    Phase::Prepared | Phase::Committed | Phase::Cancelled | Phase::RolledBack
                )
            })
        })
        .unwrap_or(true)
}

pub(crate) fn qualify_report(
    state: &AppState,
    lease: &InstallationLease,
    report: &mut ScanReport,
) -> AppResult<Option<Composition>> {
    let manager = manager(state, lease)?;
    let components = applied_components(&manager)?;
    if components.bits() != 0
        && components.binaries().iter().all(|file| {
            report.binaries.iter().any(|pe| {
                pe.name.eq_ignore_ascii_case(&file.file_name)
                    && pe.sha256 == file.sha256
                    && pe.architecture == "x86"
            })
        })
    {
        for check in &mut report.checks {
            if check.code == "direct3d" || check.code == "wrapper" {
                check.status = CheckStatus::Passed;
                check.value = Some("managedPerformanceCandidate".into());
            }
        }
        return Ok(Some(components));
    }
    Ok(None)
}

pub(crate) fn runtime_for_launch(
    state: &AppState,
    app: &tauri::AppHandle,
    lease: &InstallationLease,
    report: &mut ScanReport,
) -> AppResult<Option<arachnid_launch::runtime::VulkanRuntime>> {
    let current = manager(state, lease)?;
    if current
        .applied_version()?
        .is_some_and(|version| matches!(version.as_str(), "local-preview-1" | "local-preview-2"))
    {
        return Err(AppError::new("modPackUpdateRequired"));
    }
    let components = applied_components(&current)?;
    let pack = manager_for(
        state,
        lease,
        if components.bits() == 0 {
            Composition::from_bits(7)?
        } else {
            components
        },
    )?;
    let status = pack_status(&pack, components)?;
    if status.recovery_required {
        return Err(AppError::new("recoveryRequired"));
    }
    if !status.active {
        return Ok(None);
    }
    performance::check_conflicts(lease.root())?;
    if status.files_changed || status.files_restored {
        return Err(AppError::new("modOwnedFileChanged"));
    }
    if status.settings_out_of_sync {
        return Err(AppError::new("modSettingsOutOfSync"));
    }
    if !cfg!(debug_assertions) && !performance::RUNTIME_QUALIFIED {
        return Err(AppError::new("performancePackUnqualified"));
    }
    let root = asset_root(app)?;
    if !performance::available(&root) {
        return Err(AppError::new("modPayloadMissing"));
    }
    let vulkan = root.join("vulkan");
    let mut files = if components.dxvk && components.reshade {
        performance::verified_files(&vulkan, &performance::vulkan_files())?
    } else {
        Vec::new()
    };
    files.extend(performance::verified_files(
        lease.root(),
        &components.binaries(),
    )?);
    let config = child_directory(&pack_directory(state, lease)?, "reshade")?;
    let _guard = DirectoryGuard::acquire(&config)?;
    let config_path = filesystem::resolve(&config, "ReShade.ini")?;
    if components.reshade && !config_path.exists() {
        let mut temporary = tempfile::NamedTempFile::new_in(&config)
            .map_err(|_| AppError::new("journalUnavailable"))?;
        temporary
            .write_all(
                b"[INSTALL]\nLogging=1\n[GENERAL]\nEffectSearchPaths=.\\shaders\nTextureSearchPaths=\nTechniques=\n",
            )
            .map_err(|_| AppError::new("journalUnavailable"))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| AppError::new("journalUnavailable"))?;
        temporary
            .persist_noclobber(&config_path)
            .map_err(|_| AppError::new("journalUnavailable"))?;
    }
    if components.reshade {
        prepare_shader(&config, &config_path)?;
        let source = if cfg!(debug_assertions) {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/reshade-effects")
        } else {
            app.path()
                .resource_dir()
                .map_err(|_| AppError::new("modPayloadMissing"))?
                .join("reshade-effects")
        };
        let destination = child_directory(&config, "community-effects")?;
        arachnid_mods::effects::deploy(&source, &destination)?;
        let image = filesystem::read(&config, "ReShade.ini")?
            .ok_or_else(|| AppError::new("settingAbsent"))?;
        let text = std::str::from_utf8(&image.bytes).map_err(|_| AppError::new("settingOpaque"))?;
        let updated =
            general_search_path(text, "EffectSearchPaths", r".\community-effects\shaders");
        let updated = general_search_path(
            &updated,
            "TextureSearchPaths",
            r".\community-effects\textures",
        );
        if updated != text {
            let mut temporary = tempfile::NamedTempFile::new_in(&config)
                .map_err(|_| AppError::new("journalUnavailable"))?;
            temporary
                .write_all(updated.as_bytes())
                .map_err(|_| AppError::new("journalUnavailable"))?;
            temporary
                .as_file()
                .sync_all()
                .map_err(|_| AppError::new("journalUnavailable"))?;
            temporary
                .persist(&config_path)
                .map_err(|_| AppError::new("journalUnavailable"))?;
        }
    }
    qualify_report(state, lease, report)?;
    Ok(Some(
        arachnid_launch::runtime::VulkanRuntime::for_components(
            &vulkan,
            &config,
            files,
            components.dxvk,
            components.reshade,
        )?,
    ))
}

#[tauri::command]
pub async fn reconcile_graphics_mods(
    window: WebviewWindow,
    app: tauri::AppHandle,
    apply_defaults: bool,
) -> AppResult<Option<ScanReport>> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _update = state
            .updates
            .installing
            .try_lock()
            .map_err(|_| AppError::new("appUpdateBusy"))?;
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        let report = crate::commands::report(&state)?;
        let lease = crate::commands::lease(&state, &report)?;
        reconcile_graphics(&state, &app, &lease, &report, apply_defaults)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

pub(crate) fn reconcile_graphics(
    state: &AppState,
    app: &tauri::AppHandle,
    lease: &InstallationLease,
    report: &ScanReport,
    apply_defaults: bool,
) -> AppResult<Option<ScanReport>> {
    let build = crate::commands::build(report)?;
    if !report.recognized
        || !arachnid_core::game_build::supported(build)
        || crate::saves::pending(state)
        || crate::skins::pending(state, lease)
        || arachnid_repair::pending(&crate::commands::journal_path(state, lease)?)
        || arachnid_repair::pending(&crate::commands::initial_settings_path(state, lease)?)
    {
        return Ok(None);
    }
    let current = manager(state, lease)?;
    let operation = current.operation()?;
    if operation.as_ref().is_some_and(|journal| {
        !matches!(
            journal.phase,
            Phase::Committed | Phase::Cancelled | Phase::RolledBack
        )
    }) {
        return Ok(None);
    }
    let saved = applied_components(&current)?;
    let restoring = saved.bits() != 0;
    if report.checks.iter().any(|check| {
        check.status == CheckStatus::Error && !(restoring && is_graphics_file_check(check))
    }) {
        return Ok(None);
    }
    let requested = if restoring {
        let status = pack_status(&current, saved)?;
        if !needs_reconciliation(&status) {
            return Ok(None);
        }
        saved
    } else {
        if !apply_defaults || operation.is_some() {
            return Ok(None);
        }
        let catalog = bundled_catalog()?;
        let selection = Selection::load(&state.data_directory, &catalog)?;
        if !selection.enabled.iter().any(|id| id == "performance-pack") {
            return Ok(None);
        }
        Composition::DEFAULT
    };
    crate::commands::ensure_no_session(state)?;
    check_game(lease)?;
    let display = display()?;
    let root = asset_root(app)?;
    if (!cfg!(debug_assertions) && !performance::RUNTIME_QUALIFIED)
        || !performance::available(&root)
    {
        return Err(AppError::new("modPayloadMissing"));
    }
    let source = root.join("profiles").join(requested.bits().to_string());
    let mut manager = manager_for(state, lease, requested)?;
    let plan = if restoring {
        manager.prepare_selection(PackOperation::Enable, &source, &display, build)?
    } else {
        manager.prepare(PackOperation::Enable, &source, &display, build)?
    };
    manager.apply(&plan.digest, &display, || check_game(lease))?;
    state.storage.record(if restoring {
        "saved_performance_pack_reconciled"
    } else {
        "default_performance_pack_applied"
    });
    let report = verify_pack_change(state, lease)?;
    if !report.recognized
        || report
            .checks
            .iter()
            .any(|check| check.status == CheckStatus::Error)
        || needs_reconciliation(&pack_status(&manager, requested)?)
    {
        return Err(AppError::new("validationFailed"));
    }
    Ok(Some(report))
}

fn is_graphics_file_check(check: &arachnid_core::Check) -> bool {
    check.code == "direct3d"
        || (check.code == "localBinary"
            && check.value.as_deref().is_some_and(|name| {
                ["d3d9.dll", "ar_dxvk.dll"]
                    .iter()
                    .any(|target| name.eq_ignore_ascii_case(target))
            }))
}

fn needs_reconciliation(status: &PackStatus) -> bool {
    status.active && (status.files_changed || status.files_restored || status.settings_out_of_sync)
}

#[tauri::command]
pub async fn prepare_pack(
    window: WebviewWindow,
    app: tauri::AppHandle,
    operation: PackOperation,
    components: Option<Composition>,
) -> AppResult<PackPreview> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        crate::commands::ensure_no_session(&state)?;
        let report = crate::commands::report(&state)?;
        let lease = crate::commands::lease(&state, &report)?;
        if operation == PackOperation::Disable {
            arachnid_launch::ensure_game_stopped(lease.root())?;
        } else {
            check_game(&lease)?;
        }
        if operation != PackOperation::Disable
            && ((!cfg!(debug_assertions) && !performance::RUNTIME_QUALIFIED)
                || !performance::available(&asset_root(&app)?))
        {
            return Err(AppError::new("modPayloadMissing"));
        }
        if components.is_some_and(|value| match operation {
            PackOperation::Enable => value.bits() == 0,
            PackOperation::Disable => value.bits() != 0,
            PackOperation::Sync => false,
        }) {
            return Err(AppError::new("invalidModSelection"));
        }
        let requested = components.unwrap_or(applied_components(&manager(&state, &lease)?)?);
        let requested = if requested.bits() == 0 {
            Composition::DEFAULT
        } else {
            requested
        };
        let mut manager = manager_for(&state, &lease, requested)?;
        let display = if operation == PackOperation::Disable {
            manager
                .preview()?
                .ok_or_else(|| AppError::new("modNotActive"))?
                .display
        } else {
            display()?
        };
        manager.prepare_selection(
            operation,
            &asset_root(&app)?
                .join("profiles")
                .join(requested.bits().to_string()),
            &display,
            crate::commands::build(&report)?,
        )?;
        manager
            .preview()?
            .ok_or_else(|| AppError::new("invalidModState"))
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub async fn apply_pack(
    window: WebviewWindow,
    app: tauri::AppHandle,
    digest: String,
    recover: bool,
) -> AppResult<Option<ScanReport>> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        crate::commands::ensure_no_session(&state)?;
        let report = crate::commands::report(&state)?;
        let lease = crate::commands::lease(&state, &report)?;
        let mut manager = manager(&state, &lease)?;
        let preview = manager
            .preview()?
            .ok_or_else(|| AppError::new("invalidModState"))?;
        let stopped_only = recover || preview.operation == PackOperation::Disable;
        if !recover && crate::skins::pending(&state, &lease) {
            return Err(AppError::new("recoveryRequired"));
        }
        if !stopped_only
            && ((!cfg!(debug_assertions) && !performance::RUNTIME_QUALIFIED)
                || !performance::available(&asset_root(&app)?))
        {
            return Err(AppError::new("modPayloadMissing"));
        }
        let gate = || {
            if stopped_only {
                arachnid_launch::ensure_game_stopped(lease.root())
            } else {
                check_game(&lease)
            }
        };
        gate()?;
        if recover {
            manager.recover(&digest, gate)?;
        } else {
            let display = if stopped_only {
                preview.display
            } else {
                display()?
            };
            manager.apply(&digest, &display, gate)?;
        }
        state.storage.record(if recover {
            "performance_pack_recovered"
        } else {
            "performance_pack_applied"
        });
        let report = verify_pack_change(&state, &lease);
        if report.is_err() {
            state.storage.record("mod_verification_failed");
        }
        Ok(report.ok())
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_errors_on_graphics_pack_binaries_can_be_rechecked_after_reconciliation() {
        let binary = |name: &str| {
            arachnid_core::Check::new("localBinary", CheckStatus::Error, Some(name.into()))
        };
        assert!(is_graphics_file_check(&binary("D3D9.dll")));
        assert!(is_graphics_file_check(&binary("ar_dxvk.dll")));
        assert!(!is_graphics_file_check(&binary("steam_api.dll")));
        assert!(!is_graphics_file_check(&binary("Data/d3d9.dll")));
    }

    #[test]
    fn automatic_reconciliation_reuses_the_active_selection_only_when_it_has_drifted() {
        let mut status = PackStatus {
            active: true,
            recovery_required: false,
            settings_out_of_sync: false,
            files_changed: false,
            files_restored: false,
            phase: Some(Phase::Committed),
            requires_update: false,
        };
        assert!(!needs_reconciliation(&status));
        status.files_changed = true;
        assert!(needs_reconciliation(&status));
        status.files_changed = false;
        status.files_restored = true;
        assert!(needs_reconciliation(&status));
        status.files_restored = false;
        status.settings_out_of_sync = true;
        assert!(needs_reconciliation(&status));
        status.active = false;
        assert!(!needs_reconciliation(&status));
    }

    #[test]
    fn fix_has_a_fullscreen_default_for_a_fresh_registry() {
        assert_eq!(
            resolve_display(None, None, None).unwrap(),
            DisplaySettings {
                width: 1920,
                height: 1080,
                windowed: false
            }
        );
        assert!(resolve_display(Some(1280), None, None).is_err());
        assert!(
            !resolve_display(Some(1920), Some(1080), Some(1))
                .unwrap()
                .windowed
        );
    }

    #[test]
    fn reshade_search_paths_repair_our_old_separators_and_escape_literal_commas() {
        let old = "[GENERAL]\r\nEffectSearchPaths=.\\shaders;.\\community-effects\\shaders\r\nTextureSearchPaths=.\\community-effects\\textures\r\nTechniques=PersonalEffect\r\n";
        let fixed = general_search_path(old, "EffectSearchPaths", r".\community-effects\shaders");
        assert!(fixed.contains("EffectSearchPaths=.\\shaders,.\\community-effects\\shaders\r\n"));
        assert!(fixed.contains("Techniques=PersonalEffect\r\n"));
        assert_eq!(
            general_search_path(&fixed, "EffectSearchPaths", r".\community-effects\shaders"),
            fixed
        );
        assert_eq!(
            reshade_paths(r"C:\Effects,, HDR,C:\Other;Personal,.\shaders"),
            vec![r"C:\Effects, HDR", r"C:\Other;Personal", r".\shaders"]
        );
        let personal = "[GENERAL]\nEffectSearchPaths=C:\\Effects,, HDR,C:\\Other;Personal\n";
        let updated = general_search_path(personal, "EffectSearchPaths", r".\shaders");
        assert!(
            updated.contains(r"EffectSearchPaths=C:\Effects,, HDR,C:\Other;Personal,.\shaders")
        );
    }

    #[test]
    fn known_shader_is_updated_with_backup_and_personal_edits_are_preserved() {
        let config = tempfile::tempdir().unwrap();
        let config_path = config.path().join("ReShade.ini");
        std::fs::write(&config_path, "[GENERAL]\nEffectSearchPaths=\nTechniques=\n").unwrap();
        let directory = child_directory(config.path(), "shaders").unwrap();
        let legacy = include_bytes!("../../tests/fixtures/shaders/ArachnidColor.v1.fx");
        let current = include_bytes!("../resources/shaders/ArachnidColor.fx");
        assert!(is_legacy_shader(legacy));
        let crlf = std::str::from_utf8(legacy)
            .unwrap()
            .replace("\r\n", "\n")
            .replace('\n', "\r\n");
        assert!(is_legacy_shader(crlf.as_bytes()));
        assert!(!is_legacy_shader(current));
        std::fs::write(directory.join("ArachnidColor.fx"), legacy).unwrap();
        prepare_shader(config.path(), &config_path).unwrap();
        assert_eq!(
            std::fs::read(directory.join("ArachnidColor.fx")).unwrap(),
            current
        );
        assert_eq!(
            std::fs::read(directory.join("ArachnidColor.v1.fx.bak")).unwrap(),
            legacy
        );
        prepare_shader(config.path(), &config_path).unwrap();
        std::fs::write(directory.join("ArachnidColor.fx"), b"personal shader").unwrap();
        prepare_shader(config.path(), &config_path).unwrap();
        assert_eq!(
            std::fs::read(directory.join("ArachnidColor.fx")).unwrap(),
            b"personal shader"
        );
    }

    #[test]
    fn conflicting_shader_backup_preserves_the_installed_shader() {
        let config = tempfile::tempdir().unwrap();
        let config_path = config.path().join("ReShade.ini");
        std::fs::write(&config_path, "[GENERAL]\n").unwrap();
        let directory = child_directory(config.path(), "shaders").unwrap();
        let legacy = include_bytes!("../../tests/fixtures/shaders/ArachnidColor.v1.fx");
        std::fs::write(directory.join("ArachnidColor.fx"), legacy).unwrap();
        std::fs::write(
            directory.join("ArachnidColor.v1.fx.bak"),
            b"personal backup",
        )
        .unwrap();
        assert!(prepare_shader(config.path(), &config_path).is_err());
        assert_eq!(
            std::fs::read(directory.join("ArachnidColor.fx")).unwrap(),
            legacy
        );
    }

    #[test]
    fn shader_search_path_preserves_user_effects_settings_and_line_endings() {
        let original = "[GENERAL]\r\nEffectSearchPaths=C:\\PersonalShaders\r\nTechniques=UserEffect\r\n[OTHER]\r\nValue=7\r\n";
        let updated = shader_search_path(original);
        assert!(updated.contains("EffectSearchPaths=C:\\PersonalShaders,.\\shaders\r\n"));
        assert!(updated.contains("Techniques=UserEffect\r\n"));
        assert!(updated.contains("[OTHER]\r\nValue=7\r\n"));
        assert_eq!(shader_search_path(&updated), updated);
        let inserted = shader_search_path("[GENERAL]\nTechniques=\n[OTHER]\nValue=7\n");
        assert_eq!(inserted.matches("[GENERAL]").count(), 1);
        assert!(inserted.contains("EffectSearchPaths=.\\shaders"));
        let effects = general_search_path(
            &updated,
            "EffectSearchPaths",
            r".\community-effects\shaders",
        );
        let effects = general_search_path(
            &effects,
            "TextureSearchPaths",
            r".\community-effects\textures",
        );
        assert!(effects.contains("Techniques=UserEffect\r\n"));
        assert!(effects.contains(r"C:\PersonalShaders,.\shaders,.\community-effects\shaders"));
        assert_eq!(
            general_search_path(
                &effects,
                "TextureSearchPaths",
                r".\community-effects\textures"
            ),
            effects
        );
    }

    fn synthetic_pe(marker: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x14cu16.to_le_bytes());
        bytes[134..136].copy_from_slice(&1u16.to_le_bytes());
        bytes[148..150].copy_from_slice(&224u16.to_le_bytes());
        bytes[150..152].copy_from_slice(&0x102u16.to_le_bytes());
        bytes[152..154].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes[244..248].copy_from_slice(&16u32.to_le_bytes());
        let section = 376;
        bytes[section..section + 5].copy_from_slice(b".text");
        bytes[section + 16..section + 20].copy_from_slice(&512u32.to_le_bytes());
        bytes[section + 20..section + 24].copy_from_slice(&512u32.to_le_bytes());
        bytes[1023] = marker;
        bytes
    }

    #[test]
    fn pack_changes_refresh_cached_fingerprints_and_removed_binaries() {
        let game = tempfile::tempdir().unwrap();
        let cache = Mutex::new(None);
        let dll = game.path().join("ar_dxvk.dll");
        std::fs::write(&dll, synthetic_pe(1)).unwrap();
        refresh_report(&cache, game.path(), |_| Ok(())).unwrap();
        let previous = cache.lock().unwrap().as_ref().unwrap().binaries[0]
            .sha256
            .clone();
        std::fs::write(&dll, synthetic_pe(2)).unwrap();
        refresh_report(&cache, game.path(), |_| Ok(())).unwrap();
        let current = cache.lock().unwrap().as_ref().unwrap().binaries[0]
            .sha256
            .clone();
        assert_ne!(previous, current);
        std::fs::remove_file(&dll).unwrap();
        refresh_report(&cache, game.path(), |_| Ok(())).unwrap();
        assert!(cache.lock().unwrap().as_ref().unwrap().binaries.is_empty());
        assert!(refresh_report(&cache, &game.path().join("missing"), |_| Ok(())).is_err());
        assert!(cache.lock().unwrap().is_none());
    }
}
