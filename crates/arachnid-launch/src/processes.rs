use arachnid_core::{AppError, AppResult};
use std::path::Path;

#[cfg(windows)]
pub fn game_present(root: &Path) -> AppResult<bool> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_NO_MORE_FILES, GetLastError, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
    };
    // An inaccessible matching process is ambiguous, never evidence that the game has stopped.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(AppError::new("processStateUnknown"));
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut available = Process32FirstW(snapshot, &mut entry);
        let mut found = false;
        while available != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length]).eq_ignore_ascii_case("Game.exe")
            {
                let process =
                    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
                if process.is_null() {
                    CloseHandle(snapshot);
                    return Err(AppError::new("processStateUnknown"));
                }
                let mut image = vec![0u16; 32768];
                let mut length = image.len() as u32;
                let success =
                    QueryFullProcessImageNameW(process, 0, image.as_mut_ptr(), &mut length);
                CloseHandle(process);
                if success == 0 {
                    CloseHandle(snapshot);
                    return Err(AppError::new("processStateUnknown"));
                }
                let actual =
                    std::path::PathBuf::from(String::from_utf16_lossy(&image[..length as usize]))
                        .canonicalize();
                let expected = root.join("Game.exe").canonicalize();
                match (actual, expected) {
                    (Ok(actual), Ok(expected))
                        if actual
                            .to_string_lossy()
                            .eq_ignore_ascii_case(&expected.to_string_lossy()) =>
                    {
                        found = true;
                        break;
                    }
                    (Ok(_), Ok(_)) => (),
                    _ => {
                        CloseHandle(snapshot);
                        return Err(AppError::new("processStateUnknown"));
                    }
                }
            }
            available = Process32NextW(snapshot, &mut entry);
        }
        let error = GetLastError();
        CloseHandle(snapshot);
        if !found && error != ERROR_NO_MORE_FILES {
            return Err(AppError::new("processStateUnknown"));
        }
        Ok(found)
    }
}

#[cfg(not(windows))]
pub fn game_present(_: &Path) -> AppResult<bool> {
    Err(AppError::new("processStateUnknown"))
}
