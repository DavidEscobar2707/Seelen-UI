//! One shared "pause" flag for every periodic loop.
//!
//! Loops that run at idle must check [`is_paused`] and skip their work while it is true:
//! - the session is not interactive (locked / switched user),
//! - the display is off (`GUID_CONSOLE_DISPLAY_STATE`, registered on the background window),
//! - a fullscreen app is focused (updated from the foreground / rect win events),
//! - game mode suspended the widgets (`GAME_MODE_ACTIVE`).

use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    hook::HookManager,
    widgets::manager::GAME_MODE_ACTIVE,
    windows_api::{
        event_window::IS_INTERACTIVE_SESSION,
        window::{Window, event::WinEvent},
    },
};

static DISPLAY_OFF: AtomicBool = AtomicBool::new(false);
static FULLSCREEN_FOCUSED: AtomicBool = AtomicBool::new(false);

/// true when background polling/work should be skipped
pub fn is_paused() -> bool {
    !IS_INTERACTIVE_SESSION.load(Ordering::Acquire)
        || DISPLAY_OFF.load(Ordering::Acquire)
        || FULLSCREEN_FOCUSED.load(Ordering::Acquire)
        || GAME_MODE_ACTIVE.load(Ordering::Acquire)
}

pub fn set_display_off(off: bool) {
    if DISPLAY_OFF.swap(off, Ordering::AcqRel) != off {
        log::info!("Display {}", if off { "off" } else { "on" });
    }
}

/// Starts tracking the fullscreen state, call once after the win hook is registered.
pub fn init() {
    HookManager::subscribe(|(event, _origin)| {
        if matches!(
            event,
            WinEvent::SystemForeground | WinEvent::SynDebouncedRectChange
        ) {
            let fullscreen = Window::get_foregrounded().is_fullscreen();
            if FULLSCREEN_FOCUSED.swap(fullscreen, Ordering::AcqRel) != fullscreen {
                log::info!("Fullscreen app focused: {fullscreen}");
            }
        }
    });
}
