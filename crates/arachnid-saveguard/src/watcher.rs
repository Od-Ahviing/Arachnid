use std::{path::Path, time::Duration};

/// Notifications are hints: callers still reconcile the complete bounded group periodically.
pub struct Watcher {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl Watcher {
    pub fn new(path: &Path) -> Option<Self> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::*;
            let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let handle = unsafe {
                FindFirstChangeNotificationW(
                    name.as_ptr(),
                    1,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_SIZE
                        | FILE_NOTIFY_CHANGE_LAST_WRITE,
                )
            };
            if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE || handle.is_null() {
                return None;
            }
            Some(Self { handle })
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            None
        }
    }

    pub fn changed(&self, timeout: Duration) -> bool {
        #[cfg(windows)]
        {
            let changed = unsafe {
                windows_sys::Win32::System::Threading::WaitForSingleObject(
                    self.handle,
                    timeout.as_millis().min(u32::MAX as u128) as u32,
                )
            } == windows_sys::Win32::Foundation::WAIT_OBJECT_0;
            if changed {
                unsafe {
                    windows_sys::Win32::Storage::FileSystem::FindNextChangeNotification(
                        self.handle,
                    );
                }
            }
            changed
        }
        #[cfg(not(windows))]
        {
            std::thread::sleep(timeout);
            false
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Storage::FileSystem::FindCloseChangeNotification(self.handle);
        }
    }
}
