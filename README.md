# Shanka

Shanka is a Tauri + Vue + Bun desktop app for system-level AI text refinement.

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
```

## Architecture

- `src/`: Vue 3 settings panel and HUD frontend.
- `src-tauri/`: Rust system host for hotkeys, clipboard capture, input simulation, tray, windows, provider calls, keychain storage, local history, and rewrite workflow.
- `server/`: Bun + Hono local service bus and Drizzle SQLite schema retained for development experiments; production rewrite currently runs in Rust without a sidecar.
- `shared/`: shared contracts, events, modes, HUD statuses, and error codes.
