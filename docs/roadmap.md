# Roadmap (Alpha)

This roadmap is intentionally pragmatic: it prioritizes "time-to-first-success" and UX gaps before deeper architecture work.

## P0 - Config Persistence

Goal: `deskify update <id>` works without `--url`, and `deskify list --verbose` can show URL/backend.

- Store metadata under `~/.local/share/deskify/apps/<id>.json`
- Persist (v1):
  - `id`, `name`, `url`
  - `backend`, `browser_bin` (optional), `profile_scope`
  - `fullscreen`, `no_decorations`, `user_agent` (optional), `width`, `height`, `dark_mode`
  - `schema_version`
- Migration: older installs without metadata should be migrated on first update/build

## P1 - Distribution / Onboarding

Goal: install `deskify` without a full Rust/Tauri toolchain.

- Promote "install from GitHub Releases" in README
- Keep CI releases producing a Linux binary asset
- Enable community packaging (AUR `deskify-bin`, etc.)

## P2 - Shared Runtime Model (only after P0/P1)

Goal: reduce per-app build time and prerequisites for the `tauri` backend.

- Concept: shared runtime/launcher + per-app configs (no per-app compile)

## P3 - QoL

- `deskify list --verbose` improvements (URLs/backend/profile scope)
- Better FAQ: WebView vs Chromium backend tradeoffs
- Optional DE integrations (later): KDE/GNOME widgets/extensions

