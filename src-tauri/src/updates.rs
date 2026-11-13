use crate::{AppState, authorize, commands};
use arachnid_core::{AppError, AppResult};
use serde::Serialize;
use std::{
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State, WebviewWindow, ipc::Channel};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Default)]
pub(crate) struct Updates {
    checking: tauri::async_runtime::Mutex<()>,
    pub(crate) installing: tauri::async_runtime::Mutex<()>,
    pending: Mutex<Option<Update>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DownloadProgress {
    received: u64,
    total: Option<u64>,
}

fn is_release_installer(update: &Update) -> bool {
    release_installer_url(
        update.download_url.scheme(),
        update.download_url.host_str(),
        update.download_url.path(),
        &update.version,
    ) && update.download_url.port().is_none()
        && update.download_url.username().is_empty()
        && update.download_url.password().is_none()
        && update.download_url.query().is_none()
        && update.download_url.fragment().is_none()
}

fn release_installer_url(scheme: &str, host: Option<&str>, path: &str, version: &str) -> bool {
    scheme == "https"
        && host == Some("github.com")
        && path
            == format!(
                "/Od-Ahviing/Arachnid/releases/download/v{version}/Arachnid_{version}_x64-setup.exe"
            )
}

#[tauri::command]
pub(crate) async fn check_app_update(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<String>> {
    authorize(&window)?;
    if !app.config().plugins.0.contains_key("updater") {
        return Ok(None);
    }
    let _busy = state
        .updates
        .checking
        .try_lock()
        .map_err(|_| AppError::new("appUpdateBusy"))?;
    *state
        .updates
        .pending
        .lock()
        .map_err(|_| AppError::new("internal"))? = None;
    let update = app
        .updater_builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| AppError::new("appUpdateCheckFailed"))?
        .check()
        .await
        .map_err(|_| AppError::new("appUpdateCheckFailed"))?;
    if update
        .as_ref()
        .is_some_and(|update| !is_release_installer(update))
    {
        return Err(AppError::new("appUpdateCheckFailed"));
    }
    let version = update.as_ref().map(|update| update.version.clone());
    *state
        .updates
        .pending
        .lock()
        .map_err(|_| AppError::new("internal"))? = update;
    Ok(version)
}

fn ensure_idle(state: &AppState) -> AppResult<()> {
    let has_session = state
        .session
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .is_some();
    if has_session {
        commands::status(state)?;
    }
    commands::ensure_no_session(state)?;
    let path = state
        .preferences
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .installation_path
        .clone();
    if let Some(path) = path {
        arachnid_launch::ensure_game_stopped(Path::new(&path))?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn install_app_update(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
    version: String,
    progress: Channel<DownloadProgress>,
) -> AppResult<()> {
    authorize(&window)?;
    let _busy = state
        .updates
        .installing
        .try_lock()
        .map_err(|_| AppError::new("appUpdateBusy"))?;
    let mut update = state
        .updates
        .pending
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .as_ref()
        .filter(|update| update.version == version)
        .cloned()
        .ok_or_else(|| AppError::new("appUpdateCheckFailed"))?;
    {
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        ensure_idle(&state)?;
    }

    update.timeout = Some(Duration::from_secs(60 * 60));
    let mut received = 0u64;
    let mut last_progress = None;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let now = Instant::now();
                if last_progress
                    .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(200))
                    || total == Some(received)
                {
                    let _ = progress.send(DownloadProgress { received, total });
                    last_progress = Some(now);
                }
            },
            || {},
        )
        .await
        .map_err(|_| AppError::new("appUpdateDownloadFailed"))?;

    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        ensure_idle(&state)?;
        state.storage.record("app_update_installing");
        update
            .install(bytes)
            .map_err(|_| AppError::new("appUpdateInstallFailed"))
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[cfg(test)]
mod tests {
    use super::release_installer_url;

    #[test]
    fn update_must_target_the_tagged_windows_installer_in_the_public_repository() {
        let path = "/Od-Ahviing/Arachnid/releases/download/v0.1.2/Arachnid_0.1.2_x64-setup.exe";
        assert!(release_installer_url(
            "https",
            Some("github.com"),
            path,
            "0.1.2"
        ));
        assert!(!release_installer_url(
            "https",
            Some("github.com"),
            path,
            "0.1.3"
        ));
        assert!(!release_installer_url(
            "http",
            Some("github.com"),
            path,
            "0.1.2"
        ));
        assert!(!release_installer_url(
            "https",
            Some("example.org"),
            path,
            "0.1.2"
        ));
        assert!(!release_installer_url(
            "https",
            Some("github.com"),
            &path.replace("Od-Ahviing", "other"),
            "0.1.2"
        ));
    }
}
