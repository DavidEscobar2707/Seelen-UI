use std::sync::Arc;

use seelen_core::{
    handlers::SeelenEvent,
    resource::ResourceKind,
    state::{Theme, Widget},
};

use crate::{
    app::emit_to_webviews,
    error::{Result, ResultLogExt},
    widgets::manager::WIDGET_MANAGER,
    widgets::permissions::WIDGET_PERMISSIONS,
};

use super::ResourceManager;

// Reusable getters for resources.
impl ResourceManager {
    pub fn widgets(&self) -> Vec<Arc<Widget>> {
        let mut widgets = Vec::new();
        self.widgets.iter_sync(|_, v| {
            widgets.push(v.clone());
            true
        });
        widgets
    }

    pub fn themes(&self) -> Vec<Arc<Theme>> {
        let mut themes = Vec::new();
        self.themes.iter_sync(|_, v| {
            themes.push(v.clone());
            true
        });
        themes
    }

    pub fn plugins(&self) -> Vec<Arc<seelen_core::state::Plugin>> {
        let mut plugins = Vec::new();
        self.plugins.iter_sync(|_, v| {
            plugins.push(v.clone());
            true
        });
        plugins
    }

    pub fn icon_packs(&self) -> Vec<Arc<seelen_core::state::IconPack>> {
        let mut icon_packs = Vec::new();
        // Add system icon pack if it exists
        if let Some(system_pack) = self.system_icon_pack.lock().as_ref() {
            icon_packs.push(std::sync::Arc::new(system_pack.clone()));
        }
        // Add user icon packs
        self.icon_packs.iter_sync(|_, v| {
            icon_packs.push(v.clone());
            true
        });
        icon_packs
    }

    pub fn wallpapers(&self) -> Vec<Arc<seelen_core::state::Wallpaper>> {
        let mut wallpapers = Vec::new();
        self.wallpapers.iter_sync(|_, v| {
            wallpapers.push(v.clone());
            true
        });
        wallpapers
    }
}

impl ResourceManager {
    pub fn emit_widgets(&self) -> Result<()> {
        let widgets = self.widgets();
        emit_to_webviews(SeelenEvent::StateWidgetsChanged, widgets.clone());
        WIDGET_PERMISSIONS.remove_uninstalled_widgets().log_error();

        WIDGET_MANAGER.reconcile()?;
        Ok(())
    }

    pub fn emit_themes(&self) {
        emit_to_webviews(SeelenEvent::StateThemesChanged, self.themes())
    }

    pub fn emit_plugins(&self) {
        emit_to_webviews(SeelenEvent::StatePluginsChanged, self.plugins())
    }

    pub fn emit_icon_packs(&self) {
        emit_to_webviews(SeelenEvent::StateIconPacksChanged, self.icon_packs())
    }

    pub fn emit_wallpapers(&self) {
        emit_to_webviews(SeelenEvent::StateWallpapersChanged, self.wallpapers())
    }

    pub fn emit_kind_changed(&self, kind: &ResourceKind) -> Result<()> {
        match kind {
            ResourceKind::Theme => self.emit_themes(),
            ResourceKind::Widget => {
                self.emit_plugins();
                self.emit_widgets()?;
            }
            ResourceKind::Plugin => self.emit_plugins(),
            ResourceKind::IconPack => self.emit_icon_packs(),
            ResourceKind::Wallpaper => self.emit_wallpapers(),
            ResourceKind::SoundPack => {
                // feature not implemented
            }
        }
        Ok(())
    }
}
