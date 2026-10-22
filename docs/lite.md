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
