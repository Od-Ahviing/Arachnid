use crate::{AppState, authorize, commands};
use arachnid_core::{AppError, AppResult};
use arachnid_saveguard::{STABILITY_WINDOW, Snapshot, SourceProfile, Store};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

pub struct SaveState {
    discovery: Mutex<Option<arachnid_saveguard::discovery::Discovery>>,
    profile: Mutex<Option<SourceProfile>>,
    error: Mutex<Option<String>>,
    busy: AtomicBool,
    configuration_error: AtomicBool,
}

impl Default for SaveState {
    fn default() -> Self {
        Self {
            discovery: Mutex::new(None),
            profile: Mutex::new(None),
            error: Mutex::new(None),
            busy: AtomicBool::new(false),
            configuration_error: AtomicBool::new(false),
        }
    }
}

fn backup_root(state: &AppState) -> PathBuf {
    state.data_directory.join("saveguard")
}
fn config_path(state: &AppState) -> PathBuf {
    state.data_directory.join("saveguard-profile.json")
}

fn profile(state: &AppState) -> AppResult<Option<SourceProfile>> {
    Ok(state
        .saves
        .profile
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .clone())
}

fn store(state: &AppState) -> AppResult<Store> {
    Store::open(
        &backup_root(state),
        profile(state)?.ok_or_else(|| AppError::new("saveNotConfigured"))?,
    )
}

fn record_error(state: &AppState, result: &AppResult<()>) {
    if let Ok(mut error) = state.saves.error.lock() {
        *error = result.as_ref().err().map(|e| e.code.clone());
    }
}

pub fn pending(state: &AppState) -> bool {
    if profile(state).is_ok_and(|p| p.is_none()) {
        return state.saves.configuration_error.load(Ordering::Relaxed);
    }
    match profile(state) {
        Ok(Some(profile))
            if profile.id.len() == 64 && profile.id.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            arachnid_repair::pending(&backup_root(state).join(profile.id).join("restore.json"))
        }
        _ => true,
    }
}

pub fn session_snapshot(state: &AppState, reason: &str) {
    if !profile(state).ok().flatten().is_some_and(|p| p.enabled) {
        return;
    }
    let result =
        store(state).and_then(|store| store.reconcile(reason, STABILITY_WINDOW).map(|_| ()));
    record_error(state, &result);
}

