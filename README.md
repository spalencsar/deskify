<h1 align="center">deskify</h1>

<p align="center">
  <b>Turn websites into first-class Linux desktop applications with Rust, Tauri, and the system WebView.</b><br>
  Build native-feeling, standalone web app wrappers with CLI-first Linux integration.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/OS-Linux-blue?style=flat-square" alt="Linux only" />
  <img src="https://img.shields.io/badge/Language-Rust-orange?style=flat-square" alt="Written in Rust" />
  <img src="https://img.shields.io/badge/Powered_by-Tauri_v2-grey?style=flat-square" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/Status-Alpha-yellow?style=flat-square" alt="Alpha status" />
</p>

<p align="center">
  <img src="docs/assets/chat-kde.png" alt="Deskify app window running chat.com on KDE Plasma" width="49%" />
  <img src="docs/assets/chat-launcher-kde.png" alt="Deskify app visible in KDE launcher" width="49%" />
</p>

<p align="center">
  <img src="docs/assets/deskify-list-terminal.png" alt="deskify list terminal output" width="80%" />
</p>

<p align="center"><sub>Screenshots: KDE Plasma on Arch Linux (alpha MVP workflow).</sub></p>

---

## Why Deskify Exists

Web apps are now core tools, but Linux desktops still treat them like second-class citizens.

