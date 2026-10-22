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

## Configuration

There is no settings app. Everything that used to open it (dock menu entry, second launch of the exe, `slu settings`) opens the
config folder in Explorer. Edit the files by hand while the app is closed (it rewrites them), then start it again.

Config folder: `%APPDATA%\com.seelen.seelen-ui\`

| File | Content |
| --- | --- |
| `settings.json` | global settings; dock settings live under `byWidget` -> `"@seelen/weg"` (`position`, `size`, `hideMode`, `mode`, `enabled`, ...) |
| `data\seelen-weg\state.yml` | dock items: `left`, `center`, `right` lists (pinned apps, separators, plugin buttons) |
| `themes\`, `plugins\`, `widgets\`, `iconpacks\` | user resources, loaded from disk |

Both files are created on first run. If `state.yml` does not exist yet, the dock imports the pins of the Windows taskbar.
Missing settings fall back to defaults, so `settings.json` only needs the keys you change:

```json
{
  "byWidget": {
    "@seelen/weg": { "position": "Bottom", "size": 40, "hideMode": "OnOverlap" }
  }
}
```

`position`: `Left | Right | Top | Bottom`. `hideMode`: `Never | Always | OnOverlap`. `mode`: `FullWidth | MinContent`.

### Pinned dock apps by hand

`data\seelen-weg\state.yml` (item `type` is one of `AppOrFile`, `Separator`, `Media`, `Plugin`; `AppOrFile` may omit `id`, the other
types require an `id`; the nil uuid `00000000-0000-0000-0000-000000000000` is fine, a fresh one is generated on load):

```yaml
isReorderDisabled: false
left:
  - type: Plugin
    id: 00000000-0000-0000-0000-000000000000
    plugin: "@default/weg-start-menu"
center:
  - type: AppOrFile            # a classic exe: path is enough
    displayName: File Explorer
    path: C:\Windows\explorer.exe
    pinned: true
  - type: AppOrFile            # a Store/UWP app: umid (path is resolved from it)
    displayName: Calculator
    umid: Microsoft.WindowsCalculator_8wekyb3d8bbwe!App
    path: ""
    pinned: true
  - type: AppOrFile            # custom launch command
    displayName: Terminal
    path: C:\Program Files\WindowsApps\Microsoft.WindowsTerminal\wt.exe
    pinned: true
    relaunch:
      command: wt.exe
      args: ["-d", "C:\\Projects"]
      workingDir: C:\Projects
  - type: Separator
    id: 00000000-0000-0000-0000-000000000000
right:
  - type: Plugin
    id: 00000000-0000-0000-0000-000000000000
    plugin: "@default/weg-trash-bin"
  - type: Media
    id: 00000000-0000-0000-0000-000000000000
