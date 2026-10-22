# Seelen Lite: fork notes

## Goal

Debloat Seelen UI to a "minimum usable" Windows 11 shell. Delete and disable; do not rebuild.

- Phase 1: dock only.
- Phase 2: top bar.

## Budget (idle, dock + bar)

- CPU < 1 %
- RAM < 150 MB total (seelen processes + their msedgewebview2 processes)
- No measurable increase in idle power

## Commit rule

One removal per commit, so each one can be reverted alone.

## Removed in phase 1

- Window manager (tiling): `src/background/widgets/window_manager`, `src/ui/svelte/window-manager`,
  `src/static/widgets/window-manager`, all `wm_*` plugins, wm CLI subcommand, wm commands/events, wm theme
  style. Twm/wm settings types in `libs/core` stay (data only). Reason: not used; dock does not need it.
- Wallpaper manager (live wallpaper widget): `src/background/widgets/wallpaper_manager`, `src/ui/svelte/wallpaper-manager`,
  `src/static/widgets/wallpaper-manager`, its theme style, `slu wallpaper` CLI, `set_as_wallpaper` command, unused
  `WindowsApi::virtual_screen_rect`. Windows wallpaper is never touched. `virtual_desktops::wallpapers` and the
  `wallpaper_next/prev` commands stay for now (still wired to virtual desktops); settings types stay (data only).
- Workspaces viewer: `src/ui/svelte/workspaces-viewer`, `src/static/widgets/workspaces-viewer` (incl. its shortcuts), theme style.
  `src/background/virtual_desktops` stays (dock and weg use it). Reason: overview panel not needed for the dock.
- Task switcher (alt-tab replacement): `src/background/widgets/task_switcher`, `src/ui/svelte/task-switcher`,
  `src/static/widgets/task-switcher` (incl. its shortcuts), theme style, `TaskSwitcher` slu-ipc CLI command. Reason: use Windows alt-tab.
- Weg preview (window thumbnails on multi-window click): `src/ui/svelte/weg-preview`, `src/static/widgets/weg-preview`, theme style,
  `weg/previewWidget.ts`, `UserApplicationPreview.svelte`, `get_user_app_windows_previews` command, `user::windows-previews-changed` event,
  `UserAppWindowPreview` type, webp/base64 thumbnail encoding. Clicking a dock item with several windows now toggles the first window.
  `modules/apps/application/previews.rs` (`WinPreviewManager`) stays but only samples edge colors (`UserAppWindowColors`, used by
  dock/toolbar tinting): follow-up candidate if the per-window capture costs idle CPU.
