use seelen_core::system_state::AppBarEdge;
use std::sync::LazyLock;
use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    UI::Shell::{
        ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_NEW, ABM_REMOVE, ABM_SETPOS,
        ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage,
    },
};

use crate::{error::Result, utils::lock_free::TracedMutex};

/// Bars registered on the shell, with the last data sent, so they can be registered again when
/// explorer.exe restarts (the shell forgets every ABM_NEW registration).
static REGISTERED_BARS: LazyLock<TracedMutex<Vec<(isize, RegisteredBar)>>> =
    LazyLock::new(|| TracedMutex::new(Vec::new()));

#[derive(Debug, Clone, Copy)]
struct RegisteredBar {
    rect: RECT,
    edge: u32,
}

/// Registers again every bar after the shell was restarted.
pub fn reregister_all_bars() {
    let guard = REGISTERED_BARS.lock();
    for (addr, bar) in guard.iter() {
        let mut data = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: HWND(*addr as *mut _),
            uEdge: bar.edge,
            rc: bar.rect,
            ..Default::default()
        };
        unsafe {
            SHAppBarMessage(ABM_REMOVE, &mut data);
            if SHAppBarMessage(ABM_NEW, &mut data) != 0 {
                SHAppBarMessage(ABM_SETPOS, &mut data);
            }
        }
    }
}

/// https://learn.microsoft.com/en-us/windows/win32/shell/abm-setstate#parameters
#[derive(Debug, Clone, Copy)]
pub enum AppBarDataState {
    BothOff = 0,
    AutoHide = ABS_AUTOHIDE as isize,
    AlwaysOnTop = ABS_ALWAYSONTOP as isize,
    BothOn = 3,
}

impl From<AppBarDataState> for LPARAM {
    fn from(val: AppBarDataState) -> Self {
        LPARAM(val as isize)
    }
}

impl From<u32> for AppBarDataState {
    fn from(state: u32) -> Self {
        match state {
            0 => AppBarDataState::BothOff,
            ABS_AUTOHIDE => AppBarDataState::AutoHide,
            ABS_ALWAYSONTOP => AppBarDataState::AlwaysOnTop,
            3 => AppBarDataState::BothOn,
            _ => unreachable!(),
        }
    }
}

pub struct AppBarData(pub APPBARDATA);

#[allow(dead_code)]
impl AppBarData {
    pub fn from_handle(hwnd: HWND) -> Self {
        Self(APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            ..Default::default()
        })
    }

    pub fn set_state(&self, state: AppBarDataState) {
        let mut data = self.0;
        data.lParam = state.into();
        unsafe { SHAppBarMessage(ABM_SETSTATE, &mut data) };
    }

    /// Current AppBar state flags of the shell taskbar (0 normal, 1 autohide, 2 always on top, 3 both).
    pub fn get_state(&self) -> u32 {
        let mut data = self.0;
        (unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) } as u32) & 3
    }

    pub fn set_edge(&mut self, edge: AppBarEdge) {
        self.0.uEdge = match edge {
            AppBarEdge::Left => ABE_LEFT,
            AppBarEdge::Top => ABE_TOP,
            AppBarEdge::Right => ABE_RIGHT,
            AppBarEdge::Bottom => ABE_BOTTOM,
        };
    }

    pub fn set_rect(&mut self, rect: RECT) {
        self.0.rc = rect;
    }

    pub fn register_as_new_bar(&mut self) -> Result<()> {
        let mut data = self.0;
        let addr = data.hWnd.0 as isize;
        let mut guard = REGISTERED_BARS.lock();

        let bar = RegisteredBar {
            rect: data.rc,
            edge: data.uEdge,
        };
        if let Some(entry) = guard.iter_mut().find(|(a, _)| *a == addr) {
            entry.1 = bar;
        } else {
            let ok = unsafe { SHAppBarMessage(ABM_NEW, &mut data) };
            if ok == 0 {
                return Err("Failed to register App Bar".into());
            }
            guard.push((addr, bar));
        }

        unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) };
        Ok(())
    }

    pub fn unregister_bar(&mut self) -> Result<()> {
        let mut data = self.0;
        let addr = data.hWnd.0 as isize;
        let mut guard = REGISTERED_BARS.lock();
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) };
        guard.retain(|(x, _)| *x != addr);
        Ok(())
    }
}
