# Changelog

All notable changes to `deskify` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows early MVP/alpha versioning with tags like `v0.1.0-alpha.1`.

## [Unreleased]

## [v0.1.1-alpha.2] - 2026-05-28

### Changed
- Major internal refactoring of XDG / Base Directory handling:
  - Introduced dedicated `src/xdg.rs` module to centralize all path resolution logic.
  - Removed duplicated `BaseDirs` usage across `app.rs`, `desktop.rs`, `chromium.rs`, `doctor.rs`, and `install.rs`.
- Moved remaining unit tests out of `main.rs` into their proper modules (`desktop.rs`, `chromium.rs`, `tauri.rs`, `types.rs`, `validation.rs`).
- Updated `rust-version` from 1.85 to 1.95 to match the project's documented requirements in `AGENTS.md`.

### Fixed
- Fixed a bug in the Chromium backend where writing the icon would fail with `ENOENT` because the temporary directory was dropped too early.
  Thanks to [@shocklateboy92](https://github.com/shocklateboy92) for the fix in [#1](https://github.com/spalencsar/deskify/pull/1).

### Removed
- Temporary refactoring artifacts and internal "work-in-progress" comments from the xdg centralization phase (preparation for public repository).

### Improved
- Significantly reduced code duplication in path handling.
- Cleaner and more maintainable project structure for future development.

## [v0.1.1-alpha.1] - 2026-05-04

### Added
- Comprehensive unit test suite (37 tests covering desktop entry escaping, Tauri config generation, validation, Chromium backend, and app config serialization)
- Module refactoring: code split into 10 focused modules (types, validation, tauri, chromium, icon, desktop, install, app, doctor)

### Fixed
- Test coverage gaps resolved with tests for:
  - `desktop_exec_escape` handling of empty strings, whitespace, quotes, and backslashes
  - `validate_remove_id` rejection of uppercase, special characters, and path traversal attempts
  - Tauri config generation (fullscreen, no-decorations, dark-mode, user-agent, window dimensions, CSP)
  - App config serialization roundtrip

### Changed
- Split `main.rs` (~1900 lines) into modular structure:
  - `src/types.rs` - CLI definitions and types
  - `src/validation.rs` - ID sanitization and validation
  - `src/tauri.rs` - Tauri config and project generation
  - `src/chromium.rs` - Browser detection and exec building
  - `src/icon.rs` - Icon fetching and processing
  - `src/desktop.rs` - Desktop entry utilities
  - `src/install.rs` - App installation logic
  - `src/app.rs` - App management (list/remove/config)
  - `src/doctor.rs` - Diagnostics
  - `src/main.rs` - Entry point and orchestration

## [v0.1.0-alpha.9] - 2026-02-25

### Fixed
- Accept legacy `profile_scope=default` in persisted app configs (treated as `isolated`) and fall back to desktop metadata when a config is invalid
- Validate Chromium browser binaries before installing `.desktop` entries (prevents silent failures with wrapper scripts)

### Added
- AUR publish automation (PKGBUILD generator + GitHub Actions workflow) for `deskify-bin`

### Documentation
- Quick Start improvements and Chromium backend troubleshooting notes
- AUR install instructions and alpha warning (`yay -S deskify-bin`)

## [v0.1.0-alpha.8] - 2026-02-25

### Added
- Persisted app metadata under `~/.local/share/deskify/apps/<id>.json`
- `deskify list --verbose` to show URL/backend when metadata is available

### Changed
- `deskify update <id>` no longer requires `--url` when metadata exists (migrates older installs on first update/build)

## [v0.1.0-alpha.7] - 2026-02-25

### Added
- AUR `deskify-bin` packaging template with checksums

## [v0.1.0-alpha.6] - 2026-02-23

### Added
- `chromium` compatibility backend via `--backend chromium` using an installed Chromium-based browser in app mode (no local Tauri build for generated apps)
- `build` / `update` flags for Chromium backend selection and runtime control:
  - `--backend <tauri|chromium>`
  - `--browser-bin <PATH>`
  - `--profile-scope <isolated|shared>`
- Chromium app install flow with XDG `.desktop` launchers and per-app metadata (`X-Deskify-Backend=chromium`)
- Optional isolated Chromium profiles under `~/.local/share/deskify/profiles/<id>`
- `doctor` now reports whether a Chromium-based browser is available in `PATH`

### Changed
- `update <id>` now keeps the internal app ID stable even when the display name changes
- `list` now derives the internal ID from the `.desktop` filename (works for both Tauri and Chromium backends)
- `remove` now also cleans up Chromium profile directories (when present)
- `update` keeps Chromium isolated profiles by default to preserve sessions/cookies during reinstall

### Fixed
- Tauri wrapper generation now consistently uses the existing internal ID for generated app identifiers (prevents accidental ID drift during updates)
- `.desktop` command generation for Chromium backend now escapes arguments (URLs / user agents with spaces or special characters)

### Documentation
- README now documents backend modes (`tauri` vs `chromium`) and recommends `--backend chromium` for sites that fail in the system WebView backend
- Added Chromium backend usage examples (`--browser-bin`, `--profile-scope`) and updated compatibility notes

### Added
- `build --no-decorations` option to create frameless windows (`decorations: false`) for kiosk/dashboard-style setups
- README screenshots for KDE/Arch showing launcher integration, running app window, and `deskify list` workflow

### Fixed
- Icon handling now normalizes downloaded and custom icons to RGBA PNG before Tauri build (prevents `icon ... is not RGBA` build failures)

## [v0.1.0-alpha.2] - 2026-02-23

### Fixed
- Resolved a CI failure caused by Clippy (`items_after_test_module`) by moving the test module to the end of `src/main.rs`

### Changed
- No functional CLI behavior changes; this is a release/CI correctness follow-up to `v0.1.0-alpha.1`

## [v0.1.0-alpha.1] - 2026-02-23

### Added
- Initial public MVP/alpha release of `deskify` (Linux-only)
- CLI commands:
  - `deskify build --url <URL> --name <NAME>`
  - `deskify list`
  - `deskify remove <internal-id>`
- `build` supports advanced options:
  - `--icon`
  - `--fullscreen`
  - `--user-agent`
  - `--dark-mode`
  - `--width`
  - `--height`
- Tauri-based wrapper generation in a temporary build directory
- XDG desktop integration:
  - installs wrapper binary into local executable directory (`~/.local/bin` fallback)
  - creates `.desktop` launcher in `~/.local/share/applications`
  - installs app icon into `~/.local/share/icons/hicolor/128x128/apps`
- Automatic favicon download from Google Favicon API with dummy-icon fallback
- `.desktop` metadata marker (`X-Deskify-Managed=true`) for more reliable app discovery
- Unit tests for:
  - app ID sanitization
  - `remove` ID validation
  - Deskify desktop entry detection (marker + legacy fallback)
- GitHub CI workflow (`cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo check`)
- Tag-based GitHub Release workflow to upload a Linux `deskify` CLI binary
- GitHub PR template and release-note categorization config

### Changed
- `tauri.conf.json` generation now uses structured JSON serialization (`serde_json`) instead of manual string interpolation
- App discovery in `list` prefers the Deskify marker and keeps a legacy heuristic fallback for older installs
- Documentation updated for alpha/MVP positioning, known limitations, smoke tests, and release tagging (`v0.1.0-alpha.N`)

### Fixed
- `remove` now validates internal IDs and rejects unsafe values (for example path traversal inputs like `../foo`)
- Generated Tauri config creation is more robust for names/URLs/user agents containing special characters
