use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
};

use seelen_core::state::WegItemData;
use seelen_core::system_state::RelaunchArguments;
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Shell::ABS_ALWAYSONTOP,
        WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL},
    },
};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

use crate::{
    error::Result,
    utils::constants::SEELEN_COMMON,
    windows_api::{AppBarData, AppBarDataState, WindowEnumerator, WindowsApi},
};

/// Tracks whether we hid the native taskbar, so we only restore what we hid
/// (hiding Shell_TrayWnd does not need elevation, so the app does it directly).
static NATIVE_TASKBAR_HIDDEN: AtomicBool = AtomicBool::new(false);
/// AppBar state the taskbar had before we touched it, restored everywhere (never hardcoded).
static ORIGINAL_STATE: AtomicU32 = AtomicU32::new(ABS_ALWAYSONTOP);
/// The watchdog process is spawned once per app process; it outlives hide/restore cycles.
static WATCHDOG_SPAWNED: AtomicBool = AtomicBool::new(false);

/// Primary `Shell_TrayWnd` handle at the last hide, a different one means explorer restarted.
static LAST_TRAY_HWND: AtomicIsize = AtomicIsize::new(0);

const STATE_FILE: &str = "taskbar-state.txt";

fn state_file() -> PathBuf {
    SEELEN_COMMON.app_data_dir().join(STATE_FILE)
}

fn show_taskbars_with_state(state: u32) -> Result<()> {
    for hwnd in get_taskbars_handles()? {
        AppBarData::from_handle(hwnd).set_state(AppBarDataState::from(state));
        WindowsApi::show_window_async(hwnd, SW_SHOWNORMAL)?;
    }
    Ok(())
}

/// A previous instance died hard (and its watchdog too): restore from the file it left.
/// Call before hiding the taskbar on startup.
pub fn restore_after_previous_crash() {
    let file = state_file();
    if let Some((pid, state)) = slu_ipc::taskbar_watchdog::read_state_file(&file) {
        log::warn!("Found a stale taskbar state file (pid {pid}), restoring state {state}");
        if let Err(err) = show_taskbars_with_state(state) {
            log::error!("Failed to restore taskbar after previous crash: {err:?}");
        }
    }
    let _ = std::fs::remove_file(file);
}

/// Records the original state, persists it and starts the watchdog (once).
fn arm_watchdog() {
    let handles = get_taskbars_handles().unwrap_or_default();
    if let Some(hwnd) = handles.first() {
        ORIGINAL_STATE.store(
            AppBarData::from_handle(*hwnd).get_state(),
            Ordering::Release,
        );
    }
    let state = ORIGINAL_STATE.load(Ordering::Acquire);
    let pid = std::process::id();
    let file = state_file();
    if let Err(err) = std::fs::write(&file, slu_ipc::taskbar_watchdog::format_state(pid, state)) {
        log::error!("Failed to write taskbar state file: {err}");
        return;
    }
    if WATCHDOG_SPAWNED.swap(true, Ordering::AcqRel) {
        return;
    }
    let Some(slu) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("slu.exe")))
        .filter(|p| p.exists())
    else {
        log::warn!("slu.exe not found next to the app, taskbar watchdog disabled");
        WATCHDOG_SPAWNED.store(false, Ordering::Release);
        return;
    };
    let spawn = |flags: u32| {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&slu)
            .args(["taskbar-watchdog", "--pid", &pid.to_string()])
            .args(["--state", &state.to_string()])
            .arg("--file")
            .arg(&file)
            .creation_flags(flags)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
    };
    // CREATE_NO_WINDOW | CREATE_BREAKAWAY_FROM_JOB, falling back when the job forbids breakaway
    match spawn(0x0800_0000 | 0x0100_0000).or_else(|_| spawn(0x0800_0000)) {
        Ok(child) => log::info!("Taskbar watchdog started (pid {})", child.id()),
        Err(err) => {
            log::error!("Failed to start taskbar watchdog: {err}");
            WATCHDOG_SPAWNED.store(false, Ordering::Release);
        }
    }
}

