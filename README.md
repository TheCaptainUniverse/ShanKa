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
bun run typecheck
bun run db:generate
bun run db:push
```

## Architecture

- `src/`: Vue 3 settings panel and HUD frontend.
- `src-tauri/`: Rust system host for hotkeys, clipboard, input simulation, tray, windows, and service bridge.
- `server/`: Bun + Hono local AI service bus and Drizzle SQLite schema.
- `shared/`: shared contracts, events, modes, HUD statuses, and error codes.
