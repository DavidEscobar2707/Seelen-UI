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
- Toolbar (top bar) (restored in phase 2): `src/ui/svelte/fancy-toolbar`, `src/static/widgets/toolbar`, `src/background/widgets/toolbar` (stub: height helper + a
  commented hook), all 11 `tb_*` plugins in `src/static/plugins`, the theme style `default/styles/fancy-toolbar.scss`, its i18n completion line in
  `scripts/translate/mod.ts`. The dock (`weg/state/settings.svelte.ts`) no longer reserves space for the bar (`workArea` = monitor rect); the revert of this
  commit puts the offset back. Kept: `fancy_toolbar` settings types + `ToolbarState`/`toolbar_items.rs` + `state_get/write_toolbar_items` commands
  (data only, lazy), so existing `settings.json` files still deserialize. `src/static/themes/bubbles` still lists a toolbar style (theme removed in step 10).
- Toolbar popups (restored in phase 2): UI `src/ui/svelte/` + bundle `src/static/widgets/` for bluetooth-popup, calendar-popup, keyboard-selector, media-popup,
  network-popup, notifications, power-menu, quick-settings, system-tray (each bundle also carried its `toolbar-plugin.yml` and opt-in `dock-plugin.yml`/dock
  plugin js, so the optional dock plugins `@seelen/dock-*` are gone too), their 9 default-theme styles, translate-script lines. Backend modules untouched.
  The dock itself opens none of them by default (the dock Media item drives playback directly), so none was kept. Left in place on purpose:
  `src/ui/svelte/notifications/components/Notification.svelte` (imported by flyouts, deleted together with flyouts in the unwanted-widgets step).
  Known dangling triggers until that step: apps-menu footer asks for `@seelen/power-menu` (logged "Widget not found").
- Unwanted widgets/plugins: `apps-menu` (UI + bundle + theme style; the dock start button now always sends the Win key, `show_start_menu` no longer
  looks for the widget), `flyouts` (volume/brightness OSD), `user-menu` (`user` bundle), plugins `weg_default_power` and `weg_show_desktop`, theme
  `src/static/themes/bubbles`, leftover `Notification.svelte`, translate-script lines. The dock default items drop the show-desktop button; an existing
  `state.yml` that still lists a removed plugin just renders nothing for that slot. `get_start_menu_items`/`get_native_start_menu` commands stay (backend).
- Unused dependencies: Rust `url` and `open` (src), `paste` and `slu-macros` (libs/core); `tao` stays (it is a `[patch.crates-io]` git source, not a dependency).
  `reqwest` stays: `slu resource translate` (dev CLI) uses it. npm: dead React helper code `libs/ui/react/{components,utils/*}` (only `utils/index.ts` and
  `utils/layered.ts` are used by the dock) was deleted, then removed from package.json/package-lock: `@dnd-kit/react`, `@preact/compat`, `@preact/signals`, `preact`,
  `preact-render-to-string`, `react-router`, `react-i18next`, `i18next`, `antd`, `framer-motion`, `qrcode`, `@types/qrcode`, `flexsearch`, `unified`, `remark-gfm`,
  `remark-parse`, `remark-rehype`, `rehype-stringify`, `esbuild-css-modules-plugin`, `@types/react-dom`. Done with `npm uninstall --package-lock-only` so the shared
  `node_modules` junction (main checkout, baseline build) is not modified; run `npm install`/`npm ci` here to prune it. `react-icons` + `real-react-dom` stay (icon
  extraction step). `tsconfig.json` drops the preact JSX/paths config. `windows` crate features were not touched.
- Elevated service (`slu-service`): `src/service` (hotkeys, Task Scheduler task, elevated window helpers), `ServicePipe` + `start_service`, `libs/slu-ipc`
  `service.rs`/`SvcAction`, global shortcuts handling (`resolve_shortcuts` calls, `cli/shortcuts.rs`, `ToggleShortcutsPause` instance command), the
  `positioning` dependency of `src`, the `service.*` i18n strings. No UAC prompt at startup. The app now hides/restores `Shell_TrayWnd` itself
  (`widgets/weg/native_taskbar.rs`); restore runs on exit, panic hook, `WM_ENDSESSION` and when the dock is disabled. A hard kill (taskkill /F, crash
  without unwinding) leaves the taskbar hidden until explorer restarts or the app runs again with the dock disabled. Windows of elevated processes
  can no longer be shown/moved/focused (calls just fail and log). Autostart is `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value `SeelenLite`
  (`set_auto_start`/`get_auto_start_status`); the old `\Seelen\Seelen UI Service` task is never read or touched. NSIS/AppX templates and
  `scripts/bundle.msix.ts` still mention `slu-service.exe` (bundling is not used by the fork; fix before building an installer).
- Single event transport: `emit_to_webviews` only publishes on the events websocket hub (the tauri `emit_str` broadcast is gone). `__unsafe_tauri_subscribe`
  removed from `libs/core`; `newOnEvent` (State.ts), `layered.ts` (GlobalMouseMove) and the `MainSetup.ts` widget-reload listener now use `subscribe`.
  Per-webview tauri events stay (`WidgetTriggered`, liveness ping, context-menu callbacks). The salvo `oapi` feature stays: `seelen-core` types derive
  `ToSchema` under the `salvo` feature and the `/ping`/themes endpoints use it; dropping it is a separate refactor (the Scalar page is served from
  an embedded 1-file html, no extra dependency).
- Window pixel capture: `modules/apps/application/previews.rs` (`WinPreviewManager`, 200 ms `capture_window` + edge-color sampling), command
  `get_user_app_windows_colors`, event `user::windows-colors-changed`, `UserAppWindowColors` type, the `win-screenshot` dependency and the
  dock `--window-gradient` / `data-there-is-maximized-on-bg` tinting (the dock now always uses theme colors).
