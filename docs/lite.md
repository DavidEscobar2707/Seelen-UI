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
- Popups destroyed on close: lazy widgets (`@seelen/context-menu`, `@seelen/tooltip`) used to hide and keep the webview for 30 s; `Widget.hide()` now
  closes it 300 ms after hiding (the delay only lets a quick re-show reuse it; `show()` cancels the close). The `WindowEvent::Destroyed` handler
  kills the pod and never respawns lazy widgets; no lazy widget is created at startup (`reconcile` only starts non-lazy deployments). A trigger on a
  destroyed lazy widget re-runs `reconcile` and starts a new webview through `PENDING_TRIGGERS`.

## Perf

Shared pause flag: `utils::idle::is_paused()` = session not interactive OR display off (`GUID_CONSOLE_DISPLAY_STATE` on the background window) OR a
fullscreen app focused (foreground/rect events) OR `GAME_MODE_ACTIVE`. Periodic loops check it.

| Loop | Before | Now |
| --- | --- | --- |
| MouseEventHook (`GlobalMouseMove`) | 10 Hz always | 10 Hz only while a webview subscribed to the event (dock click-through hitbox) and not paused; otherwise blocked on a condvar woken by the hub's subscribe |
| Zombie window killer | 1 s | 10 s, skipped while paused |
| Widget liveness ping | 5 s | 60 s, skipped while paused |
| Memory pressure simulation | 30 s | 30 s, skipped while paused |
| InteractableWindowsRevalidator | 5 s | 10 s, skipped while paused |
| System info monitor (cpu/disks/net) | `polling_interval` (default 3, min 1) | min 2, skipped while paused |
| Notification fallback thread | 1 s | 2 s, skipped while paused |
| Keyboard layout / TIP / IME monitor (`system_settings/language`; starts when the bar asks for the layout) | 1 s, only skipped while the session was not interactive | 2 s, skipped while paused (no Windows notification covers other processes' per-thread layouts; Win+Space can take up to 2 s to show) |
| DDC/CI brightness poll (external monitors) | 5 s always once the brightness module existed | 5 s only within 30 s after a frontend read of the monitors or a brightness write (`BrightnessManager::touch_watch`), skipped while paused |
| Waveform capture | ran when instantiated, 5 ms loop | drains at 2 s unless something subscribed to `media::waveform` and not paused (never instantiated by the dock) |

Judged and left (only run during an active operation or are retries/debounces): input simulation (`windows_api/input.rs`), flashing tracker
(only while a window flashes), tray registry lookup retry (200 ms, per icon), monitor/virtual desktop retry sleeps, wifi connect wait, bluetooth
pairing, media players removal/thumbnail retries, TracedMutex cycle check (only while a lock is contended), websocket ping (30 s), power resume refresh, debouncers (100-1000 ms, event-triggered), wallpaper rotation (>= 60 s).

Phase 2 (top bar) perf rules: popups are `lazy: true` (power-menu and the 9 popups; one toolbar webview per monitor, `ReplicaByMonitor`) and are
destroyed on close by the phase 1 mechanism. The only `infinite` animations left are the `rotateToTheLeft` spinners of the bluetooth and network popups
(`.bluetooth-scanning`, `.network-scanning`: rendered only while a scan runs, and the popup webview only exists while open). The `filter: blur(14px)` of the
media popup thumbnail is gone (faint opacity tint instead). The clock (`libs/ui/svelte/runes/date.svelte.ts`) ticks once per minute, aligned to the minute
boundary, and once per second only if the format contains `ss` (default `ddd D MMM, hh:mm A` does not). Event driven with no poll: network
(NetworkInformation / wlan notifications), radios and bluetooth (`Radio.StateChanged`, device watchers), power/battery (`PowerManager` + power broadcast),
audio devices/volume (endpoint notifications), media sessions (SMTC events), notifications (WPN database watcher + the 2 s fallback in the table),
system tray (hook events + 200 ms registry retry per new icon), monitors (display change events). Usage data (`SystemInfo`: cpu, memory, gpu, disks, net) is
created lazily by the first `get_system_*` command, which only a `tb_*_usage` plugin placed in the bar issues; none is in the default items, so that
monitor loop never starts by default. The toolbar subscribes `GlobalMouseMove` only for auto-hide modes (the layered click-through hitbox shared with the
dock already needs it).

CSS: `weg-item-attention-indicator-flashing` now runs 6 iterations instead of `infinite`; the blur filter of the media-session thumbnail
(`MediaSession.svelte`) and the `backdrop-filter` of the shared Wallpaper error toast are gone. No `infinite` animation or blur/backdrop-filter remains in
`src/static`, `src/ui`, `libs/ui` styles (the scroll-timeline shadows in `weg.scss` are driven by scroll, not time).

Webview args/defaults: `hardware_acceleration` now defaults to `false` (software path, `--in-process-gpu --disable-gpu`: no GPU process) and
`unstable_optimizations` (= `--process-per-site`) defaults to `true`. These are `serde` defaults: an existing `settings.json` that already stores the
old values keeps them, edit it by hand to switch. Tauri `devtools` feature removed; `open_devtools` calls are `cfg(debug_assertions)` only (release
logs a warning for `debug_open_dev_tools`). Release profile: `debug = false` (was `limited`); `lto = true`, `codegen-units = 1`, `strip = true`,
`panic` stays unwind (the panic hook restores the taskbar).

## Phase 1 results (measured 2026-10-22, plugged in, 60 s window, `scripts/lite/measure.ps1`)

Sum over seelen-ui.exe + its own msedgewebview2 processes (matched by Seelen user-data-dir). CPU = % of the whole
machine (16 logical CPUs). Raw data: `docs/measure-*.json`. Side by side: `docs/compare-phase1-dock.png`.

| Run | Processes | CPU % | Working set sum MB | Private MB |
| --- | ---: | ---: | ---: | ---: |
| Upstream, default widgets (bar, dock, wallpaper, flyouts...) | 12 | 5.09 | 1470 | 1002 |
| Upstream, default widgets, 5 min later | 12 | 4.99 | 1590 | 1125 |
| Upstream, configured dock only | 7 | 0.21 | 484 | 234 |
| Seelen Lite phase 1 (dock) | 6 | 0.18 | 377 | 129 |

Baseline patches (local only, not committed): local signing key + matching public key; elevated service start
skipped (UAC cannot be clicked unattended), so upstream numbers exclude slu-service.exe.

Not met: the working-set sum (377 MB) is above 150 MB. One WebView2 surface alone is 5 processes (browser,
renderer, network, storage, crashpad) at ~285 MB working set; working sets double-count shared DLL pages. Private
bytes (129 MB) is under 150 MB. Tried `NetworkServiceInProcess`: one process less but private +30 MB, not kept.
Not measured: idle power (laptop was plugged in; `BatteryStatus.DischargeRate` is only valid on battery).

## Phase 2 (top bar)

Restored from upstream by reverting the two removal commits (`4d312354` toolbar, `d60c6fcd` popups), then adapted to phase 1:

- Toolbar widget `@seelen/fancy-toolbar` (`src/ui/svelte/fancy-toolbar`, `src/static/widgets/toolbar`, `src/background/widgets/toolbar`), theme style, the 11
  `tb_*` plugins, and the dock reserves space for the bar again (`weg/state/settings.svelte.ts`, `workArea`).
- Popups (all lazy, only the ones the default bar opens): `bluetooth-popup`, `calendar-popup`, `keyboard-selector`, `media-popup`, `network-popup`,
  `notifications` (+ its `Notification.svelte`, restored because the flyouts that used to share it are gone), `quick-settings`, `system-tray`, `power-menu`.
  Their optional `dock-plugin.yml`/`dock_plugin.js` came back with the bundles (opt-in dock buttons, not in the default dock items).
- Default items (only applied if the toolbar `state.yml` in its widget data dir does not exist yet; an existing file keeps its own items, delete it to
  get the new defaults). Left: user (`@seelen/tb-user-menu`, new `tb_default_user.yml`, shows picture and name; click opens `ms-settings:yourinfo`),
  `|`, focused app, `-`, focused app title. Center: date/time (opens the calendar). Right: system tray, keyboard layout, bluetooth, media/volume,
  network/wifi, battery (`@default/power`, opens `ms-settings:powersleep`), notifications, quick settings.
- Not in the default: `tb_cpu_usage`, `tb_memory_usage`, `tb_gpu_usage`, `tb_disk_usage`, `tb_network_usage` (installed, add them by hand to
  the toolbar state; the first one placed starts the system info monitor), `tb_workspaces_*` (virtual desktop switchers; still functional because
  `virtual_desktops` stays, kept), `@seelen/tb-power-menu` (available, not in the default).
- Frontend fixes: window pixel capture is gone, so the toolbar no longer reads `GetUserAppWindowsColors` / `UserAppWindowsColorsChanged` nor sets
  `--window-gradient`; all backend events reach the toolbar and popups through `subscribe` (websocket). The remaining tauri `listen`/`emit` are
  webview-local (context-menu callbacks, `hidden::sync-toolbar-items` between monitors).
- Dangling triggers are harmless: `@seelen/settings` opens the config folder, `@seelen/user-menu`, `@seelen/apps-menu` (only read as a settings key) do nothing.

Stays removed: user-menu, apps-menu, flyouts (volume/brightness OSD), settings app, window manager, workspaces viewer, wallpaper manager, task switcher.

## Phase 2 results (measured 2026-10-22, 60 s window, `scripts/lite/measure.ps1`)

| Run | Processes | CPU % | Working set sum MB | Private MB |
| --- | ---: | ---: | ---: | ---: |
| Upstream, default widgets | 12 | 5.09 | 1470 | 1002 |
| Upstream, configured dock only | 7 | 0.21 | 484 | 234 |
| Seelen Lite phase 1 (dock) | 6 | 0.18 | 377 | 129 |
| Seelen Lite phase 2 (dock + bar) | 6 | 0.20 | 434 | 173 |

Bar side by side with the target: `docs/compare-phase2-bar.png`.

Power (on battery, `scripts/lite/power.ps1`, BatteryStatus.DischargeRate sampled every 5 s for 120 s, alternating
app on / app off): on 8518, 8912, 14265 mW; off 10609, 8648 mW. The spread inside one window (6-20 W) is larger than
any on/off difference, because the desktop was not idle (other apps running). Result: no increase detectable with
this method; a clean idle-power number needs a quiet desktop and longer windows.

Not met: working-set sum 434 MB and private 173 MB are above 150 MB (WebView2 floor, see Phase 1 results).
