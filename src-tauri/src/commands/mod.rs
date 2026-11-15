use crate::{AppState, authorize};
use arachnid_core::{
    AppError, AppResult, Language, ScanReport,
    coordination::InstallationLease,
    readiness::{self, LaunchDecision},
};
use arachnid_launch::{Session, SessionStatus};
use arachnid_repair::{Journal, Phase, registry::InstallationResources};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use tauri::{State, WebviewWindow};

pub(crate) fn ensure_no_session(state: &AppState) -> AppResult<()> {
    let mut session = state
        .session
        .lock()
        .map_err(|_| AppError::new("internal"))?;
    if let Some(active) = session.as_mut() {
        if active.status()?.state != "exited" {
            return Err(AppError::new("gameAlreadyRunning"));
        }
        *session = None;
    }
    Ok(())
}

pub(crate) fn report(state: &AppState) -> AppResult<ScanReport> {
    if state
        .scan
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .is_some()
    {
        return Err(AppError::new("scanBusy"));
    }
    state
        .report
        .lock()
        .map_err(|_| AppError::new("internal"))?
        .clone()
        .filter(|report| report.recognized)
        .ok_or_else(|| AppError::new("noReport"))
}

pub(crate) fn lease(state: &AppState, report: &ScanReport) -> AppResult<InstallationLease> {
    let root = arachnid_detector::validate_root(Path::new(&report.root))?;
    InstallationLease::acquire(&root, &state.data_directory.join("locks"))
}

pub(crate) fn journal_path(state: &AppState, lease: &InstallationLease) -> AppResult<PathBuf> {
    let directory = state.data_directory.join("transactions");
    std::fs::create_dir_all(&directory).map_err(|_| AppError::new("journalUnavailable"))?;
    Ok(directory.join(format!("{}.json", lease.id())))
}

pub(crate) fn initial_settings_path(
    state: &AppState,
    lease: &InstallationLease,
) -> AppResult<PathBuf> {
    Ok(journal_path(state, lease)?.with_extension("initial.json"))
}

fn settings_recovery_path(state: &AppState, lease: &InstallationLease) -> AppResult<PathBuf> {
    let initial = initial_settings_path(state, lease)?;
    if arachnid_repair::pending(&initial) {
        Ok(initial)
    } else {
        journal_path(state, lease)
    }
}

fn initialize_settings_for_installation(
    state: &AppState,
    lease: &InstallationLease,
    build: &str,
) -> AppResult<bool> {
    if !arachnid_core::game_build::supported(build) {
        return Ok(false);
    }
    let gate = || arachnid_launch::ensure_game_stopped(lease.root());
    gate()?;
    let path = initial_settings_path(state, lease)?;
    let manual_path = journal_path(state, lease)?;
    if crate::saves::pending(state)
        || crate::mods::pending(state, lease)
        || crate::skins::pending(state, lease)
        || arachnid_repair::pending(&manual_path)
        || arachnid_repair::pending(&path)
    {
        return Err(AppError::new("recoveryRequired"));
    }
    if path.exists() && Journal::load(&path)?.phase == Phase::Committed {
        return Ok(false);
    }
    if manual_path.exists() && Journal::load(&manual_path)?.phase == Phase::Prepared {
        return Ok(false);
    }
    let mut resources = InstallationResources { lease };
    let Some(plan) = arachnid_settings::initial_plan(lease.id(), build, &resources)? else {
        return Ok(false);
    };
    let mut journal = arachnid_repair::prepare(plan, &resources, &path)?;
    let digest = journal.digest.clone();
    arachnid_repair::apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        gate,
    )?;
    state.storage.record("native_settings_initialized");
    Ok(true)
}

#[tauri::command]
pub async fn initialize_game_settings(
    window: WebviewWindow,
    app: tauri::AppHandle,
) -> AppResult<bool> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        let report = report(&state)?;
        if !arachnid_core::game_build::supported(build(&report)?)
            || report
                .checks
                .iter()
                .any(|check| check.status == arachnid_core::CheckStatus::Error)
        {
            return Ok(false);
        }
        ensure_no_session(&state)?;
        let lease = lease(&state, &report)?;
        let executable = arachnid_detector::inspect_pe(
            &lease.root().join("Game.exe"),
            "Game.exe",
            &AtomicBool::new(false),
        )?;
        initialize_settings_for_installation(&state, &lease, &executable.sha256)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

