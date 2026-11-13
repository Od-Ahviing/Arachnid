#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    ReleaseInterface,
    ReopenInterface,
}

#[derive(Default)]
pub struct Presentation {
    waiting: bool,
    background: bool,
}

impl Presentation {
    pub fn started(&mut self) {
        self.waiting = true;
    }
    pub fn background(&self) -> bool {
        self.background
    }
    pub fn observe(&self, running: bool, visible_game_window: bool) -> Action {
        if self.background && !running {
            Action::ReopenInterface
        } else if self.waiting && running && visible_game_window {
            Action::ReleaseInterface
        } else {
            Action::None
        }
    }
    pub fn released(&mut self) {
        self.waiting = false;
        self.background = true;
    }
    pub fn reopened(&mut self) {
        self.waiting = false;
        self.background = false;
    }
    pub fn exited_before_window(&mut self) {
        self.waiting = false;
    }
}

#[cfg(windows)]
pub fn has_visible_game_window(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, RECT},
        UI::WindowsAndMessaging::*,
    };
    struct Search {
        pid: u32,
        found: bool,
    }
    unsafe extern "system" fn visit(window: HWND, parameter: LPARAM) -> i32 {
        unsafe {
            let search = &mut *(parameter as *mut Search);
            let mut pid = 0;
            GetWindowThreadProcessId(window, &mut pid);
            let mut rect = RECT::default();
            if pid == search.pid
                && IsWindowVisible(window) != 0
                && GetWindow(window, GW_OWNER).is_null()
                && GetClientRect(window, &mut rect) != 0
                && rect.right > rect.left
                && rect.bottom > rect.top
            {
                search.found = true;
                return 0;
            }
            1
        }
    }
    if pid == 0 {
        return false;
    }
    let mut search = Search { pid, found: false };
    unsafe {
        EnumWindows(Some(visit), &mut search as *mut Search as LPARAM);
    }
    search.found
}

#[cfg(not(windows))]
pub fn has_visible_game_window(_: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interface_lifetime_waits_for_a_real_window_and_reopens_after_exit() {
        let mut state = Presentation::default();
        state.started();
        assert_eq!(state.observe(true, false), Action::None);
        assert_eq!(state.observe(false, false), Action::None);
        assert_eq!(state.observe(true, true), Action::ReleaseInterface);
        state.released();
        assert_eq!(state.observe(true, false), Action::None);
        assert_eq!(state.observe(false, false), Action::ReopenInterface);
        assert_eq!(state.observe(false, false), Action::ReopenInterface);
        state.reopened();
        assert_eq!(state.observe(false, false), Action::None);
    }
    #[test]
    fn failed_or_console_only_launch_does_not_remove_the_launcher() {
        let mut state = Presentation::default();
        state.started();
        state.exited_before_window();
        assert_eq!(state.observe(true, true), Action::None);
        assert!(!has_visible_game_window(0));
    }
}