pub fn get_taskbars_handles() -> Result<Vec<HWND>> {
    let mut founds = Vec::new();
    WindowEnumerator::new().for_each(|window| {
        let hwnd = window.hwnd();
        let class = WindowsApi::get_class(hwnd).unwrap_or_default();
        if (class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd")
            && WindowsApi::get_window_text(hwnd).is_empty()
        {
            founds.push(hwnd);
        }
    })?;
    Ok(founds)
}

fn primary_tray_hwnd() -> isize {
    get_taskbars_handles()
        .ok()
        .and_then(|h| h.first().map(|h| h.0 as isize))
        .unwrap_or(0)
}

/// explorer.exe broadcasts `TaskbarCreated` to top-level windows when it (re)creates the
/// taskbar: hide the new one and register our app bars again. Zero cost until it happens.
fn init_explorer_restart_handler() {
    use windows::{Win32::UI::WindowsAndMessaging::RegisterWindowMessageW, core::w};
    let msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    if msg == 0 {
        log::error!("Failed to register the TaskbarCreated message");
        return;
    }
    crate::windows_api::event_window::subscribe_to_background_window(move |m, _, _| {
        if m != msg || !NATIVE_TASKBAR_HIDDEN.load(Ordering::Acquire) {
            return Ok(());
        }
        // our own tray refresh broadcasts the same message, only act on a new taskbar
        let current = primary_tray_hwnd();
        if current == 0 || current == LAST_TRAY_HWND.load(Ordering::Acquire) {
            return Ok(());
        }
        log::info!("Taskbar re-created (explorer restart), hiding it again");
        hide_native_taskbar();
        crate::windows_api::reregister_all_bars();
        Ok(())
    });
}

pub fn hide_native_taskbar() {
    LAST_TRAY_HWND.store(primary_tray_hwnd(), Ordering::Release);
    // the state is only read on the first hide, later calls (taskbar re-created / shown) would
    // read our own AutoHide
    if !NATIVE_TASKBAR_HIDDEN.swap(true, Ordering::AcqRel) {
        arm_watchdog();
        init_explorer_restart_handler();
    }
    std::thread::spawn(|| match get_taskbars_handles() {
        Ok(handles) => {
            let mut attempts = 0;
            while attempts < 10 && NATIVE_TASKBAR_HIDDEN.load(Ordering::Acquire) {
                for hwnd in &handles {
                    AppBarData::from_handle(*hwnd).set_state(AppBarDataState::AutoHide);
                    let _ = WindowsApi::show_window_async(*hwnd, SW_HIDE);
                }
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        Err(err) => log::error!("Failed to get taskbars handles: {err:?}"),
    });
}

/// Safe to call from the panic hook / shutdown paths: no-op if we never hid the taskbar.
pub fn restore_native_taskbar() -> Result<()> {
    if NATIVE_TASKBAR_HIDDEN
        .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }

    // clean exit: the watchdog sees no state file and does nothing
    let _ = std::fs::remove_file(state_file());
    show_taskbars_with_state(ORIGINAL_STATE.load(Ordering::Acquire))
}

/// Extracts the list of apps currently pinned to the native Windows taskbar.
///
/// It first tries to read the pin order from the registry (`Taskband\FavoritesResolve`),
/// falling back to scanning the "Quick Launch\User Pinned\TaskBar" folder (sorted by
/// creation time) when the registry value is missing, empty or unreadable.
pub fn get_windows_taskbar_pinned_apps() -> Result<Vec<WegItemData>> {
    let mut pinned_apps = match get_taskbar_order_from_registry() {
        Ok(ordered_paths) if !ordered_paths.is_empty() => ordered_paths
            .into_iter()
            .map(PathBuf::from)
            .filter(|lnk_path| lnk_path.exists())
            .filter_map(|lnk_path| create_weg_item_from_shortcut(&lnk_path).ok())
            .collect(),
        _ => fallback_get_taskbar_items()?,
    };

    // Deduplicate by path (case-insensitive)
    let mut seen_paths = HashSet::new();
    pinned_apps.retain(|item: &WegItemData| {
        let path_lower = item.path.to_string_lossy().to_lowercase();
        seen_paths.insert(path_lower)
    });

    Ok(pinned_apps)
}

/// Extract taskbar pinned items order from the Windows registry.
///
/// Windows stores the pinned taskbar order as a binary blob (`FavoritesResolve`) under
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Taskband`. That blob embeds the
/// `.lnk` shortcut paths as UTF-16-ish ASCII runs, so we scan the raw bytes for anything that
/// looks like a path under the user's profile.
fn get_taskbar_order_from_registry() -> Result<Vec<String>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let taskband_key =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Taskband")?;

    let favorites_resolve = taskband_key.get_raw_value("FavoritesResolve")?.bytes;
    let result: String = favorites_resolve.iter().map(|&byte| byte as char).collect();

    let userprofile =
        std::env::var("USERPROFILE").map_err(|e| crate::error::AppError::from(e.to_string()))?;
    let pattern_str = format!(r"{}.+?\.\w{{2,4}}", regex::escape(&userprofile));
    let pattern =
        regex::Regex::new(&pattern_str).map_err(|e| crate::error::AppError::from(e.to_string()))?;

    let paths = pattern
        .find_iter(&result)
        .map(|m| m.as_str().to_string())
        .collect();

    Ok(paths)
}

/// Fallback: scan the Quick Launch pinned taskbar folder and sort by creation time.
fn fallback_get_taskbar_items() -> Result<Vec<WegItemData>> {
    let mut pinned_apps = Vec::new();

    if let Ok(app_data) = std::env::var("APPDATA") {
        let user_pinned_path = PathBuf::from(app_data)
            .join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");

        if let Ok(entries) = std::fs::read_dir(&user_pinned_path) {
            let mut items_with_time: Vec<(WegItemData, std::time::SystemTime)> = Vec::new();

            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.extension().is_some_and(|ext| ext == "lnk")
                    && let Ok(item_data) = create_weg_item_from_shortcut(&entry_path)
                {
                    let created = std::fs::metadata(&entry_path)
                        .and_then(|m| m.created())
                        .unwrap_or_else(|_| std::time::SystemTime::now());
                    items_with_time.push((item_data, created));
                }
            }

            items_with_time.sort_by_key(|(_, time)| *time);
            pinned_apps = items_with_time.into_iter().map(|(item, _)| item).collect();
        }
    }

    Ok(pinned_apps)
}

/// Creates a `WegItemData` from a `.lnk` shortcut file.
fn create_weg_item_from_shortcut(lnk_path: &Path) -> Result<WegItemData> {
    let (target_path, arguments, working_dir) = WindowsApi::resolve_lnk_target(lnk_path)?;

    let display_name = lnk_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let umid = WindowsApi::get_file_umid(lnk_path).ok();

    let args = if arguments.is_empty() {
        None
    } else {
        Some(RelaunchArguments::String(
            arguments.to_string_lossy().to_string(),
        ))
    };

    let custom_icon_path = WindowsApi::resolve_lnk_custom_icon_path(lnk_path)
        .ok()
        .map(|(icon_path, _)| icon_path);

    let command = if !target_path.as_os_str().is_empty() {
        target_path.to_string_lossy().to_string()
    } else if let Some(ref icon_path) = custom_icon_path {
        icon_path.to_string_lossy().to_string()
    } else {
        String::new()
    };

    let relaunch = if !command.is_empty() {
        let working_dir = if !working_dir.as_os_str().is_empty() {
            Some(working_dir)
        } else {
            target_path.parent().map(|p| p.to_path_buf())
        };

        Some(seelen_core::system_state::Relaunch {
            command,
            args,
            working_dir,
            icon: None,
        })
    } else {
        None
    };

    Ok(WegItemData {
        id: uuid::Uuid::new_v4(),
        display_name,
        umid,
        path: lnk_path.to_path_buf(),
        pinned: true,
        prevent_pinning: false,
        relaunch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_weg_item_from_shortcut_handles_invalid_path() {
        let invalid_path = PathBuf::from("C:\\NonExistent\\Path\\file.lnk");
        let result = create_weg_item_from_shortcut(&invalid_path);
        assert!(result.is_err(), "Should fail for non-existent path");
    }

    #[test]
    fn test_deduplication_case_insensitive() {
        let mut items = vec![
            WegItemData {
                id: uuid::Uuid::new_v4(),
                display_name: "Item1".to_string(),
                umid: None,
                path: PathBuf::from("C:\\Test\\App.exe"),
                pinned: true,
                prevent_pinning: false,
                relaunch: None,
            },
            WegItemData {
                id: uuid::Uuid::new_v4(),
                display_name: "Item2".to_string(),
                umid: None,
                path: PathBuf::from("C:\\TEST\\APP.EXE"), // Same path, different case
                pinned: true,
                prevent_pinning: false,
                relaunch: None,
            },
            WegItemData {
                id: uuid::Uuid::new_v4(),
                display_name: "Item3".to_string(),
                umid: None,
                path: PathBuf::from("C:\\Other\\App.exe"),
                pinned: true,
                prevent_pinning: false,
                relaunch: None,
            },
        ];

        let mut seen_paths = HashSet::new();
        items.retain(|item| {
            let path_lower = item.path.to_string_lossy().to_lowercase();
            seen_paths.insert(path_lower)
        });

        assert_eq!(items.len(), 2, "Should deduplicate case-insensitive paths");
    }

    #[test]
    fn test_fallback_get_taskbar_items_handles_missing_directory() {
        unsafe { std::env::set_var("APPDATA", "C:\\NonExistent\\AppData") };
        let result = fallback_get_taskbar_items();

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_regex_pattern_handles_various_extensions() {
        let userprofile = "C:\\Users\\TestUser";
        let pattern_str = format!(r"{}.+?\.\w{{2,4}}", regex::escape(userprofile));
        let pattern = regex::Regex::new(&pattern_str).expect("Pattern should be valid");

        let test_cases = vec![
            (format!("{}\\path\\file.lnk", userprofile), true),
            (format!("{}\\path\\file.exe", userprofile), true),
            (format!("{}\\path\\file.msix", userprofile), true),
            (format!("{}\\path\\file.url", userprofile), true),
            ("C:\\OtherUser\\file.lnk".to_string(), false),
        ];

        for (path, should_match) in test_cases {
            let matches = pattern.is_match(&path);
            assert_eq!(
                matches, should_match,
                "Pattern matching failed for: {}. Expected: {}, Got: {}",
                path, should_match, matches
            );
        }
    }

    #[test]
    fn test_get_windows_taskbar_pinned_apps_returns_valid_structure() {
        let result = get_windows_taskbar_pinned_apps();

        match result {
            Ok(items) => {
                for item in items {
                    assert!(
                        !item.display_name.is_empty(),
                        "Display name should not be empty"
                    );
                    assert!(item.id != uuid::Uuid::nil(), "ID should not be nil");
                }
            }
            Err(e) => {
                eprintln!(
                    "Note: get_windows_taskbar_pinned_apps failed (acceptable in test): {}",
                    e
                );
            }
        }
    }
}
