use std::sync::atomic::Ordering;

use tauri::{AppHandle, Wry};
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

use crate::{
    APP_HANDLE,
    error::{Result, ResultLogExt},
    hook::register_win_hook,
    migrations::Migrations,
    modules::{start::application::StartMenuManager, user::infrastructure::reemit_user},
    resources::RESOURCES,
    state::application::{AppSettings, FULL_STATE, initialize_user_resources_watcher},
    widgets::{manager::WIDGET_MANAGER, weg::SeelenWeg},
    windows_api::event_window::{IS_INTERACTIVE_SESSION, create_background_window},
};

const AUTOSTART_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const AUTOSTART_VALUE: &str = "SeelenLite";

/// Tauri app handle
pub fn get_app_handle<'a>() -> &'a AppHandle<Wry> {
    APP_HANDLE
        .get()
        .expect("get_app_handle called but app is still not initialized")
}

type InternalListener = std::sync::Arc<dyn Fn() + Send + Sync + 'static>;

static INTERNAL_LISTENERS: std::sync::LazyLock<
    parking_lot::RwLock<Vec<(&'static str, InternalListener)>>,
> = std::sync::LazyLock::new(Default::default);

/// In-process subscription to the events published by [`emit_to_webviews`] (the tauri
/// `listen` on the app handle no longer receives them, the websocket hub is the only transport).
/// The callback runs synchronously on the emitting thread, so it must not block.
pub fn on_internal_event(event: &'static str, cb: impl Fn() + Send + Sync + 'static) {
    INTERNAL_LISTENERS
        .write()
        .push((event, std::sync::Arc::new(cb)));
}

pub fn emit_to_webviews<S>(event: &str, payload: S)
where
    S: serde::Serialize,
{
    if !IS_INTERACTIVE_SESSION.load(Ordering::Acquire) {
        return;
    }

    let payload = match serde_json::to_string(&payload) {
        Ok(payload) => payload,
        Err(err) => {
            log::error!("Failed to serialize event {event}: {err}");
            return;
        }
    };

    // single transport: the events websocket hub (tauri events leak memory, see libs/core handlers)
    crate::server::events::EVENTS_HUB.publish(event, &payload);

    let listeners: Vec<InternalListener> = INTERNAL_LISTENERS
        .read()
        .iter()
        .filter(|(name, _)| *name == event)
        .map(|(_, cb)| cb.clone())
        .collect();
    for cb in listeners {
        cb();
    }
}

pub struct SeelenUI {}

/* ============== Methods ============== */
impl SeelenUI {
    /// this fn prepares the app loading assets and settings in memory parallelly
    pub async fn pre_start() -> Result<()> {
        Migrations::run_all()?;

        let webview_check = tokio::task::spawn(async {
            crate::measure!(
                "Webview optimal state",
                crate::utils::integrity::check_for_webview_optimal_state().await
            )
        });

        crate::measure!("RESOURCES", RESOURCES.initialize().await);
        crate::measure!("START_MENU", StartMenuManager::initialize().log_error());
        crate::measure!("APP SETTINGS", FULL_STATE.load());

        if !webview_check.await.unwrap_or(false) {
            return Err("Webview optimal state check failed".into());
        }

        FULL_STATE.rcu(|state| {
            let mut state = state.cloned();
            state.complete_initialization(&RESOURCES);
            state
        });

        Ok(())
    }

    pub async fn start() -> Result<()> {
        let state = FULL_STATE.load();
        rust_i18n::set_locale(state.locale());

        if state.is_weg_enabled() {
            SeelenWeg::hide_native_taskbar();
        }

        WIDGET_MANAGER.reconcile()?;
        create_background_window()?;
        register_win_hook()?;

        initialize_user_resources_watcher()?;

        Ok(())
    }

    pub fn on_settings_change(state: &AppSettings) -> Result<()> {
        rust_i18n::set_locale(state.locale());

        if state.is_weg_enabled() {
            SeelenWeg::hide_native_taskbar();
        } else {
            SeelenWeg::restore_native_taskbar()?;
        }

        // Re-emit user so streaming mode redaction takes effect immediately.
        reemit_user();
        Ok(())
    }

    /// Autostart lives in HKCU\...\Run (no admin, no Task Scheduler).
    pub fn is_auto_start_enabled() -> Result<bool> {
        let run = RegKey::predef(HKEY_CURRENT_USER).open_subkey(AUTOSTART_KEY)?;
        Ok(run.get_value::<String, _>(AUTOSTART_VALUE).is_ok())
    }

    pub fn set_auto_start(enabled: bool) -> Result<()> {
        let (run, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(AUTOSTART_KEY)?;
        if enabled {
            let exe = std::env::current_exe()?;
            run.set_value(AUTOSTART_VALUE, &format!("\"{}\"", exe.display()))?;
        } else if run.get_value::<String, _>(AUTOSTART_VALUE).is_ok() {
            run.delete_value(AUTOSTART_VALUE)?;
        }
        Ok(())
    }
}