pub fn initialize(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let result = (|| {
        std::fs::create_dir_all(backup_root(&state)).map_err(|_| AppError::new("writeFailed"))?;
        if config_path(&state).exists() {
            let mut loaded: SourceProfile =
                arachnid_saveguard::load_json(&config_path(&state), 16 * 1024)?;
            if loaded.upgrade_default_quota() {
                arachnid_saveguard::save_json(&config_path(&state), &loaded)?;
            }

            *state
                .saves
                .profile
                .lock()
                .map_err(|_| AppError::new("internal"))? = Some(loaded);
        }
        Ok(())
    })();
    state
        .saves
        .configuration_error
        .store(result.is_err(), Ordering::Relaxed);
    record_error(&state, &result);
    if result.is_ok() {
        let detected = detect_source(app, &state, false);
        record_error(&state, &detected);
    }
    let app = app.clone();
    // The worker belongs to the Rust application, never to a page or a Pinia subscription.
    std::thread::spawn(move || {
        let mut discovery = Instant::now();
        let mut watched = None;
        let mut watcher = None;
        let mut last = Instant::now() - Duration::from_secs(30);
        let mut dirty_since: Option<Instant> = None;
        loop {
            let state = app.state::<AppState>();
            let mut reopened = false;
            if (state.session.lock().is_ok_and(|s| s.is_some()) || state.lifecycle.in_background())
                && let Ok(_operation) = state.operations.try_lock()
                && let Ok(status) = commands::status(&state)
            {
                match crate::lifecycle::update(&app, &state.lifecycle, &status.session) {
                    arachnid_launch::presentation::Action::ReleaseInterface => {
                        state.storage.record("interface_released_for_game")
                    }
                    arachnid_launch::presentation::Action::ReopenInterface => {
                        state.storage.record("interface_reopened_after_game");
                        reopened = true;
                    }
                    _ => (),
                }
            }
            if reopened {
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
            if discovery.elapsed() >= Duration::from_secs(5)
                && (profile(&state).is_ok_and(|p| p.is_none()) || discovery_changed(&state))
            {
                if let Ok(_operation) = state.operations.try_lock() {
                    let result = detect_source(&app, &state, false);
                    if result.is_err() {
                        record_error(&state, &result);
                    }
                }
                discovery = Instant::now();
            }
            let active = profile(&state).ok().flatten().filter(|p| p.enabled);
            let source = active.map(|p| p.source);
            if source != watched {
                watcher = source
                    .as_ref()
                    .and_then(|p| arachnid_saveguard::watcher::Watcher::new(p));
                watched = source.clone();
                dirty_since = Some(Instant::now());
            }
            let changed = if let Some(watcher) = &watcher {
                watcher.changed(Duration::from_secs(1))
            } else {
                std::thread::sleep(Duration::from_secs(1));
                false
            };
            if changed {
                dirty_since = Some(Instant::now());
            }
            if source.is_none() {
                continue;
            }
            // A complete pass every 30s recovers missed notifications, unavailable watchers and restart.
            if last.elapsed() < Duration::from_secs(30)
                && !dirty_since.is_some_and(|t| t.elapsed() >= Duration::from_secs(2))
            {
                continue;
            }
            let Ok(_operation) = state.operations.try_lock() else {
                continue;
            };
            state.saves.busy.store(true, Ordering::Relaxed);
            let result = store(&state)
                .and_then(|store| store.reconcile("watch", STABILITY_WINDOW).map(|_| ()));
            record_error(&state, &result);
            state.saves.busy.store(false, Ordering::Relaxed);
            dirty_since = None;
            last = Instant::now();
        }
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDelta {
    path: String,
    before_bytes: Option<usize>,
    after_bytes: Option<usize>,
    before_hash: Option<String>,
    after_hash: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    digest: String,
    phase: arachnid_repair::Phase,
    files: Vec<FileDelta>,
}

fn preview(journal: arachnid_repair::Journal) -> RestorePreview {
    RestorePreview {
        digest: journal.digest,
        phase: journal.phase,
        files: journal
            .plan
            .actions
            .iter()
            .map(|action| FileDelta {
                path: match &action.target {
                    arachnid_repair::Target::File { path } => path.clone(),
                    _ => String::new(),
                },
                before_bytes: action.before.as_ref().map(|i| i.bytes.len()),
                after_bytes: action.after.as_ref().map(|i| i.bytes.len()),
                before_hash: action
                    .before
                    .as_ref()
                    .map(|i| arachnid_saveguard::source::hash(&i.bytes)),
                after_hash: action
                    .after
                    .as_ref()
                    .map(|i| arachnid_saveguard::source::hash(&i.bytes)),
            })
            .collect(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStatus {
    profile: Option<SourceProfile>,
    snapshots: Vec<Snapshot>,
    operation: Option<RestorePreview>,
    error: Option<String>,
    busy: bool,
    backup_directory: String,
}

#[tauri::command]
pub fn save_status(window: WebviewWindow, state: State<'_, AppState>) -> AppResult<SaveStatus> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    let profile = profile(&state)?;
    let mut snapshots = Vec::new();
    let mut operation = None;
    let mut error = state
        .saves
        .error
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .clone();
    if profile.is_some() {
        match store(&state).and_then(|s| Ok((s.index()?, s.operation()?.map(preview)))) {
            Ok((history, restore)) => {
                snapshots = history;
                operation = restore;
            }
            Err(failure) => error = Some(failure.code),
        }
    }
    Ok(SaveStatus {
        backup_directory: profile
            .as_ref()
            .map(|p| backup_root(&state).join(&p.id))
            .unwrap_or_else(|| backup_root(&state))
            .to_string_lossy()
            .into_owned(),
        profile,
        snapshots,
        operation,
        error,
        busy: state.saves.busy.load(Ordering::Relaxed),
    })
}

fn detect_source(app: &tauri::AppHandle, state: &AppState, reset: bool) -> AppResult<()> {
    if pending(state) {
        return Err(AppError::new("recoveryRequired"));
    }
    if !reset && profile(state)?.is_some_and(|p| p.manual_source) {
        return Ok(());
    }
    let documents = app
        .path()
        .document_dir()
        .map_err(|_| AppError::new("saveSourceUnavailable"))?;
    let roaming = app
        .path()
        .data_dir()
        .map_err(|_| AppError::new("saveSourceUnavailable"))?;
    let installation = state
        .preferences
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .installation_path
        .as_ref()
        .map(PathBuf::from);
    let mut discovery = state
        .saves
        .discovery
        .lock()
        .map_err(|_| AppError::new("internal"))?;
    if reset
        || discovery
            .as_ref()
            .is_none_or(|d| d.installation != installation)
    {
        *discovery = Some(arachnid_saveguard::discovery::Discovery::new(
            &documents,
            &roaming,
            installation.as_deref(),
        )?);
    }
    let detection = discovery
        .as_ref()
        .ok_or_else(|| AppError::new("internal"))?;
    let found = detection.find(&backup_root(state))?;
    if let Some(mut next) = found {
        if let Some(previous) = profile(state)? {
            if previous.id == next.id && previous.automatic && !reset {
                return Ok(());
            }
            let archive = backup_root(state).join(format!("previous-profile-{}.json", previous.id));
            arachnid_saveguard::save_json(&archive, &previous)?;
            next.keep = previous.keep;
            next.quota_bytes = previous.quota_bytes;
            if previous.automatic || previous.manual_source {
                next.enabled = previous.enabled;
            }
        }
        arachnid_saveguard::save_json(&config_path(state), &next)?;
        *state
            .saves
            .profile
            .lock()
            .map_err(|_| AppError::new("internal"))? = Some(next);
        record_error(state, &Ok(()));
    } else if reset {
        return Err(AppError::new("saveSourceUnavailable"));
    } else if profile(state)?.is_some_and(|p| !p.automatic || !detection.matches_source(&p.source))
    {
        *state
            .saves
            .profile
            .lock()
            .map_err(|_| AppError::new("internal"))? = None;
    }
    Ok(())
}

fn discovery_changed(state: &AppState) -> bool {
    let installation = match state.preferences.lock() {
        Ok(p) => p.installation_path.as_ref().map(PathBuf::from),
        Err(_) => return false,
    };
    state
        .saves
        .discovery
        .lock()
        .is_ok_and(|d| d.as_ref().is_none_or(|d| d.installation != installation))
}

#[tauri::command]
pub fn detect_save_source(
    window: WebviewWindow,
    app: tauri::AppHandle,
    reset: Option<bool>,
) -> AppResult<()> {
    authorize(&window)?;
    let state = app.state::<AppState>();
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    detect_source(&app, &state, reset.unwrap_or(false))
}

#[tauri::command]
pub async fn choose_save_source(window: WebviewWindow, app: tauri::AppHandle) -> AppResult<bool> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut dialog = app.dialog().file();
        if let Some(profile) = profile(&state)? {
            dialog = dialog.set_directory(profile.source);
        }
        let Some(chosen) = dialog.blocking_pick_folder() else {
            return Ok(false);
        };
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        restore_gate(&state)?;
        if pending(&state) {
            return Err(AppError::new("recoveryRequired"));
        }
        let source = PathBuf::from(chosen.to_string());
        if let Some(report) = state
            .report
            .lock()
            .map_err(|_| AppError::new("internal"))?
            .as_ref()
        {
            arachnid_saveguard::source::separate(&source, std::path::Path::new(&report.root))?;
        }
        let mut next = SourceProfile::new(&source, &backup_root(&state))?;
        next.enabled = true;
        next.manual_source = true;
        if let Some(previous) = profile(&state)? {
            next.keep = previous.keep;
            next.quota_bytes = previous.quota_bytes;
        }
        arachnid_saveguard::save_json(&config_path(&state), &next)?;
        *state
            .saves
            .profile
            .lock()
            .map_err(|_| AppError::new("internal"))? = Some(next);
        record_error(&state, &Ok(()));
        Ok(true)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub fn open_save_folder(window: WebviewWindow, state: State<'_, AppState>) -> AppResult<()> {
    authorize(&window)?;
    let directory = if profile(&state)?.is_some() {
        store(&state)?.directory().to_path_buf()
    } else {
        backup_root(&state)
    };
    arachnid_saveguard::source::safe_directory(&directory)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let explorer = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .ok_or_else(|| AppError::new("saveFolderUnavailable"))?
            .join("explorer.exe");
        std::process::Command::new(explorer)
            .arg(directory)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|_| AppError::new("saveFolderUnavailable"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err(AppError::new("windowsOnly"))
    }
}

#[tauri::command]
pub fn delete_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    undo: bool,
) -> AppResult<()> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    let store = store(&state)?;
    if undo {
        store.undo_delete(&id)
    } else {
        store.delete(&id)
    }
}

#[tauri::command]
pub fn configure_saveguard(
    window: WebviewWindow,
    state: State<'_, AppState>,
    enabled: bool,
    keep: usize,
    quota_mib: u64,
) -> AppResult<()> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    let mut next = profile(&state)?.ok_or_else(|| AppError::new("saveNotConfigured"))?;
    next.enabled = enabled;
    next.keep = keep;
    next.quota_bytes = quota_mib
        .checked_mul(1024 * 1024)
        .ok_or_else(|| AppError::new("operationLimit"))?;
    Store::open(&backup_root(&state), next.clone())?;
    arachnid_saveguard::save_json(&config_path(&state), &next)?;
    *state
        .saves
        .profile
        .lock()
        .map_err(|_| AppError::new("internal"))? = Some(next);
    Ok(())
}

#[tauri::command]
pub async fn snapshot_saves(window: WebviewWindow, app: tauri::AppHandle) -> AppResult<()> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        let result = store(&state).and_then(|s| s.snapshot("manual", STABILITY_WINDOW).map(|_| ()));
        record_error(&state, &result);
        result
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub fn pin_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> AppResult<()> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    store(&state)?.pin(&id, pinned)
}

fn restore_gate(state: &AppState) -> AppResult<()> {
    commands::ensure_no_session(state)?;
    let report = state.report.lock().map_err(|_| AppError::new("internal"))?;
    let report = report.as_ref().ok_or_else(|| AppError::new("noReport"))?;
    arachnid_launch::ensure_game_stopped(std::path::Path::new(&report.root))
}

#[tauri::command]
pub async fn prepare_save_restore(
    window: WebviewWindow,
    app: tauri::AppHandle,
    id: String,
    producers_stopped: bool,
) -> AppResult<RestorePreview> {
    authorize(&window)?;
    if !producers_stopped {
        return Err(AppError::new("saveProducerActive"));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        store(&state)?
            .prepare_restore(&id, || restore_gate(&state))
            .map(preview)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub async fn apply_save_restore(
    window: WebviewWindow,
    app: tauri::AppHandle,
    digest: String,
    producers_stopped: bool,
    undo: bool,
) -> AppResult<()> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        let store = store(&state)?;
        let cancelling_preview = undo
            && store.operation()?.is_some_and(|j| {
                j.phase == arachnid_repair::Phase::Prepared && j.intent_count == 0
            });
        if !producers_stopped && !cancelling_preview {
            return Err(AppError::new("saveProducerActive"));
        }
        if cancelling_preview {
            return store.undo_restore(&digest, || Ok(()));
        }
        let result = if undo {
            store.undo_restore(&digest, || restore_gate(&state))
        } else {
            store.apply_restore(&digest, || restore_gate(&state))
        };
        record_error(&state, &result);
        result
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}
