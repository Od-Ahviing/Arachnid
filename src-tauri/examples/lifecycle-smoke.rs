#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[path = "../src/lifecycle.rs"]
mod lifecycle;

use arachnid_launch::{SessionStatus, presentation::Action};
use arachnid_saveguard::{SourceProfile, Store};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;

fn phase(root: &Path, phase: &str) -> Result<(), String> {
    fs::write(root.join("phase.txt"), phase).map_err(|e| e.to_string())
}

fn exercise(app: &tauri::AppHandle, root: &Path) -> Result<(), String> {
    let source = root.join("saves");
    let backups = root.join("backups");
    fs::create_dir_all(&source).map_err(|e| e.to_string())?;
    fs::create_dir_all(&backups).map_err(|e| e.to_string())?;
    fs::write(source.join("slot.dat"), b"generation A").map_err(|e| e.to_string())?;
    let profile = SourceProfile::new(&source, &backups).map_err(|e| e.code)?;
    let store = Store::open(&backups, profile).map_err(|e| e.code)?;
    store
        .reconcile("watch", Duration::ZERO)
        .map_err(|e| e.code)?;
    phase(root, "interface-ready")?;
    thread::sleep(Duration::from_secs(5));
    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target/debug/arachnid-process-harness.exe");
    let mut command = Command::new(harness);
    command
        .args(["game-window", "1000", "0"])
        .arg(root)
        .arg("fixture")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let state = app.state::<lifecycle::Lifecycle>();
    state.started();
    let deadline = Instant::now() + Duration::from_secs(40);
    let mut background_at = None;
    let mut observed_destruction = false;
    loop {
        if Instant::now() > deadline {
            let _ = child.kill();
            return Err("lifecycle timeout".into());
        }
        let exited = child.try_wait().map_err(|e| e.to_string())?.is_some();
        let session = SessionStatus {
            state: if exited { "exited" } else { "running" }.into(),
            pid: Some(child.id()),
            exit_code: exited.then_some(0),
        };
        if exited && state.in_background() {
            phase(root, "reopening")?;
        }
        let action = lifecycle::update(app, &state, &session);
        if action == Action::ReleaseInterface {
            if !root.join("window-visible").exists() {
                return Err("interface removed before fixture window".into());
            }
            background_at = Some(Instant::now());
        }
        if state.in_background()
            && app.get_webview_window("main").is_none()
            && !observed_destruction
        {
            observed_destruction = true;
            fs::write(source.join("slot.dat"), b"generation B").map_err(|e| e.to_string())?;
            store
                .reconcile("watch", Duration::from_millis(750))
                .map_err(|e| e.code)?;
            if store.history().map_err(|e| e.code)?.len() != 2 {
                return Err("background SaveGuard copy missing".into());
            }
            phase(root, "background-copy-verified")?;
        }
        if background_at.is_some_and(|at| at.elapsed() > Duration::from_secs(10)) {
            fs::write(root.join("release-window"), b"exit").map_err(|e| e.to_string())?;
        }
        if action == Action::ReopenInterface {
            if !observed_destruction || app.get_webview_window("main").is_none() {
                return Err("window was not recreated".into());
            }
            phase(root, "reopened")?;
            thread::sleep(Duration::from_secs(5));
            phase(root, "passed")?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn main() {
    let Some(root) = std::env::args_os().nth(1).map(PathBuf::from) else {
        return;
    };
    if !root.is_absolute() || !root.is_dir() {
        return;
    }
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "org.arachnid.lifecycle-smoke".into();
    context.config_mut().app.windows[0].title = "Arachnid · Lifecycle test".into();
    context.config_mut().app.windows[0].url = tauri::WebviewUrl::App("index.html".into());
    tauri::Builder::default()
        .setup(move |app| {
            app.manage(lifecycle::Lifecycle::default());
            let app = app.handle().clone();
            thread::spawn(move || match exercise(&app, &root) {
                Ok(()) => app.exit(0),
                Err(error) => {
                    let _ = fs::write(root.join("failure.txt"), error);
                    app.exit(2);
                }
            });
            Ok(())
        })
        .build(context)
        .expect("original lifecycle fixture failed to build")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
                && app.state::<lifecycle::Lifecycle>().in_background()
            {
                api.prevent_exit();
            }
        });
}