pub(crate) fn build(report: &ScanReport) -> AppResult<&str> {
    report
        .binaries
        .iter()
        .find(|image| image.name.eq_ignore_ascii_case("Game.exe"))
        .map(|image| image.sha256.as_str())
        .ok_or_else(|| AppError::new("noReport"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStatus {
    decision: LaunchDecision,
    pub(super) session: SessionStatus,
    recovery_pending: bool,
    graphics_crash_advice: Option<GraphicsCrashAdvice>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum GraphicsCrashAdvice {
    Reshade,
    GraphicsMods,
}

fn graphics_crash_advice(
    session: &SessionStatus,
    components: Option<arachnid_mods::composition::Composition>,
) -> Option<GraphicsCrashAdvice> {
    const ACCESS_VIOLATION: i32 = 0xc000_0005_u32 as i32;
    if session.state != "exited" || session.exit_code != Some(ACCESS_VIOLATION) {
        return None;
    }
    let components = components?;
    if components.reshade {
        Some(GraphicsCrashAdvice::Reshade)
    } else if components.fix || components.dxvk {
        Some(GraphicsCrashAdvice::GraphicsMods)
    } else {
        None
    }
}

#[cfg(test)]
#[test]
fn graphics_crash_advice_matches_access_violation_and_active_mods() {
    use arachnid_mods::composition::Composition;

    let mut session = SessionStatus {
        state: "exited".into(),
        pid: Some(42),
        exit_code: Some(-1_073_741_819),
    };
    assert_eq!(
        graphics_crash_advice(&session, Some(Composition::from_bits(5).unwrap())),
        Some(GraphicsCrashAdvice::Reshade)
    );
    assert_eq!(
        graphics_crash_advice(&session, Some(Composition::DEFAULT)),
        Some(GraphicsCrashAdvice::GraphicsMods)
    );
    assert_eq!(graphics_crash_advice(&session, None), None);
    for exit_code in [Some(0), Some(-1_073_740_791), None] {
        session.exit_code = exit_code;
        assert_eq!(
            graphics_crash_advice(&session, Some(Composition::DEFAULT)),
            None
        );
    }
}

pub(super) fn status(state: &AppState) -> AppResult<LaunchStatus> {
    let mut report = report(state)?;
    let mut sessions = state
        .session
        .lock()
        .map_err(|_| AppError::new("internal"))?;
    let session = if let Some(active) = sessions.as_mut() {
        active.status()?
    } else {
        state
            .last_session
            .lock()
            .map_err(|_| AppError::new("internal"))?
            .clone()
            .unwrap_or(SessionStatus {
                state: "idle".into(),
                pid: None,
                exit_code: None,
            })
    };
    if session.state == "exited" {
        if sessions.is_some() {
            crate::saves::session_snapshot(state, "afterGame");
        }
        *sessions = None;
        *state
            .last_session
            .lock()
            .map_err(|_| AppError::new("internal"))? = Some(session.clone());
    }
    drop(sessions);
    let saves_pending = crate::saves::pending(state);
    let (installation_pending, components) = match lease(state, &report) {
        Ok(lease) if !saves_pending => {
            let pending = arachnid_repair::pending(&journal_path(state, &lease)?)
                || arachnid_repair::pending(&initial_settings_path(state, &lease)?)
                || crate::mods::pending(state, &lease)
                || crate::skins::pending(state, &lease);
            let components = if pending {
                None
            } else {
                crate::mods::qualify_report(state, &lease, &mut report)?
            };
            (pending, components)
        }
        _ => (session.state != "running", None),
    };
    let recovery_pending = saves_pending || installation_pending;
    let channel = arachnid_launch::qualified_game_channel(build(&report)?);
    let mut decision = readiness::decide(
        &report,
        channel.is_some(),
        recovery_pending && session.state != "running",
    );
    if matches!(session.state.as_str(), "running" | "unknown") {
        decision.ready = false;
        decision.reasons.push("gameAlreadyRunning".into());
    } else if let Err(error) = arachnid_launch::ensure_game_stopped(Path::new(&report.root)) {
        decision.ready = false;
        decision.reasons.push(error.code);
    }
    Ok(LaunchStatus {
        decision,
        graphics_crash_advice: graphics_crash_advice(&session, components),
        session,
        recovery_pending,
    })
}

#[tauri::command]
pub fn launch_status(window: WebviewWindow, state: State<'_, AppState>) -> AppResult<LaunchStatus> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    status(&state)
}

#[tauri::command]
pub async fn start_game(
    window: WebviewWindow,
    app: tauri::AppHandle,
    language: Language,
    diagnostics: tauri::ipc::Channel<ScanReport>,
) -> AppResult<LaunchStatus> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
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
        ensure_no_session(&state)?;
        let root = match report(&state) {
            Ok(report) => report.root,
            Err(error) if error.code == "noReport" => state
                .preferences
                .lock()
                .map_err(|_| AppError::new("internal"))?
                .installation_path
                .clone()
                .ok_or(error)?,
            Err(error) => return Err(error),
        };
        let root = arachnid_detector::validate_root(Path::new(&root))?;
        let lease = InstallationLease::acquire(&root, &state.data_directory.join("locks"))?;
        arachnid_launch::ensure_game_stopped(lease.root())?;
        let mut current = arachnid_detector::scan(lease.root(), &AtomicBool::new(false), |_| {})?;
        arachnid_diagnostics::evaluate(&mut current);
        // Publish the raw findings even if the mod journal needs manual recovery.
        let _ = crate::mods::qualify_report(&state, &lease, &mut current);
        *state.report.lock().map_err(|_| AppError::new("internal"))? = Some(current.clone());
        let _ = diagnostics.send(current.clone());
        if crate::saves::pending(&state)
            || arachnid_repair::pending(&journal_path(&state, &lease)?)
            || arachnid_repair::pending(&initial_settings_path(&state, &lease)?)
            || crate::mods::pending(&state, &lease)
            || crate::skins::pending(&state, &lease)
        {
            return Err(AppError::new("recoveryRequired"));
        }
        arachnid_launch::qualified_game_channel(build(&current)?)
            .ok_or_else(|| AppError::new("channelUnqualified"))?;
        if let Some(updated) =
            crate::mods::reconcile_graphics(&state, &app, &lease, &current, false)?
        {
            current = updated;
            let _ = diagnostics.send(current.clone());
        }
        if let Some(reason) = readiness::decide(&current, true, false)
            .reasons
            .into_iter()
            .next()
        {
            *state.report.lock().map_err(|_| AppError::new("internal"))? = Some(current);
            return Err(AppError::new(&reason));
        }
        initialize_settings_for_installation(&state, &lease, build(&current)?)?;
        synchronize_before_launch(synchronize_language_for_installation(
            &state,
            language,
            &lease,
            build(&current)?,
        ))?;
        let runtime = crate::mods::runtime_for_launch(&state, &app, &lease, &mut current)?;
        let runtime = crate::skins::runtime(&state, &app, &lease, runtime)?;
        let executable = arachnid_detector::inspect_pe(
            &lease.root().join("Game.exe"),
            "Game.exe",
            &AtomicBool::new(false),
        )?;
        let channel = arachnid_launch::qualified_game_channel(&executable.sha256)
            .ok_or_else(|| AppError::new("channelUnqualified"))?;
        let game = current
            .binaries
            .iter_mut()
            .find(|pe| pe.name.eq_ignore_ascii_case("Game.exe"))
            .ok_or_else(|| AppError::new("noReport"))?;
        *game = executable;
        crate::saves::session_snapshot(&state, "beforeGame");
        let session = Session::start_with_runtime(lease, &channel, runtime)?;
        state.lifecycle.started();
        *state
            .session
            .lock()
            .map_err(|_| AppError::new("internal"))? = Some(session);
        *state.report.lock().map_err(|_| AppError::new("internal"))? = Some(current);
        state.storage.record("game_started_direct");
        status(&state)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

fn synchronize_language_locked(
    state: &AppState,
    language: Language,
) -> AppResult<arachnid_settings::languages::LanguageSync> {
    ensure_no_session(state)?;
    let report = report(state)?;
    let lease = lease(state, &report)?;
    let executable = arachnid_detector::inspect_pe(
        &lease.root().join("Game.exe"),
        "Game.exe",
        &AtomicBool::new(false),
    )?;
    synchronize_language_for_installation(state, language, &lease, &executable.sha256)
}

fn synchronize_before_launch(
    result: AppResult<arachnid_settings::languages::LanguageSync>,
) -> AppResult<()> {
    match result {
        Ok(_) => Ok(()),
        Err(error) if error.code == "languageResourcesMissing" => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[test]
fn missing_language_resources_preserve_launch_but_write_failures_do_not() {
    assert!(synchronize_before_launch(Err(AppError::new("languageResourcesMissing"))).is_ok());
    assert_eq!(
        synchronize_before_launch(Err(AppError::new("writeFailed")))
            .unwrap_err()
            .code,
        "writeFailed"
    );
}

fn synchronize_language_for_installation(
    state: &AppState,
    language: Language,
    lease: &InstallationLease,
    build: &str,
) -> AppResult<arachnid_settings::languages::LanguageSync> {
    let preferences = state
        .preferences
        .lock()
        .map_err(|_| AppError::new("internal"))?;
    if preferences.language != Language::Auto && preferences.language != language {
        return Err(AppError::new("stalePlan"));
    }
    drop(preferences);
    let code = match language {
        Language::En => "en",
        Language::Fr => "fr",
        Language::De => "de",
        Language::It => "it",
        Language::Es => "es",
        Language::Auto => return Err(AppError::new("unsupportedSetting")),
    };
    arachnid_launch::ensure_game_stopped(lease.root())?;
    if crate::saves::pending(state)
        || arachnid_repair::pending(&journal_path(state, lease)?)
        || arachnid_repair::pending(&initial_settings_path(state, lease)?)
        || crate::mods::pending(state, lease)
        || crate::skins::pending(state, lease)
    {
        return Err(AppError::new("recoveryRequired"));
    }
    let directory = crate::mods::child_directory(&state.data_directory, "language-sync")?;
    let path = arachnid_repair::filesystem::resolve(&directory, &format!("{}.json", lease.id()))?;
    let result = arachnid_settings::languages::synchronize(
        &lease.id(),
        build,
        lease.root(),
        code,
        &mut InstallationResources { lease },
        &path,
        || arachnid_launch::ensure_game_stopped(lease.root()),
    )?;
    if result.changed {
        state.storage.record("game_language_synchronized");
    }
    Ok(result)
}

#[tauri::command]
pub async fn synchronize_game_language(
    window: WebviewWindow,
    app: tauri::AppHandle,
    language: Language,
) -> AppResult<arachnid_settings::languages::LanguageSync> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        synchronize_language_locked(&state, language)
    })
    .await
    .map_err(|_| AppError::new("internal"))?
}

#[tauri::command]
pub fn native_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<arachnid_settings::SettingsReport> {
    authorize(&window)?;
    let report = report(&state)?;
    arachnid_settings::read(build(&report)?, Path::new(&report.root))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPlan {
    digest: String,
    plan: arachnid_repair::OperationPlan,
}

#[tauri::command]
pub fn settings_operation(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> AppResult<Option<Journal>> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    ensure_no_session(&state)?;
    let report = report(&state)?;
    let lease = lease(&state, &report)?;
    let path = settings_recovery_path(&state, &lease)?;
    if !path.exists() {
        return Ok(None);
    }
    let journal = Journal::load(&path)?;
    if journal.plan.installation != lease.id() {
        return Err(AppError::new("recoveryRequired"));
    }
    if path == initial_settings_path(&state, &lease)? {
        arachnid_settings::validate_initial_plan(&journal.plan, &lease.id())?;
    } else {
        arachnid_settings::validate_plan(&journal.plan, &lease.id())?;
    }
    Ok(Some(journal))
}

#[tauri::command]
pub fn prepare_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
    changes: Vec<arachnid_settings::SettingChange>,
    profile: Option<arachnid_settings::profiles::Profile>,
) -> AppResult<SettingsPlan> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    ensure_no_session(&state)?;
    let report = report(&state)?;
    let lease = lease(&state, &report)?;
    arachnid_launch::ensure_game_stopped(lease.root())?;
    let current = arachnid_detector::inspect_pe(
        &lease.root().join("Game.exe"),
        "Game.exe",
        &AtomicBool::new(false),
    )?;
    let resources = InstallationResources { lease: &lease };
    let plan = if let Some(profile) = profile {
        arachnid_settings::profiles::plan_with_changes(
            lease.id(),
            &current.sha256,
            profile,
            changes,
            &resources,
        )?
    } else {
        arachnid_settings::plan(lease.id(), &current.sha256, changes, &resources)?
    };
    arachnid_settings::languages::validate_selection(lease.root(), &plan)?;
    if arachnid_repair::pending(&initial_settings_path(&state, &lease)?) {
        return Err(AppError::new("recoveryRequired"));
    }
    let path = journal_path(&state, &lease)?;
    if path.exists() {
        let prior = Journal::load(&path)?;
        if prior.phase == Phase::Prepared {
            let mut prior = prior;
            prior.phase = Phase::Cancelled;
            prior.save(&path)?;
        } else if !prior.terminal() {
            return Err(AppError::new("recoveryRequired"));
        }
        let archive = tempfile::Builder::new()
            .prefix("history-")
            .suffix(".json")
            .tempfile_in(
                path.parent()
                    .ok_or_else(|| AppError::new("journalUnavailable"))?,
            )
            .map_err(|_| AppError::new("journalUnavailable"))?;
        std::fs::copy(&path, archive.path()).map_err(|_| AppError::new("journalUnavailable"))?;
        archive
            .as_file()
            .sync_all()
            .map_err(|_| AppError::new("journalUnavailable"))?;
        archive
            .keep()
            .map_err(|_| AppError::new("journalUnavailable"))?;
    }
    let journal = arachnid_repair::prepare(plan, &resources, &path)?;
    Ok(SettingsPlan {
        digest: journal.digest,
        plan: journal.plan,
    })
}

#[tauri::command]
pub fn apply_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
    digest: String,
) -> AppResult<()> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    ensure_no_session(&state)?;
    let report = report(&state)?;
    let lease = lease(&state, &report)?;
    if arachnid_repair::pending(&initial_settings_path(&state, &lease)?) {
        return Err(AppError::new("recoveryRequired"));
    }
    let path = journal_path(&state, &lease)?;
    let mut journal = Journal::load(&path)?;
    if journal.plan.installation != lease.id() {
        return Err(AppError::new("invalidPlan"));
    }
    arachnid_settings::validate_plan(&journal.plan, &lease.id())?;
    let current = arachnid_detector::inspect_pe(
        &lease.root().join("Game.exe"),
        "Game.exe",
        &AtomicBool::new(false),
    )?;
    if current.sha256 != arachnid_settings::LOCAL_BUILD {
        return Err(AppError::new("channelUnqualified"));
    }
    arachnid_settings::languages::validate_selection(lease.root(), &journal.plan)?;
    let mut resources = InstallationResources { lease: &lease };
    arachnid_repair::apply(
        &mut journal,
        &digest,
        &mut resources,
        &path,
        &AtomicBool::new(false),
        || arachnid_launch::ensure_game_stopped(lease.root()),
    )?;
    state.storage.record("settings_applied");
    Ok(())
}

#[tauri::command]
pub fn restore_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
    digest: String,
) -> AppResult<()> {
    authorize(&window)?;
    let _operation = state
        .operations
        .try_lock()
        .map_err(|_| AppError::new("installationBusy"))?;
    ensure_no_session(&state)?;
    let report = report(&state)?;
    let lease = lease(&state, &report)?;
    arachnid_launch::ensure_game_stopped(lease.root())?;
    let path = settings_recovery_path(&state, &lease)?;
    let mut journal = Journal::load(&path)?;
    if journal.plan.installation != lease.id() || journal.digest != digest {
        return Err(AppError::new("approvalInvalid"));
    }
    if path == initial_settings_path(&state, &lease)? {
        arachnid_settings::validate_initial_plan(&journal.plan, &lease.id())?;
    } else {
        arachnid_settings::validate_plan(&journal.plan, &lease.id())?;
    }
    arachnid_repair::rollback_guarded(
        &mut journal,
        &mut InstallationResources { lease: &lease },
        &path,
        || arachnid_launch::ensure_game_stopped(lease.root()),
    )?;
    state.storage.record("settings_restored");
    Ok(())
}
