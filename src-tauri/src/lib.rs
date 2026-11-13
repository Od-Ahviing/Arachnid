mod commands;
mod lifecycle;
mod mods;
mod saves;
mod skins;
mod storage;

use arachnid_core::{AppError, AppResult, Language, Preferences, ScanReport};
use std::{
    fs::{File, OpenOptions},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

struct AppState {
    lifecycle: lifecycle::Lifecycle,
    saves: saves::SaveState,
    data_directory: std::path::PathBuf,
    operations: Mutex<()>,
    session: Mutex<Option<arachnid_launch::Session>>,
    last_session: Mutex<Option<arachnid_launch::SessionStatus>>,
    storage: storage::Storage,
    preferences: Mutex<Preferences>,
    report: Mutex<Option<ScanReport>>,
    scan: Mutex<Option<Arc<AtomicBool>>>,
    _instance_lock: File,
}

fn authorize(window: &WebviewWindow) -> AppResult<()> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err(AppError::new("unauthorized"))
    }
}

#[tauri::command]
fn load_preferences(window: WebviewWindow, state: State<'_, AppState>) -> AppResult<Preferences> {
    authorize(&window)?;
    state.storage.load()
}

#[tauri::command]
fn save_language(
    window: WebviewWindow,
    state: State<'_, AppState>,
    language: Language,
) -> AppResult<()> {
    authorize(&window)?;
    let mut preferences = state
        .preferences
        .lock()
        .map_err(|_| AppError::new("internal"))?;
    let mut next = preferences.clone();
    next.language = language;
    state.storage.save(&next)?;
    *preferences = next;
    Ok(())
}

#[tauri::command]
async fn pick_game_directory(
    window: WebviewWindow,
    app: tauri::AppHandle,
) -> AppResult<Option<String>> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|p| p.to_string())
    })
    .await
    .map_err(|_| AppError::new("dialogUnavailable"))
}

#[tauri::command]
fn cancel_scan(window: WebviewWindow, state: State<'_, AppState>) -> AppResult<()> {
    authorize(&window)?;
    if let Some(cancel) = state
        .scan
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .as_ref()
    {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
async fn scan_installation(
    window: WebviewWindow,
    app: tauri::AppHandle,
    path: String,
) -> AppResult<ScanReport> {
    authorize(&window)?;
    let state = app.state::<AppState>();
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut active = state.scan.lock().map_err(|_| AppError::new("internal"))?;
        if active.is_some() {
            return Err(AppError::new("scanBusy"));
        }
        *active = Some(cancel.clone());
    }
    let worker_app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let worker_state = worker_app.state::<AppState>();
        let _operation = worker_state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        commands::ensure_no_session(&worker_state)?;
        let mut report = arachnid_detector::scan(Path::new(&path), &cancel, |progress| {
            let _ = worker_app.emit_to("main", "scan-progress", progress);
        })?;
        arachnid_diagnostics::evaluate(&mut report);
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::new("cancelled"));
        }
        Ok(report)
    })
    .await
    .map_err(|_| AppError::new("internal"))
    .and_then(|r| r);
    *state.scan.lock().map_err(|_| AppError::new("internal"))? = None;
    state.storage.record(if result.is_ok() {
        "scan_completed"
    } else {
        "scan_failed"
    });
    let report = result?;
    if report.recognized {
        let mut preferences = state
            .preferences
            .lock()
            .map_err(|_| AppError::new("internal"))?;
        let mut next = preferences.clone();
        next.installation_path = Some(report.root.clone());
        state.storage.save(&next)?;
        *preferences = next;
    }
    *state.report.lock().map_err(|_| AppError::new("internal"))? = Some(report.clone());
    Ok(report)
}

#[tauri::command]
fn forget_installation(window: WebviewWindow, state: State<'_, AppState>) -> AppResult<()> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    commands::ensure_no_session(&state)?;
    if state
        .scan
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .is_some()
    {
        return Err(AppError::new("scanBusy"));
    }
    let mut preferences = state
        .preferences
        .lock()
        .map_err(|_| AppError::new("internal"))?;
    let mut next = preferences.clone();
    next.installation_path = None;
    state.storage.save(&next)?;
    *preferences = next;
    *state.report.lock().map_err(|_| AppError::new("internal"))? = None;
    Ok(())
}

#[tauri::command]
async fn export_report(window: WebviewWindow, app: tauri::AppHandle) -> AppResult<bool> {
    authorize(&window)?;
    let report = app
        .state::<AppState>()
        .report
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .clone()
        .ok_or_else(|| AppError::new("noReport"))?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(destination) = app
            .dialog()
            .file()
            .set_file_name("arachnid-diagnostics.json")
            .add_filter("JSON", &["json"])
            .blocking_save_file()
        else {
            return Ok(false);
        };
        let path = destination
            .into_path()
            .map_err(|_| AppError::new("invalidPath"))?;
        let parent = path
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .ok_or_else(|| AppError::new("exportFailed"))?;
        let game_root = Path::new(&report.root)
            .canonicalize()
            .map_err(|_| AppError::new("directoryUnavailable"))?;
        if parent.starts_with(&game_root) {
            return Err(AppError::new("exportInsideGame"));
        }
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| AppError::new("exportFailed"))?;
        serde_json::to_writer_pretty(&mut file, &report.redacted())
            .map_err(|_| AppError::new("exportFailed"))?;
        file.as_file()
            .sync_all()
            .map_err(|_| AppError::new("exportFailed"))?;
        file.persist(path)
            .map_err(|_| AppError::new("exportFailed"))?;
        Ok(true)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let instance_lock = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(directory.join("instance.lock"))?;
            instance_lock
                .try_lock()
                .map_err(|_| "Arachnid is already open")?;
            let storage = storage::Storage::new(&directory).map_err(|e| e.code)?;
            let preferences = storage.load().unwrap_or_default();
            storage.record("launcher_started");
            // Startup remains usable for reporting a damaged config; saving still refuses to overwrite it.
            app.manage(AppState {
                lifecycle: lifecycle::Lifecycle::default(),
                saves: saves::SaveState::default(),
                data_directory: directory,
                operations: Mutex::new(()),
                session: Mutex::new(None),
                last_session: Mutex::new(None),
                storage,
                preferences: Mutex::new(preferences),
                report: Mutex::new(None),
                scan: Mutex::new(None),
                _instance_lock: instance_lock,
            });
            saves::initialize(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_preferences,
            mods::mod_status,
            mods::select_mod,
            mods::prepare_pack,
            mods::apply_pack,
            skins::skin_status,
            skins::prepare_skin,
            skins::apply_skin,
            save_language,
            pick_game_directory,
            scan_installation,
            cancel_scan,
            forget_installation,
            export_report,
            commands::launch_status,
            commands::start_game,
            commands::synchronize_game_language,
            commands::native_settings,
            commands::prepare_settings,
            commands::apply_settings,
            commands::restore_settings,
            commands::settings_operation,
            saves::save_status,
            saves::open_save_folder,
            saves::delete_save,
            saves::detect_save_source,
            saves::choose_save_source,
            saves::configure_saveguard,
            saves::snapshot_saves,
            saves::pin_save,
            saves::prepare_save_restore,
            saves::apply_save_restore
        ])
        .build(tauri::generate_context!())
        .expect("Arachnid could not start")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
                && app.state::<AppState>().lifecycle.in_background()
            {
                api.prevent_exit();
            }
        });
}