- Electron-based wrappers solve distribution, but often at high RAM/disk cost.
- PWAs help in some browsers, but desktop integration is inconsistent across environments.
- The original [Nativefier](https://github.com/nativefier/nativefier) proved the need, but it is now unmaintained.

`deskify` takes a Linux-first approach: it turns a website into a native-feeling desktop app using [Tauri](https://tauri.app/) and the system webview (for example `webkit2gtk`) instead of bundling a full browser engine per app.

## Problem Fit (Why Deskify vs. Other Approaches)

Deskify is optimized for Linux users who want native desktop integration and CLI-friendly automation for web apps, without shipping a bundled browser runtime per app.

The comparison below is intentionally rough and practical (not benchmark marketing). It describes tradeoffs, not winners in every category.

| Approach | Runtime model | Desktop integration | Automation / scripting | Isolation / sandbox | Setup complexity | Offline capability |
| --- | --- | --- | --- | --- | --- | --- |
| Electron / Nativefier | Bundled Chromium per app | Good | Low to medium | Per-app process, app-bundled runtime | Easy to medium | Depends on the site |
| PWA | Browser runtime | Limited / browser-dependent | Low | Shared browser context | Very easy | Depends on browser + site |
| Flatpak web app models | Sandbox-oriented runtime model | Good | Limited | Strong sandbox model | Medium | Depends on runtime + site |
| **Deskify** | System WebView | Native (`.desktop`, icons, WMClass) | **High (CLI-first)** | Medium (shared system WebView security model) | Medium (Tauri prerequisites) | Depends on the site |

### Deskify Focus (Today)

Deskify focuses on:

- system-native Linux desktop integration
- minimal runtime overhead by relying on the system WebView
- automation-friendly CLI workflows
- straightforward local install/remove lifecycle management

Deskify intentionally does not aim to:

- replace full browser sandbox products
- provide a cross-platform abstraction layer
- ship bundled browser runtimes per generated app

## How Deskify Works Today (Alpha / MVP)

`deskify` is ready for public use and testing as an **early MVP**, but it is **not production-hardened yet**.

- **Platform scope:** Linux only
- **Current architecture:** one generated Tauri wrapper project per app
- **Build model:** `deskify` compiles the wrapper locally on your machine
- **Why this MVP design:** simpler debugging, isolated failures, low contributor complexity
- **Tradeoff:** build time and distro-specific setup issues become user-facing
- **Test coverage:** currently limited (core behavior is implemented, automated coverage is growing)

### MVP Architecture (Today)

```mermaid
flowchart TD
    A[deskify CLI] --> B[Generate temporary Tauri wrapper project]
    B --> C[Fetch or copy icon]
    C --> D[Write tauri.conf.json + source files]
    D --> E[Run cargo tauri build locally]
    E --> F[Install binary to local executable dir]
    F --> G[Install icon + .desktop entry]
    G --> H[Launch from Linux app menu]
```

### Known Limitations

- Requires Tauri/Linux system dependencies to be installed locally before `deskify build`
- Generated app build success can vary by distro/system setup (WebKitGTK/Tauri prerequisites)
- No official cross-platform support (Windows/macOS) yet
- GitHub Releases / binary automation for `deskify` itself may lag behind source updates during early MVP

## Where Deskify Is Going

Deskify aims to make web applications **first-class Linux desktop applications**.

Planned evolution (direction, not promise):

- Shared runtime model for multiple apps
- Per-app config and icon installs without full rebuilds
- Stronger Linux desktop integration (deployment, automation, kiosk/admin workflows)
- Better distro compatibility guidance and troubleshooting

The current per-app build approach is intentional for the MVP. The goal is to keep it simple now while making the longer-term direction visible.

### Planned Evolution (Concept)

```mermaid
flowchart TD
    A[deskify CLI] --> B[deskify runtime binary]
    A --> C[App config files]
    A --> D[Icons + .desktop entries]
    C --> E[apps/chatgpt.json]
    C --> F[apps/netflix.json]
    B --> G[Load app config at launch]
    G --> H[Open site in system webview]
    D --> I[Linux app menu integration]
```

## Why Not Just Use Flatpak Web Apps / PWAs / Electron?

The table above covers the technical tradeoffs in more detail. In practice, `deskify` is most useful when you want:

- a CLI-first workflow for repeatable installs and automation
- Linux-native desktop integration (`.desktop`, icons, window class behavior)
- a system-WebView-based runtime model instead of shipping a bundled browser per app

### Key Features
- **System-WebView Runtime Model:** Generated wrappers stay small because Deskify relies on the system WebView instead of bundling a browser runtime per app.
- **Automatic Icon Fetching:** Scrapes high-quality 128x128 favicons automatically using the Google Favicon API.
- **XDG-Compliant System Integration:** Safely creates `.desktop` entries in `~/.local/share/applications` and manages application icons in `~/.local/share/icons/hicolor/`.
- **Wayland/X11 Ready:** Perfectly binds `StartupWMClass` to ensure your DE groups the app exactly to its custom icon (no generic gear icons in GNOME/KDE taskbars).
- **Kiosk / Fullscreen Mode:** Pin applications perfectly as dashboards.
- **User-Agent Spoofing:** Trick picky sites (like WhatsApp Web) into working within the webview.
- **Clean App Management:** Effortlessly list and remove created apps without leaving orphaned files behind.

---

## 🚀 Installation

### 1. System Dependencies
Because `deskify` compiles Tauri applications natively on your machine, you need the standard Tauri prerequisites installed before using it.

On **Ubuntu / Debian**:
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

On **Arch Linux / Manjaro**:
```bash
sudo pacman -S webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  appmenu-gtk-module \
  gtk3 \
  libappindicator-gtk3 \
  librsvg \
  libvips
```

*(For Fedora or other distros, refer to the [Tauri Prerequisites Guide](https://v2.tauri.app/start/prerequisites/).)*

### 2. Rust & Tauri CLI
You'll need the Rust compiler and the Tauri CLI to compile the generated apps.
```bash
# Ensure Rust is installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri-CLI globally
cargo install tauri-cli --version "^2.0.0"
```

### 3. Install deskify
Clone the repository and install it directly via Cargo:
```bash
git clone https://github.com/spalencsar/deskify.git
cd deskify
cargo install --path .
```

---

## 🛠️ Usage

### Creating an App (`build`)
The `build` subcommand requires a `--url` and a `--name`.

```bash
deskify build --url "https://chatgpt.com" --name "ChatGPT"
```

Once the build finishes, `ChatGPT` will instantly appear in your system Application Launcher (e.g., Rofi, Wofi, GNOME Dash).

`deskify` derives a safe internal ID from the app name (for example, `ChatGPT` becomes `chatgpt`).

#### Advanced Build Options
```bash
deskify build \
  --url "https://netflix.com" \
  --name "Netflix" \
  --fullscreen \
  --no-decorations \
  --user-agent "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36" \
  --dark-mode \
  --width 1280 \
  --height 720
```

* `--icon <PATH>`: Provide a custom PNG icon instead of auto-downloading one.
* `--fullscreen`: Starts the app in Kiosk mode.
* `--no-decorations`: Disables native window decorations (frameless window; useful for dashboards/kiosk setups).
* `--user-agent <UA>`: Useful to bypass webview restrictions on certain platforms.
* `--dark-mode`: Forces the Tauri webview into a dark theme.
* `--width <PX>` / `--height <PX>`: Sets the startup resolution.

### Managing Apps (`list` & `remove`)
You can view all applications generated by `deskify`:
```bash
deskify list
```
*Output:*

```text
Installed Deskify Apps:
- ChatGPT (Internal ID: chatgpt)
- Netflix (Internal ID: netflix)
```

To entirely uninstall an app (including the binary, desktop entry, and icons):
```bash
deskify remove chatgpt
```

`remove` expects this **internal ID** (sanitized lowercase letters, numbers, `-`), not the display name.

---

## 🧪 Manual Smoke Test Checklist (Before a Public Release)

```bash
deskify build --url "https://example.com" --name "Example"
deskify list
deskify remove example
```

Also test invalid IDs:

```bash
deskify remove ../foo
deskify remove FooBar
```

Expected: clean validation error and no unintended file deletion.

---

## 🏷️ Versioning & GitHub Releases (MVP)

- Recommended tag format during MVP: `v0.1.0-alpha.N`
- Start with **source-first** releases (repository + tags)
- Add automated binary releases later via GitHub Actions

For the current public alpha, use `v0.1.0-alpha.2` (and continue with `v0.1.0-alpha.N` for follow-up releases).

---

## 📦 Open Source Acknowledgements

`deskify` is built on the shoulders of giants. It leverages the following fantastic open-source projects:
* **[Tauri](https://tauri.app/)** - The core framework driving the native wrap.
* **[Clap](https://crates.io/crates/clap)** - Command-Line Argument Parser for Rust.
* **[Anyhow](https://crates.io/crates/anyhow)** - Excellent error handling context.
* **[Ureq](https://crates.io/crates/ureq)** - Minimalist sync HTTP request library (used for icon fetching).
* **[Directories](https://crates.io/crates/directories)** - Abstractions for standard OS directories (XDG Base Dirs).

## 📝 License
This project is licensed under the [MIT License](LICENSE).
