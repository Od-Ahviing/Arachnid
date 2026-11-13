use arachnid_launch::{
    SessionStatus,
    presentation::{Action, Presentation, has_visible_game_window},
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Manager;

#[derive(Default)]
pub struct Lifecycle {
    state: Mutex<Presentation>,
    keep_alive: AtomicBool,
}

impl Lifecycle {
    pub fn started(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.started();
        }
    }
    pub fn in_background(&self) -> bool {
        self.keep_alive.load(Ordering::Relaxed)
    }
}

pub fn update(app: &tauri::AppHandle, lifecycle: &Lifecycle, session: &SessionStatus) -> Action {
    let Ok(mut presentation) = lifecycle.state.lock() else {
        return Action::None;
    };
    let running = session.state == "running";
    let visible = session.state == "running" && session.pid.is_some_and(has_visible_game_window);
    match presentation.observe(running, visible) {
        Action::ReleaseInterface => {
            if let Some(window) = app.get_webview_window("main") {
                lifecycle.keep_alive.store(true, Ordering::Relaxed);
                if window.destroy().is_ok() {
                    presentation.released();
                    return Action::ReleaseInterface;
                } else {
                    lifecycle.keep_alive.store(false, Ordering::Relaxed);
                }
            }
        }
        Action::ReopenInterface => {
            let result = if let Some(window) = app.get_webview_window("main") {
                Ok(window)
            } else {
                tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])
                    .and_then(|builder| builder.build())
            };
            if let Ok(window) = result {
                let _ = window.show();
                let _ = window.set_focus();
                presentation.reopened();
                lifecycle.keep_alive.store(false, Ordering::Relaxed);
                return Action::ReopenInterface;
            }
        }
        Action::None => {
            if !running && !presentation.background() {
                presentation.exited_before_window();
            }
        }
    }
    Action::None
}
