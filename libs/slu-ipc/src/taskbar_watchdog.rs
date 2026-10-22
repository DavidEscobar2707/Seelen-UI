//! Taskbar watchdog: restores the native Windows taskbar when the Seelen process dies without
//! a clean exit (TerminateProcess, crash in native code), which no panic hook can cover.
//!
//! Protocol: before hiding the taskbar the app writes a state file (`<pid> <appbar state>`) and
//! spawns `slu taskbar-watchdog --pid <pid> --state <n> --file <path>`. The watchdog blocks on
//! the app process handle (no polling). A clean exit deletes the file; if the file is still there
//! (and still names this pid) when the process ends, the watchdog restores and deletes it.

use std::path::Path;

use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND},
        System::Threading::{INFINITE, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        UI::{
            Shell::{ABM_SETSTATE, APPBARDATA, SHAppBarMessage},
            WindowsAndMessaging::{FindWindowExW, FindWindowW, SW_SHOWNORMAL, ShowWindowAsync},
        },
    },
    core::w,
};

/// `<pid> <state>`
pub fn format_state(pid: u32, state: u32) -> String {
    format!("{pid} {state}\n")
}

pub fn parse_state(content: &str) -> Option<(u32, u32)> {
    let mut parts = content.split_whitespace();
    let pid = parts.next()?.parse().ok()?;
    let state = parts.next()?.parse().ok()?;
    if parts.next().is_some() || state > 3 {
        return None;
    }
    Some((pid, state))
}

pub fn read_state_file(path: &Path) -> Option<(u32, u32)> {
    parse_state(&std::fs::read_to_string(path).ok()?)
}

/// Sets the saved AppBar state on every taskbar and shows them again.
pub fn restore_taskbars(state: u32) {
    unsafe {
        let mut handles = Vec::new();
        if let Ok(hwnd) = FindWindowW(w!("Shell_TrayWnd"), None) {
            handles.push(hwnd);
        }
        let mut prev: Option<HWND> = None;
        while let Ok(hwnd) = FindWindowExW(None, prev, w!("Shell_SecondaryTrayWnd"), None) {
            handles.push(hwnd);
            prev = Some(hwnd);
        }
        for hwnd in handles {
            let mut data = APPBARDATA {
                cbSize: std::mem::size_of::<APPBARDATA>() as u32,
                hWnd: hwnd,
                lParam: windows::Win32::Foundation::LPARAM(state as isize),
                ..Default::default()
            };
            SHAppBarMessage(ABM_SETSTATE, &mut data);
            let _ = ShowWindowAsync(hwnd, SW_SHOWNORMAL);
        }
    }
}

/// Blocks until `pid` ends (or immediately when it cannot be opened, i.e. it is already gone),
/// then restores when the state file still names that pid.
pub fn run(pid: u32, state: u32, file: &Path) {
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            WaitForSingleObject(handle, INFINITE);
            let _ = CloseHandle(handle);
        }
    }
    // a clean exit deletes the file, a newer app instance rewrites it with its own pid
    if read_state_file(file).is_some_and(|(file_pid, _)| file_pid == pid) {
        restore_taskbars(state);
        let _ = std::fs::remove_file(file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrip() {
        assert_eq!(parse_state(&format_state(1234, 0)), Some((1234, 0)));
        assert_eq!(parse_state(&format_state(7, 2)), Some((7, 2)));
    }

    #[test]
    fn state_rejects_garbage() {
        assert_eq!(parse_state(""), None);
        assert_eq!(parse_state("12"), None);
        assert_eq!(parse_state("a b"), None);
        assert_eq!(parse_state("1 9"), None);
        assert_eq!(parse_state("1 2 3"), None);
    }
}
