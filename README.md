# Shanka

Shanka is a Tauri + Vue + Bun desktop app for system-level AI text refinement.

## Using Shanka

Shanka runs in the background and refines selected text from other desktop apps.
The first launch defaults to Chinese UI, and the language can be changed in
Settings. Launching Shanka again activates the existing instance instead of
starting a second background process.

Default hotkeys:

- Safe Mode: generate an editable preview above the cursor. You can copy,
  replace, or regenerate the result before it touches the original text.
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
bun run release:install-smoke
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
fixture, and prints the hotkeys to use for the Windows text-link checklist.

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
bun run release:install-smoke
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

## Privacy

By default, logs avoid full selected text, provider response bodies, and complete
API keys. Debug logging can be enabled temporarily from Settings when diagnosing
clipboard or provider issues.

## Architecture

- `src/`: Vue 3 settings panel and HUD frontend.
- `src-tauri/`: Rust system host for hotkeys, clipboard capture, input simulation, tray, windows, provider calls, keychain storage, local history, and rewrite workflow.
- `server/`: Bun + Hono local service bus and Drizzle SQLite schema retained for development experiments; production rewrite currently runs in Rust without a sidecar.
- `shared/`: shared contracts, events, modes, HUD statuses, and error codes.
