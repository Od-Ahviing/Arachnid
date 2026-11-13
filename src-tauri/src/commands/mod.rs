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

fn journal_path(state: &AppState, lease: &InstallationLease) -> AppResult<PathBuf> {
    let directory = state.data_directory.join("transactions");
    std::fs::create_dir_all(&directory).map_err(|_| AppError::new("journalUnavailable"))?;
    Ok(directory.join(format!("{}.json", lease.id())))
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
        Ok(lease) if !saves_pending => (
            arachnid_repair::pending(&journal_path(state, &lease)?)
                || crate::mods::pending(state, &lease),
            crate::mods::qualify_report(state, &lease, &mut report)?,
        ),
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
) -> AppResult<LaunchStatus> {
    authorize(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let state = app.state::<AppState>();
        let _operation = state
            .operations
            .try_lock()
            .map_err(|_| AppError::new("installationBusy"))?;
        ensure_no_session(&state)?;
        synchronize_language_locked(&state, language)?;
        let previous = report(&state)?;
        let lease = lease(&state, &previous)?;
        if crate::saves::pending(&state)
            || arachnid_repair::pending(&journal_path(&state, &lease)?)
            || crate::mods::pending(&state, &lease)
            || crate::skins::pending(&state, &lease)
        {
            return Err(AppError::new("recoveryRequired"));
        }
        let mut current =
            arachnid_detector::scan(Path::new(&previous.root), &AtomicBool::new(false), |_| {})?;
        arachnid_diagnostics::evaluate(&mut current);
        arachnid_launch::qualified_game_channel(build(&current)?)
            .ok_or_else(|| AppError::new("channelUnqualified"))?;
        let runtime = crate::mods::runtime_for_launch(&state, &app, &lease, &mut current)?;
        let runtime = crate::skins::runtime(&state, &app, &lease, runtime)?;
        if !readiness::decide(&current, true, false).ready {
            return Err(AppError::new("diagnosticBlocking"));
        }
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
    let report = report(state)?;
    let lease = lease(state, &report)?;
    arachnid_launch::ensure_game_stopped(lease.root())?;
    if crate::saves::pending(state)
        || arachnid_repair::pending(&journal_path(state, &lease)?)
        || crate::mods::pending(state, &lease)
        || crate::skins::pending(state, &lease)
    {
        return Err(AppError::new("recoveryRequired"));
    }
    if !arachnid_repair::registry::section_exists("Install")? {
        return Err(AppError::new("unsupportedSetting"));
    }
    let executable = arachnid_detector::inspect_pe(
        &lease.root().join("Game.exe"),
        "Game.exe",
        &AtomicBool::new(false),
    )?;
    let directory = crate::mods::child_directory(&state.data_directory, "language-sync")?;
    let path = arachnid_repair::filesystem::resolve(&directory, &format!("{}.json", lease.id()))?;
    let result = arachnid_settings::languages::synchronize(
        &lease.id(),
        &executable.sha256,
        lease.root(),
        code,
        &mut InstallationResources { lease: &lease },
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
    let path = journal_path(&state, &lease)?;
    if !path.exists() {
        return Ok(None);
    }
    let journal = Journal::load(&path)?;
    if journal.plan.installation != lease.id() {
        return Err(AppError::new("recoveryRequired"));
    }
    arachnid_settings::validate_plan(&journal.plan, &lease.id())?;
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
    let path = journal_path(&state, &lease)?;
    let mut journal = Journal::load(&path)?;
    if journal.plan.installation != lease.id() || journal.digest != digest {
        return Err(AppError::new("approvalInvalid"));
    }
    arachnid_settings::validate_plan(&journal.plan, &lease.id())?;
    arachnid_repair::rollback_guarded(
        &mut journal,
        &mut InstallationResources { lease: &lease },
        &path,
        || arachnid_launch::ensure_game_stopped(lease.root()),
    )?;
    state.storage.record("settings_restored");
    Ok(())
}