```

Notes: an `AppOrFile` without `umid` (or whose path is a `.lnk`) is dropped on load if the path does not exist. Fields: `displayName`,
`umid`, `path`, `pinned`, `preventPinning`, `relaunch{command,args,workingDir,icon}` (see `WegItemData` in `libs/core/src/state/weg_items.rs`).

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
- Icon editor: `src/ui/svelte/icon-editor`, `src/static/widgets/icon-editor`, the "edit icon" entries in the dock and apps-menu context menus,
  backend `resources/user_icon_pack.rs` and commands `register_user_custom_app_icon`, `delete_user_custom_app_icon`,
  `state_add_icon_to_custom_icon_pack` (a `todo!()` stub). Installed icon packs still load; only editing is gone.
- Telemetry: `src/background/telemetry.rs` (anonymous usage ping to telemetry.seelen.io + install id in registry) and its call in `main.rs`.
  Reason: no network phone-home in the fork.
- Clipboard history: `src/background/modules/clipboard`, `libs/core/src/system_state/clipboard.rs` (ClipboardData types), 7 `clipboard_*` commands,
  `clipboard::data-changed` event, the `WM_CLIPBOARDUPDATE` listener in `windows_api/event_window.rs`, WinRT helpers
  (`join_pumped`, webp/base64 stream helpers), `clipboard_history.bin` path constant. The old history file in the data dir is left untouched.
- Marketplace / resource downloads: `src/background/cli/uri` (the `seelen-ui.uri:` deep link handler, `.slu` file open handler, remote
  resource and icon/wallpaper asset downloader), `AppMessage::OpenUri` in `libs/slu-ipc`, the positional `uri` arg of the main CLI, the
  `deep-link` plugin config and `.slu` file association in `src/tauri.conf.json`. Local bundled and user resources on disk still load;
  `slu resource load/unload` stays. (`src/templates/AppxManifest.xml` still declares the protocol; msix is not built by the fork.)
- Updater: `src/background/utils/updater.rs`, `tauri-plugin-updater` (dependency + plugin registration + `Updater` error variant), commands
  `check_for_updates` / `install_last_available_update` (+ bindings), `createUpdaterArtifacts` and the `plugins.updater` pubkey in
  `tauri.conf.json`, the settings header `UpdateButton`. `UpdaterSettings`/`UpdateChannel` types stay in `libs/core` (data only). `reqwest`
  now enables `json` explicitly (it was only on via the updater plugin).
- Seelen account session + cloud backups: `src/background/session` (browser login, PasswordVault tokens, JWT refresh), `src/background/backups`,
  commands `get_seelen_session` / `seelen_login` / `seelen_logout` / `get_backup_status`, events `session::changed` / `backup::status-changed`,
  `SeelenSession` / `BackupStatus` types, session UI in the settings app. Premium-resource filtering in `resources/emitters.rs` is gone (no
  account, so every resource on disk is listed). `backup_sync_enabled` stays in settings types (data only).
- Discord rich presence: `src/background/utils/discord.rs`, `discord-rich-presence` dependency, its start/update calls in `app.rs`, the toggle in
  the settings app. `drpc` stays in settings types (data only).
- Release signing + bundle integrity: `src/build.rs` no longer writes/signs `SHA256SUMS` (no `TAURI_SIGNING_PRIVATE_KEY*` env needed, release builds work
  with no env vars); `utils/integrity/checksums.rs` (minisign signature check against upstream's key + bundle checksum check at startup) is gone, the
  WebView2 runtime checks stay. Also removed `libs/utils` `checksums`/`signature` modules, `minisign`/`sha2` deps, and the SHA256SUMS lines in
  `installer-hooks.nsh` and `scripts/bundle.msix.ts`.
- Settings app: `src/ui/react/settings`, `src/static/widgets/settings`. `show_settings` / `show_settings_at` became `open_config_folder` (Explorer on
  `%APPDATA%\com.seelen.seelen-ui`). Triggering the `@seelen/settings` widget (dock menu, start menu footer, toolbar, quick settings) is intercepted
  in `trigger_widget_inner` and opens the folder too, so those frontends need no change. `slu settings` and the second-launch path
  (`is_already_running`) go through the same function. `set_widget_permissions` is now uncallable (it only accepted the settings app);
  edit permissions in the data dir by hand. See "Configuration".
- React dialog widget: `src/ui/react/dialog`, `src/static/widgets/dialog`, `trigger_dialog` command, `trigger_dialog_backend`, `Dialog`/`DialogContent` types,
  the shortcut-conflict popup (now a `log::warn!`), the shortcut-recorder popup (`widgets/popups`, `request_to_user_input_shortcut` command; it only
  served the settings app; the service-side registration and `slu popup` IPC variants stay, unused). A corrupted `settings.json` now shows a native
  `tauri_plugin_dialog` message box. No React app remains, so `scripts/build/builders/react.ts` and the react entry discovery are gone
  (`libs/ui/react` components and the preact/react npm deps stay until the unused-deps step).
