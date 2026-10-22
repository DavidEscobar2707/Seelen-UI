pub mod cli;
pub mod handler;
pub mod hook;
pub mod native_taskbar;

use crate::{error::Result, state::application::FULL_STATE, windows_api::monitor::Monitor};

pub static TASKBAR_CLASS: [&str; 2] = ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

pub struct SeelenWeg {}

impl SeelenWeg {
    #[allow(dead_code)]
    pub fn get_weg_size_on_monitor(monitor: &Monitor) -> Result<i32> {
        let state = FULL_STATE.load();
        let settings: &seelen_core::state::SeelenWegSettings = &state.settings.by_widget.weg;
        let total_size = (settings.total_size() as f64 * monitor.scale_factor()?) as i32;
        Ok(total_size)
    }

    // The native taskbar is hidden/restored by this process (see native_taskbar.rs).
    // It is restored on exit, panic, WM_ENDSESSION and when the dock is disabled.

    pub fn hide_native_taskbar() {
        native_taskbar::hide_native_taskbar();
    }

    pub fn restore_native_taskbar() -> Result<()> {
        native_taskbar::restore_native_taskbar()
    }
}
