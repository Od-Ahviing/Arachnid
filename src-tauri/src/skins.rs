use crate::{AppState, authorize};
use arachnid_core::{AppError, AppResult, coordination::InstallationLease};
use arachnid_mods::skins::{SkinManager, SkinPreview, SkinStatus};
use std::path::{Path, PathBuf};
use tauri::{Manager, WebviewWindow};

fn assets(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    if cfg!(debug_assertions) {
        Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(arachnid_mods::skins::RESOURCE_DIRECTORY))
    } else {
        app.path()
            .resource_dir()
            .map(|path| path.join("skins"))
            .map_err(|_| AppError::new("modPayloadMissing"))
    }
}

fn manager(
    state: &AppState,
    app: &tauri::AppHandle,
    lease: &InstallationLease,
) -> AppResult<SkinManager> {
    let directory =
        crate::mods::child_directory(&crate::mods::pack_directory(state, lease)?, "skins")?;
    SkinManager::new(lease.root(), &directory, &assets(app)?, &lease.id())
}

#[tauri::command]
pub async fn skin_status(window: WebviewWindow, app: tauri::AppHandle) -> AppResult<SkinStatus> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        let report = crate::commands::report(&state)?;
        let lease = crate::commands::lease(&state, &report)?;
        manager(&state, &app, &lease)?.status()
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub async fn prepare_skin(
    window: WebviewWindow,
    app: tauri::AppHandle,
    selected: Option<String>,
) -> AppResult<SkinPreview> {
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
        crate::mods::check_game(&lease)?;
        if crate::mods::pending(&state, &lease) {
            return Err(AppError::new("recoveryRequired"));
        }
        manager(&state, &app, &lease)?.prepare(selected, crate::commands::build(&report)?)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub async fn apply_skin(
    window: WebviewWindow,
    app: tauri::AppHandle,
    digest: String,
    recover: bool,
) -> AppResult<()> {
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
        if !recover && crate::mods::pending(&state, &lease) {
            return Err(AppError::new("recoveryRequired"));
        }
        let gate = || {
            if recover {
                arachnid_launch::ensure_game_stopped(lease.root())
            } else {
                crate::mods::check_game(&lease)
            }
        };
        gate()?;
        manager(&state, &app, &lease)?.apply(&digest, recover, gate)?;
        state.storage.record("skin_transaction_finished");
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

pub(crate) fn runtime(
    state: &AppState,
    app: &tauri::AppHandle,
    lease: &InstallationLease,
    performance: Option<arachnid_launch::runtime::VulkanRuntime>,
) -> AppResult<Option<arachnid_launch::runtime::VulkanRuntime>> {
    let manager = manager(state, app, lease)?;
    let Some(files) = manager.runtime_files()? else {
        return Ok(performance);
    };
    Ok(Some(arachnid_launch::runtime::VulkanRuntime::with_skin(
        performance,
        lease.root(),
        &assets(app)?,
        files,
    )?))
}

pub(crate) fn pending(state: &AppState, lease: &InstallationLease) -> bool {
    crate::mods::pack_directory(state, lease)
        .map(|path| arachnid_repair::pending(&path.join("skins/skin-transaction.json")))
        .unwrap_or(true)
}
