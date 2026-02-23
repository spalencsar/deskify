# Changelog

All notable changes to `deskify` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows early MVP/alpha versioning with tags like `v0.1.0-alpha.1`.

## [Unreleased]

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
