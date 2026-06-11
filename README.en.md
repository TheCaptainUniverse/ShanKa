# Shanka

[中文](./README.md)

Shanka is a Tauri + Vue + Bun desktop app for system-level AI text rewriting.
It is local-first by default: settings, personas, hotkeys, and optional rewrite
history stay on your machine. API keys are stored in the system keychain when
available, and selected text is sent to your configured AI provider only after
you explicitly trigger a rewrite.

Shanka is open source under the MIT License. You may use, modify, distribute,
and commercialize it, as long as the copyright notice and license text are kept.

## Using Shanka

Shanka runs in the background and can rewrite selected text in other desktop
apps. The first launch uses Chinese by default, and the language can be changed
in settings. Starting Shanka again activates the existing instance instead of
launching a second background process.

Default hotkeys:

- Safe Mode: generates an editable diff preview above the cursor. You can review
  the changes, switch to the result text, copy, replace, or regenerate with a
  different persona.
- Magic Mode: rewrites and replaces the current selection directly. Use it when
  you are confident about the rewrite workflow.

Hotkeys can be changed in settings. While recording a hotkey, Shanka pauses Safe
Mode and Magic Mode triggers to avoid accidental rewrites.

## First Setup

1. Open Shanka from the app window or system tray.
2. Choose a provider preset such as DeepSeek, OpenAI, or OpenRouter.
3. Enter the API key, base URL, and model, then run the connection test.
4. Enable launch at startup if you want Shanka to start with your system.
5. Save settings, then select text in any desktop app.
6. Start with Safe Mode to verify the preview flow before using Magic Mode.

API keys are stored in the system keychain whenever possible. The app
configuration stores only a key reference, not the plaintext key.

## Release Packages

Build Windows packages:

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

Before testing packaged builds, close any existing `shanka.exe` process. A dev
build or older package may already have registered the global hotkeys. You can
also run the full local release check:

```bash
bun run release:preflight
```

To isolate the manual Windows text-flow test environment, set
`SHANKA_CONFIG_DIR` to a temporary config directory so your daily config is not
modified:

```powershell
$env:SHANKA_CONFIG_DIR="$env:TEMP\ShankaManualConfig"
src-tauri\target\release\shanka.exe
```

Close Shanka after testing, then remove the temporary directory.

You can also start the manual test profile with fixtures:

```bash
bun run release:manual-text-test
```

It starts Shanka with mock rewrite settings, opens Notepad and browser fixtures,
prints the hotkeys required for the Windows text-flow checks, and generates a
prefilled report under `docs/release/manual/`.

To open the generated report automatically:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -OpenReport
```

To capture Shanka stdout/stderr into the report directory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release-manual-text-test.ps1 -CaptureLog
```

## Development

```bash
bun install
bun run dev
```

Common commands:

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

Recommended checks before release:

```bash
bun run check
bun run tauri build
bun run release:smoke
bun run release:manual-text-smoke
bun run release:install-smoke
bun run release:msi-smoke
bun run release:manifest
```

Or run the complete local release preflight:

```bash
bun run release:preflight
```

## Troubleshooting

- No selected text detected: make sure the target app has an active text
  selection, then trigger Safe Mode again.
- Hotkey registration failed: the shortcut may be occupied by the system or
  another app. Record a different combination in settings.
- Paste replacement failed: if the Windows target app runs as administrator,
  Shanka keeps the result in the clipboard.
- macOS input is unavailable: enable Accessibility permission for Shanka in
  System Settings.
- Linux behavior depends on the desktop session. X11 is the currently
  prioritized validation path; Wayland may restrict global hotkeys and simulated
  input.

## Local-First Privacy

Shanka stores settings, personas, hotkeys, and rewrite history locally by
default. API keys are stored through the system keychain whenever possible, and
the config file stores only a key reference.

Selected text is sent only after you trigger Safe Mode or Magic Mode, and only
to the provider endpoint you configured. Rewrite history can be disabled or
cleared in settings.

Default logs avoid printing full selected text, provider response bodies, and
complete API keys. You can temporarily enable debug logs in settings when
diagnosing clipboard or provider issues.

## License

Shanka is licensed under the MIT License. You may use, copy, modify, merge,
publish, distribute, sublicense, and sell copies of the software, provided that
the copyright notice and license text are included in all copies or substantial
portions of the software.

See [`LICENSE`](./LICENSE) for the full terms.

## Architecture

- `src/`: Vue 3 settings panel and HUD frontend.
- `src-tauri/`: Rust system host for global hotkeys, clipboard capture, input
  simulation, tray, windows, provider calls, keychain storage, local history, and
  the rewrite pipeline.
- `server/`: Bun + Hono local service bus and Drizzle SQLite schema for
  development experiments. The production rewrite flow currently runs in Rust
  and does not require a sidecar.
- `shared/`: Shared contracts, events, schemas, HUD state, and error codes.
