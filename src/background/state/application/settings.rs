use std::path::Path;

use seelen_core::{handlers::SeelenEvent, state::Settings};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::{
    app::{SeelenUI, emit_to_webviews, get_app_handle},
    error::{Result, ResultLogExt},
    resources::ResourceManager,
    utils::constants::SEELEN_COMMON,
    widgets::manager::WIDGET_MANAGER,
};

use super::AppSettings;

impl AppSettings {
    pub(super) fn emit_settings(&self) -> Result<()> {
        emit_to_webviews(SeelenEvent::StateSettingsChanged, &self.settings);
        SeelenUI::on_settings_change(self)?;
        WIDGET_MANAGER.reconcile().log_error();
        Ok(())
    }

    pub(super) fn read_settings() -> Settings {
        let path = SEELEN_COMMON.settings_path();
        if path.exists() {
            match Settings::load(path) {
                Ok(settings) => return settings,
                Err(err) => {
                    log::error!("Failed to read settings: {err}");
                    show_corrupted_state_to_user(SEELEN_COMMON.settings_path());
                }
            }
        }
        Settings::default()
    }

    /// Resources id changed for remote/downloaded resources.
    pub(super) fn migration_v2_5_0(&mut self, resources: &ResourceManager) -> Result<()> {
        resources.themes.iter_sync(|new_id, theme| {
            let Some(remote) = &theme.metadata.internal.remote else {
                return true;
            };

            let Some(old_id) = remote.deprecated_id.clone() else {
                return true;
            };
            let old_id = old_id.into();

            if let Some(config) = self.settings.by_theme.remove(&old_id) {
                self.settings.by_theme.insert(new_id.clone(), config);
            };

            for id in &mut self.settings.active_themes {
                if id == &old_id {
                    *id = new_id.clone();
                }
            }
            true
        });

        resources.wallpapers.iter_sync(|new_id, wallpaper| {
            let Some(remote) = &wallpaper.metadata.internal.remote else {
                return true;
            };

            let Some(old_id) = remote.deprecated_id.clone() else {
                return true;
            };
            let old_id = old_id.into();

            if let Some(config) = self.settings.by_wallpaper.remove(&old_id) {
                self.settings.by_wallpaper.insert(new_id.clone(), config);
            };

            for collection in &mut self.settings.wallpaper_collections {
                for id in &mut collection.wallpapers {
                    if id == &old_id {
                        *id = new_id.clone();
                    }
                }
            }
            true
        });

        resources.widgets.iter_sync(|k, v| {
            let Some(remote) = &v.metadata.internal.remote else {
                return true;
            };

            let Some(old_id) = remote.deprecated_id.clone() else {
                return true;
            };
            let old_id = old_id.into();

            if let Some(config) = self.settings.by_widget.others.remove(&old_id) {
                self.settings.by_widget.others.insert(k.clone(), config);
            };

            self.settings.monitors_v3.values_mut().for_each(|monitor| {
                if let Some(config) = monitor.by_widget.remove(&old_id) {
                    monitor.by_widget.insert(k.clone(), config);
                };
            });
            true
        });

        Ok(())
    }

    /// Sanitize wallpaper collections to remove non-existent wallpaper IDs
    pub(super) fn sanitize_wallpaper_collections(&mut self, resources: &ResourceManager) -> bool {
        let mut changed = false;
        for collection in &mut self.settings.wallpaper_collections {
            let original_len = collection.wallpapers.len();
            collection
                .wallpapers
                .retain(|wallpaper_id| resources.wallpapers.contains_sync(wallpaper_id));
            if collection.wallpapers.len() != original_len {
                changed = true;
            }
        }
        changed
    }

    pub fn write_settings(&self) -> Result<()> {
        self.settings.save(SEELEN_COMMON.settings_path())?;
        self.emit_settings()?;
        Ok(())
    }
}

fn show_corrupted_state_to_user(path: &Path) {
    let body = format!(
        "{}\n{}: {:?}",
        t!("runtime.corrupted_file"),
        t!("runtime.corrupted_file_path"),
        path
    );
    std::thread::spawn(move || {
        get_app_handle()
            .dialog()
            .message(body)
            .title(t!("runtime.corrupted_data"))
            .kind(MessageDialogKind::Error)
            .blocking_show();
    });
}
