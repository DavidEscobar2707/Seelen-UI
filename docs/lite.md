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
