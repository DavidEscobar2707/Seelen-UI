//! Event-driven `GlobalMouseMove`.
//!
//! A `WH_MOUSE_LL` hook lives on a dedicated thread with its own message loop, and only exists
//! while a webview is subscribed to the event (the control thread blocks on the subscribers
//! change condvar, so it costs nothing otherwise). Nothing runs while the mouse is still. The hook
//! callback only stores the position and arms a one shot timer, the emit happens from the
//! timer on the same thread, so the callback stays far below the low level hook timeout and
//! moves are coalesced to at most 10 events per second.

use std::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};

use seelen_core::handlers::SeelenEvent;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, HHOOK, KillTimer, MSG, MSLLHOOKSTRUCT,
        PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SetTimer, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL, WM_APP, WM_MOUSEMOVE,
    },
};

use crate::{
    app::emit_to_webviews, server::events::EVENTS_HUB, utils::spawn_named_thread,
    windows_api::WindowsApi,
};

const WM_SET_HOOK_ENABLED: u32 = WM_APP + 1;
const MIN_EMIT_INTERVAL: Duration = Duration::from_millis(100);

static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy)]
struct MouseState {
    pos: (i32, i32),
    last_emitted: Option<(i32, i32)>,
    last_emit: Option<Instant>,
    timer_pending: bool,
}

thread_local! {
    // the hook callback, the timer and the message loop all run on the hook thread
    static STATE: Cell<MouseState> = const { Cell::new(MouseState {
        pos: (0, 0),
        last_emitted: None,
        last_emit: None,
        timer_pending: false,
    }) };
}

/// Pure coalescing rule: how long to wait before emitting the next move.
pub fn emit_delay(last_emit: Option<Instant>, now: Instant) -> Duration {
    match last_emit {
        Some(last) => MIN_EMIT_INTERVAL.saturating_sub(now.saturating_duration_since(last)),
        None => Duration::ZERO,
    }
}

unsafe extern "system" fn timer_proc(_hwnd: HWND, _msg: u32, id: usize, _time: u32) {
    unsafe {
        let _ = KillTimer(None, id);
    }
    let mut state = STATE.get();
    state.timer_pending = false;
    state.last_emit = Some(Instant::now());
    if state.last_emitted != Some(state.pos) && !crate::utils::idle::is_paused() {
        state.last_emitted = Some(state.pos);
        STATE.set(state);
        emit_to_webviews(SeelenEvent::GlobalMouseMove, [state.pos.0, state.pos.1]);
    } else {
        STATE.set(state);
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_MOUSEMOVE {
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let mut state = STATE.get();
        state.pos = (info.pt.x, info.pt.y);
        if !state.timer_pending {
            let delay = emit_delay(state.last_emit, Instant::now());
            let ms = (delay.as_millis() as u32).max(10); // USER_TIMER_MINIMUM
            if unsafe { SetTimer(None, 0, ms, Some(timer_proc)) } != 0 {
                state.timer_pending = true;
            }
        }
        STATE.set(state);
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn hook_thread() {
    let mut hook: Option<HHOOK> = None;
    let mut msg = MSG::default();
    unsafe {
        // creates the thread message queue before announcing the thread id
        let _ = PeekMessageW(&mut msg, None, WM_APP, WM_APP, PM_NOREMOVE);
        HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::Release);

        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.hwnd.0.is_null() && msg.message == WM_SET_HOOK_ENABLED {
                let enable = msg.wParam.0 != 0;
                if enable && hook.is_none() {
                    let module = WindowsApi::module_handle_w().ok();
                    match SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(mouse_proc),
                        module.map(Into::into),
                        0,
                    ) {
                        Ok(h) => hook = Some(h),
                        Err(err) => log::error!("Failed to install the mouse hook: {err}"),
                    }
                } else if !enable && let Some(h) = hook.take() {
                    let _ = UnhookWindowsHookEx(h);
                }
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Starts the mouse event source, call once.
pub fn init() {
    spawn_named_thread("MouseHookLoop", hook_thread);
    spawn_named_thread("MouseHookControl", || {
        let event = SeelenEvent::GlobalMouseMove.as_ref();
        let mut enabled = false;
        loop {
            let wanted = EVENTS_HUB.has_listeners(event);
            let thread_id = HOOK_THREAD_ID.load(Ordering::Acquire);
            if wanted != enabled && thread_id != 0 {
                let posted = unsafe {
                    PostThreadMessageW(
                        thread_id,
                        WM_SET_HOOK_ENABLED,
                        WPARAM(wanted as usize),
                        LPARAM(0),
                    )
                };
                if posted.is_ok() {
                    enabled = wanted;
                }
            }
            // woken by subscribe/unsubscribe, the timeout only covers the thread start race
            let timeout = if wanted != enabled {
                Duration::from_millis(200)
            } else {
                Duration::from_secs(3600)
            };
            EVENTS_HUB.wait_for_listeners_change(timeout);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_move_emits_immediately() {
        assert_eq!(emit_delay(None, Instant::now()), Duration::ZERO);
    }

    #[test]
    fn moves_are_coalesced_to_10hz() {
        let now = Instant::now();
        let last = now - Duration::from_millis(30);
        assert_eq!(emit_delay(Some(last), now), Duration::from_millis(70));
        let old = now - Duration::from_millis(500);
        assert_eq!(emit_delay(Some(old), now), Duration::ZERO);
    }
}
