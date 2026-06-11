# Shanka

Shanka is a Tauri + Vue + Bun desktop app for system-level AI text refinement.
It is local-first by design: settings, personas, hotkeys, and optional rewrite
history stay on your device, API keys live in the system keychain, and selected
text is sent only when you explicitly trigger a rewrite.

## Using Shanka

Shanka runs in the background and refines selected text from other desktop apps.
The first launch defaults to Chinese UI, and the language can be changed in
Settings. Launching Shanka again activates the existing instance instead of
starting a second background process.

Default hotkeys:

- Safe Mode: generate an editable diff preview above the cursor. You can review
  what changed, switch to the result text, copy, replace, or regenerate before
  it touches the original text.
- Magic Mode: rewrite and replace the selected text directly.

The default hotkeys can be changed from Settings. During hotkey recording,
Shanka pauses Safe/Magic triggers so recording a shortcut does not accidentally
rewrite text.

## First Setup

1. Open Shanka from the app window or system tray.
2. Choose a provider preset such as DeepSeek, OpenAI, or OpenRouter.
3. Enter the API key, Base URL, and model, then run the connection test.
4. Optionally enable Launch at Login if Shanka should start with the system.
5. Save the settings and select text in any desktop app.
6. Trigger Safe Mode first to verify the preview flow before using Magic Mode.

API keys are stored in the system keychain when available. The app config stores
only a key reference, not the plaintext key.

## Release Packages

Build Windows packages with:

```bash
bun run tauri build
bun run release:smoke
bun run release:manual-text-smoke
bun run release:install-smoke
bun run release:msi-smoke
bun run release:manifest
```

Current Windows outputs:

- `src-tauri/target/release/bundle/msi/Shanka_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/Shanka_0.1.0_x64-setup.exe`

Before testing a packaged build, close any existing `shanka.exe` process. A
running development or old packaged instance can already own the global hotkeys.
Use `bun run release:preflight` to run the full local release gate in one pass.

For manual Windows text-link testing, you can isolate test settings and history
from your daily Shanka profile by starting the app with `SHANKA_CONFIG_DIR`:

```powershell
$env:SHANKA_CONFIG_DIR="$env:TEMP\ShankaManualConfig"
src-tauri\target\release\shanka.exe
```

Close Shanka before returning to normal use, then remove the temporary directory
when you no longer need the test profile.

You can also launch a prepared manual test profile with fixtures:

```bash
bun run release:manual-text-test
```

It starts Shanka with mock rewrite settings, opens a Notepad fixture and browser
fixture, prints the hotkeys to use for the Windows text-link checklist, and
creates a prefilled report under `docs/release/manual/`.

Use `-OpenReport` when you want the generated report opened automatically:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -OpenReport
```

Use `-CaptureLog` to save Shanka process stdout/stderr next to the generated
manual report. This is useful when recording Blocker or High issues:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -CaptureLog
```

## Development

```bash
bun install
bun run dev
```

Useful commands:

```bash
bun run dev:web
bun run dev:server
bun run dev:app
bun run check
bun run typecheck
bun run db:generate
bun run db:push
bun run tauri build
```

Before packaging a release, run:

```bash
bun run check
bun run tauri build
bun run release:smoke
bun run release:manual-text-smoke
bun run release:install-smoke
bun run release:msi-smoke
bun run release:manifest
```

Or run the complete local release gate:

```bash
bun run release:preflight
```

## Troubleshooting

- No selected text: make sure the target app has an active text selection, then
  try Safe Mode again.
- Hotkey registration failed: another app may already use the shortcut. Record a
  different shortcut from Settings.
- Paste failed: if the target app is elevated on Windows, Shanka keeps the result
  on the clipboard and shows a saved-to-clipboard state.
- macOS input does not work: enable Accessibility permission for Shanka in
  System Settings.
- Linux behavior depends on the desktop session. X11 is the preferred validation
  path; Wayland may limit global hotkeys and simulated input.

## Local-First Privacy

By default, Shanka keeps configuration, personas, hotkeys, and rewrite history
on the local device. API keys are stored through the system keychain when
available, and the app config stores only a key reference.

Selected text is sent only after you trigger Safe Mode or Magic Mode, and only
to the provider endpoint you configured. Rewrite history can be disabled or
cleared from Settings.

Logs avoid full selected text, provider response bodies, and complete API keys
unless debug logging is enabled temporarily from Settings while diagnosing
clipboard or provider issues.

## Architecture

- `src/`: Vue 3 settings panel and HUD frontend.
- `src-tauri/`: Rust system host for hotkeys, clipboard capture, input simulation, tray, windows, provider calls, keychain storage, local history, and rewrite workflow.
- `server/`: Bun + Hono local service bus and Drizzle SQLite schema retained for development experiments; production rewrite currently runs in Rust without a sidecar.
- `shared/`: shared contracts, events, modes, HUD statuses, and error codes.
