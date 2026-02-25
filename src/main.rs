use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use directories::BaseDirs;
use image::{DynamicImage, ImageFormat};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use url::Url;

#[derive(
    clap::ValueEnum, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "lowercase")]
enum Backend {
    Tauri,
    Chromium,
}

#[derive(
    clap::ValueEnum, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "lowercase")]
enum ProfileScope {
    #[serde(alias = "default")]
    Isolated,
    Shared,
}

#[derive(Debug, Clone)]
struct BuildArgs {
    url: String,
    name: String,
    internal_id: String,
    icon: Option<String>,
    fullscreen: bool,
    no_decorations: bool,
    user_agent: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    dark_mode: bool,
    backend: Backend,
    browser_bin: Option<String>,
    profile_scope: ProfileScope,
}

/// Deskify - Turn any URL into a native Linux desktop application using Tauri.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build and install a new native application from a URL
    Build {
        /// The URL of the website to wrap (e.g., https://youtube.com)
        #[arg(short, long)]
        url: String,

        /// The name of the application (e.g., "YouTube")
        #[arg(short, long)]
        name: String,

        /// Optional path to a custom icon (PNG format is recommended)
        #[arg(short, long)]
        icon: Option<String>,

        /// Launch the application in fullscreen (Kiosk) mode
        #[arg(short, long)]
        fullscreen: bool,

        /// Disable native window decorations (frameless window)
        #[arg(long)]
        no_decorations: bool,

        /// Set a custom User-Agent string for the webview
        #[arg(short = 'A', long)]
        user_agent: Option<String>,

        /// Set the initial window width
        #[arg(short = 'W', long)]
        width: Option<f64>,

        /// Set the initial window height
        #[arg(short = 'H', long)]
        height: Option<f64>,

        /// Force the webview into Dark Mode
        #[arg(short, long)]
        dark_mode: bool,

        /// Backend to use for the generated app
        #[arg(long, value_enum, default_value_t = Backend::Tauri)]
        backend: Backend,

        /// Path to a Chromium-based browser binary (only for `--backend chromium`)
        #[arg(long)]
        browser_bin: Option<String>,

        /// Profile isolation for Chromium backend (only for `--backend chromium`)
        #[arg(long, value_enum, default_value_t = ProfileScope::Isolated)]
        profile_scope: ProfileScope,

        /// Print the generated Tauri config JSON and exit
        #[arg(long)]
        print_config: bool,

        /// Show planned build/install actions without building or installing
        #[arg(long)]
        dry_run: bool,
    },
    /// List all installed apps created by deskify
    List {
        /// Show additional metadata (URL, backend) when available
        #[arg(long)]
        verbose: bool,
    },
    /// Check local prerequisites and environment diagnostics
    Doctor,
    /// Remove a specific app by its internal ID
    Remove {
        /// The safe name/ID of the app (e.g., "youtube")
        id: String,
    },
    /// Rebuild and reinstall an existing app ID with new settings (URL is optional when persisted)
    Update {
        /// The existing internal ID (e.g., "chatgpt")
        id: String,

        /// The URL to wrap (optional if Deskify has persisted app metadata for this ID)
        #[arg(short, long)]
        url: Option<String>,

        /// Optional new display name (defaults to current desktop entry name or the ID)
        #[arg(short, long)]
        name: Option<String>,

        /// Optional path to a custom icon (PNG format is recommended)
        #[arg(short, long)]
        icon: Option<String>,

        /// Launch the application in fullscreen (Kiosk) mode
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        fullscreen: Option<bool>,

        /// Disable native window decorations (frameless window)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        no_decorations: Option<bool>,

        /// Set a custom User-Agent string for the webview
        #[arg(short = 'A', long)]
        user_agent: Option<String>,

        /// Set the initial window width
        #[arg(short = 'W', long)]
        width: Option<f64>,

        /// Set the initial window height
        #[arg(short = 'H', long)]
        height: Option<f64>,

        /// Force the webview into Dark Mode
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        dark_mode: Option<bool>,

        /// Backend to use for the updated app
        #[arg(long, value_enum)]
        backend: Option<Backend>,

        /// Path to a Chromium-based browser binary (only for `--backend chromium`)
        #[arg(long)]
        browser_bin: Option<String>,

        /// Profile isolation for Chromium backend (only for `--backend chromium`)
        #[arg(long, value_enum)]
        profile_scope: Option<ProfileScope>,

        /// Show planned update actions without rebuilding/installing
        #[arg(long)]
        dry_run: bool,

        /// Print the generated Tauri config JSON for the updated app and exit
        #[arg(long)]
        print_config: bool,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct DeskifyAppConfig {
    schema_version: u32,
    id: String,
    name: String,
    url: String,
    backend: Backend,
    browser_bin: Option<String>,
    profile_scope: ProfileScope,
    fullscreen: bool,
    no_decorations: bool,
    user_agent: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    dark_mode: bool,
}

fn sanitize_app_id(name: &str) -> String {
    let re = Regex::new(r"[^a-z0-9-]").unwrap();
    let lower_name = name.to_lowercase().replace(' ', "-");
    let sanitized = re.replace_all(&lower_name, "");
    if sanitized.is_empty() {
        "app".to_string()
    } else {
        sanitized.to_string()
    }
}

fn validate_remove_id(id: &str) -> Result<()> {
    let re = Regex::new(r"^[a-z0-9-]+$").unwrap();
    if re.is_match(id) {
        Ok(())
    } else {
        Err(anyhow!(
            "Invalid app ID '{}'. Allowed characters: lowercase letters, numbers, and '-'.",
            id
        ))
    }
}

fn profile_scope_str(scope: ProfileScope) -> &'static str {
    match scope {
        ProfileScope::Isolated => "isolated",
        ProfileScope::Shared => "shared",
    }
}

fn backend_str(backend: Backend) -> &'static str {
    match backend {
        Backend::Tauri => "tauri",
        Backend::Chromium => "chromium",
    }
}

fn deskify_data_dir() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;
    Ok(base_dirs.data_local_dir().join("deskify"))
}

fn app_config_path(id: &str) -> Result<PathBuf> {
    validate_remove_id(id)?;
    Ok(deskify_data_dir()?
        .join("apps")
        .join(format!("{}.json", id)))
}

fn read_app_config(id: &str) -> Result<Option<DeskifyAppConfig>> {
    let path = app_config_path(id)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read app config {}", path.display()))?;
    match serde_json::from_str::<DeskifyAppConfig>(&content) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(err) => {
            eprintln!(
                "Warning: Failed to parse app config {} ({}). Falling back to desktop entry metadata.",
                path.display(),
                err
            );
            Ok(None)
        }
    }
}

fn write_app_config_from_args(args: &BuildArgs) -> Result<()> {
    validate_remove_id(&args.internal_id)?;
    let path = app_config_path(&args.internal_id)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).context("Failed to create app config directory")?;
    }
    let cfg = DeskifyAppConfig {
        schema_version: 1,
        id: args.internal_id.clone(),
        name: args.name.clone(),
        url: args.url.clone(),
        backend: args.backend,
        browser_bin: args.browser_bin.clone(),
        profile_scope: args.profile_scope,
        fullscreen: args.fullscreen,
        no_decorations: args.no_decorations,
        user_agent: args.user_agent.clone(),
        width: args.width,
        height: args.height,
        dark_mode: args.dark_mode,
    };
    fs::write(
        &path,
        serde_json::to_string_pretty(&cfg).context("Failed to serialize app config")?,
    )
    .with_context(|| format!("Failed to write app config {}", path.display()))?;
    Ok(())
}

fn remove_app_config(id: &str) -> Result<()> {
    let path = app_config_path(id)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;
        println!("Removed app config: {:?}", path);
    }
    Ok(())
}

fn build_tauri_config_value(args: &BuildArgs, safe_identifier: &str, bundle_active: bool) -> Value {
    let mut window_config = Map::<String, Value>::new();
    window_config.insert("title".to_string(), json!(args.name));
    window_config.insert("url".to_string(), json!(args.url));

    if args.fullscreen {
        window_config.insert("fullscreen".to_string(), json!(true));
    }
    if args.no_decorations {
        window_config.insert("decorations".to_string(), json!(false));
    }
    if let Some(ua) = &args.user_agent {
        window_config.insert("userAgent".to_string(), json!(ua));
    }
    if let Some(w) = args.width {
        window_config.insert("width".to_string(), json!(w));
    }
    if let Some(h) = args.height {
        window_config.insert("height".to_string(), json!(h));
    }
    if args.dark_mode {
        window_config.insert("theme".to_string(), json!("Dark"));
    }

    json!({
        "$schema": "https://schema.tauri.app/config/2",
        "productName": args.name,
        "version": "0.1.0",
        "identifier": format!("com.deskify.{}", safe_identifier),
        "build": {
            "frontendDist": "dist"
        },
        "app": {
            "windows": [Value::Object(window_config)],
            "security": { "csp": null }
        },
        "bundle": {
            "active": bundle_active,
            "targets": "all",
            "icon": ["icons/icon.png"]
        }
    })
}

fn desktop_exec_escape(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    let needs_quotes = arg
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '\\');
    if !needs_quotes {
        return arg.to_string();
    }

    let escaped = arg.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn desktop_exec_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| desktop_exec_escape(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn chromium_candidates() -> &'static [&'static str] {
    &[
        "google-chrome",
        "chromium",
        "chromium-browser",
        "brave-browser",
        "vivaldi",
        "microsoft-edge",
    ]
}

fn find_binary_in_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn resolve_chromium_binary(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(anyhow!(
            "Specified browser binary does not exist or is not a file: {}",
            candidate.display()
        ));
    }

    for candidate in chromium_candidates() {
        if let Some(path) = find_binary_in_path(candidate) {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "No Chromium-based browser found in PATH. Install Chromium/Chrome/Brave or use --browser-bin /path/to/browser."
    ))
}

fn chromium_profile_dir(internal_id: &str) -> Result<PathBuf> {
    validate_remove_id(internal_id)?;
    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;
    Ok(base_dirs
        .data_local_dir()
        .join("deskify")
        .join("profiles")
        .join(internal_id))
}

fn chromium_window_size_arg(width: Option<f64>, height: Option<f64>) -> Option<String> {
    match (width, height) {
        (Some(w), Some(h)) => Some(format!("--window-size={},{}", w as i64, h as i64)),
        _ => None,
    }
}

fn build_chromium_exec(
    args: &BuildArgs,
    browser_path: &Path,
    profile_dir: Option<&Path>,
) -> String {
    let mut parts = vec![browser_path.display().to_string()];
    parts.push("--no-first-run".to_string());
    parts.push("--no-default-browser-check".to_string());
    parts.push(format!("--class={}", args.internal_id));
    parts.push(format!("--app={}", args.url));

    if let Some(profile_dir) = profile_dir {
        parts.push(format!("--user-data-dir={}", profile_dir.display()));
    }
    if args.fullscreen {
        parts.push("--start-fullscreen".to_string());
    }
    if let Some(window_size) = chromium_window_size_arg(args.width, args.height) {
        parts.push(window_size);
    }
    if let Some(user_agent) = &args.user_agent {
        parts.push(format!("--user-agent={}", user_agent));
    }
    if args.dark_mode {
        parts.push("--force-dark-mode".to_string());
    }

    desktop_exec_join(&parts)
}

fn print_generated_config(args: &BuildArgs) -> Result<()> {
    match args.backend {
        Backend::Tauri => {
            let config = build_tauri_config_value(args, &args.internal_id, false);
            println!(
                "{}",
                serde_json::to_string_pretty(&config)
                    .context("Failed to serialize generated config")?
            );
        }
        Backend::Chromium => {
            let browser = resolve_chromium_binary(args.browser_bin.as_deref())?;
            let profile_dir = chromium_profile_dir(&args.internal_id)?;
            let profile_dir = match args.profile_scope {
                ProfileScope::Isolated => Some(profile_dir),
                ProfileScope::Shared => None,
            };
            let exec = build_chromium_exec(args, &browser, profile_dir.as_deref());
            let value = json!({
                "backend": "chromium",
                "name": args.name,
                "id": args.internal_id,
                "url": args.url,
                "browser": browser.display().to_string(),
                "profileScope": profile_scope_str(args.profile_scope),
                "exec": exec,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&value)
                    .context("Failed to serialize generated config")?
            );
        }
    }
    Ok(())
}

fn print_build_plan(action: &str, args: &BuildArgs, existing_id: Option<&str>) {
    let id = existing_id.unwrap_or(&args.internal_id);
    println!("{} plan:", action);
    if let Some(id) = existing_id {
        println!("- Existing app ID: {}", id);
    }
    println!("- URL: {}", args.url);
    println!("- Name: {}", args.name);
    println!("- Internal ID: {}", id);
    match args.backend {
        Backend::Tauri => {
            println!("- Backend: tauri (system WebView)");
            println!("- Local build: cargo tauri build (temporary generated project)");
            println!("- Install targets: local binary + XDG icon + .desktop entry");
        }
        Backend::Chromium => {
            println!("- Backend: chromium (installed browser runtime)");
            println!("- Local build: none");
            println!("- Install targets: XDG icon + .desktop entry (+ optional isolated profile)");
            println!("- Profile scope: {}", profile_scope_str(args.profile_scope));
            if let Some(path) = &args.browser_bin {
                println!("- Browser binary: {}", path);
            } else {
                println!("- Browser binary: auto-detect from PATH");
            }
            if args.no_decorations {
                println!("- Note: --no-decorations is not guaranteed in Chromium app mode");
            }
        }
    }
    if args.fullscreen {
        println!("- Window: fullscreen enabled");
    }
    if args.no_decorations {
        println!("- Window: native decorations disabled");
    }
}

fn is_deskify_desktop_entry(content: &str) -> bool {
    content.contains("X-Deskify-Managed=true")
        || (content.contains("Categories=Network;WebBrowser;")
            && (content.contains(".local/bin/") || content.contains("deskify")))
}

fn write_rgba_png(img: DynamicImage, output_path: &Path) -> Result<()> {
    // Tauri expects PNG icons in RGBA format; many favicons are palette/LA/ICO etc.
    let rgba = img.to_rgba8();
    let rgba_img = DynamicImage::ImageRgba8(rgba);
    rgba_img
        .save_with_format(output_path, ImageFormat::Png)
        .with_context(|| format!("Failed to write RGBA PNG icon to {}", output_path.display()))?;
    Ok(())
}

fn download_icon_from_url(icon_url: &str, output_path: &Path) -> Result<bool> {
    let response = match ureq::get(icon_url).call() {
        Ok(resp) => resp,
        Err(_) => return Ok(false),
    };
    let mut bytes = Vec::new();
    if response.into_reader().read_to_end(&mut bytes).is_err() || bytes.is_empty() {
        return Ok(false);
    }
    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(_) => return Ok(false),
    };
    write_rgba_png(img, output_path)?;
    Ok(true)
}

fn extract_icon_candidates_from_html(base_url: &Url, html: &str) -> Vec<String> {
    let link_re = Regex::new(r#"(?is)<link\s+[^>]*rel\s*=\s*["'][^"']*icon[^"']*["'][^>]*href\s*=\s*["']([^"']+)["'][^>]*>"#).unwrap();
    let mut candidates = Vec::new();
    for cap in link_re.captures_iter(html) {
        if let Some(href) = cap.get(1)
            && let Ok(joined) = base_url.join(href.as_str())
        {
            let candidate = joined.to_string();
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

fn try_download_site_icon(website_url: &str, output_path: &Path) -> Result<bool> {
    let base_url = match Url::parse(website_url) {
        Ok(url) => url,
        Err(_) => return Ok(false),
    };

    let mut html = String::new();
    if let Ok(response) = ureq::get(website_url).call()
        && response.into_reader().read_to_string(&mut html).is_ok()
    {
        for icon_url in extract_icon_candidates_from_html(&base_url, &html) {
            if download_icon_from_url(&icon_url, output_path)? {
                return Ok(true);
            }
        }
    }

    if let Ok(favicon_url) = base_url.join("/favicon.ico")
        && download_icon_from_url(favicon_url.as_str(), output_path)?
    {
        return Ok(true);
    }

    Ok(false)
}

fn fetch_or_create_icon(
    website_url: &str,
    custom_icon: Option<&String>,
    output_path: &Path,
) -> Result<()> {
    if let Some(icon_path) = custom_icon {
        if Path::new(icon_path).exists() {
            let img = image::open(icon_path)
                .with_context(|| format!("Failed to read custom icon image from {}", icon_path))?;
            write_rgba_png(img, output_path)?;
            return Ok(());
        } else {
            eprintln!(
                "Warning: Custom icon path '{}' does not exist, falling back to downloaded icon.",
                icon_path
            );
        }
    }

    if try_download_site_icon(website_url, output_path)? {
        return Ok(());
    }

    if let Ok(parsed_url) = Url::parse(website_url)
        && let Some(host) = parsed_url.host_str()
    {
        println!("Falling back to Google favicon API for {}...", host);
        let api_url = format!("https://www.google.com/s2/favicons?domain={}&sz=128", host);
        if download_icon_from_url(&api_url, output_path)? {
            return Ok(());
        }
    }

    // Fallback to a transparent dummy icon
    let dummy_png: [u8; 67] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    fs::write(output_path, dummy_png).context("Failed to write dummy icon fallback")?;

    Ok(())
}

fn generate_project(args: &BuildArgs, project_dir: &Path) -> Result<()> {
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

    // 1. Cargo.toml
    let cargo_toml = r#"
[package]
name = "deskify-app"
version = "0.1.0"
description = "A native application wrapper"
authors = ["deskify"]
edition = "2021"

[dependencies]
tauri = { version = "2.0.0", features = [] }

[build-dependencies]
tauri-build = "2.0.0"
"#;
    fs::write(project_dir.join("Cargo.toml"), cargo_toml).context("Failed to write Cargo.toml")?;

    // 2. Build script (build.rs)
    let build_rs = r#"
fn main() {
    tauri_build::build()
}
"#;
    fs::write(project_dir.join("build.rs"), build_rs).context("Failed to write build.rs")?;

    // 3. src/main.rs
    let main_rs = r#"
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
"#;
    fs::write(src_dir.join("main.rs"), main_rs).context("Failed to write src/main.rs")?;

    // 4. tauri.conf.json
    let dist_dir = project_dir.join("dist");
    fs::create_dir_all(&dist_dir).context("Failed to create dist directory")?;
    fs::write(
        dist_dir.join("index.html"),
        "<!DOCTYPE html><html><body></body></html>",
    )
    .context("Failed to write dummy index.html")?;

    let icons_dir = project_dir.join("icons");
    fs::create_dir_all(&icons_dir).context("Failed to create icons directory")?;

    fetch_or_create_icon(&args.url, args.icon.as_ref(), &icons_dir.join("icon.png"))?;

    let tauri_conf = build_tauri_config_value(args, &args.internal_id, false);
    fs::write(
        project_dir.join("tauri.conf.json"),
        serde_json::to_string_pretty(&tauri_conf).context("Failed to serialize tauri.conf.json")?,
    )
    .context("Failed to write tauri.conf.json")?;

    Ok(())
}

fn build_project(project_dir: &Path) -> Result<PathBuf> {
    // 1. Check if tauri-cli is installed
    let tauri_check = Command::new("cargo")
        .arg("tauri")
        .arg("--version")
        .output()
        .context("Failed to execute `cargo`. Make sure rust/cargo is installed.")?;

    if !tauri_check.status.success() {
        return Err(anyhow!(
            "Tauri CLI is not installed or not working.\nPlease install it by running:\n  cargo install tauri-cli --version \"^2.0.0\""
        ));
    }

    println!("Building native app (this may take a few minutes the first time)...");

    let status = Command::new("cargo")
        .arg("tauri")
        .arg("build")
        .current_dir(project_dir)
        .status()
        .context("Failed to execute `cargo tauri build`")?;

    if !status.success() {
        return Err(anyhow!("Tauri build completed with an error"));
    }

    let bin_path = project_dir.join("target/release/deskify-app");
    if bin_path.exists() {
        println!("Successfully built binary at {:?}", bin_path);
        Ok(bin_path)
    } else {
        Err(anyhow!(
            "Binary not found after build (expected at {:?})",
            bin_path
        ))
    }
}

fn desktop_paths(internal_id: &str) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs (e.g. $HOME)"))?;
    let data_local_dir = base_dirs.data_local_dir(); // Usually ~/.local/share
    let executable_dir = base_dirs
        .executable_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            // Fallback for some systems where executable_dir is none
            base_dirs.home_dir().join(".local/bin")
        });

    let applications_dir = data_local_dir.join("applications");
    let icon_dir = data_local_dir.join("icons/hicolor/128x128/apps");
    let target_bin = executable_dir.join(internal_id);
    let target_icon = icon_dir.join(format!("{}.png", internal_id));
    let desktop_file_path = applications_dir.join(format!("{}.desktop", internal_id));

    Ok((
        target_bin,
        target_icon,
        desktop_file_path,
        data_local_dir.to_path_buf(),
    ))
}

fn install_tauri_app(args: &BuildArgs, bin_path: &Path, project_dir: &Path) -> Result<()> {
    let (target_bin, target_icon, desktop_file_path, _data_local_dir) =
        desktop_paths(&args.internal_id)?;
    let executable_dir = target_bin
        .parent()
        .ok_or_else(|| anyhow!("Invalid executable target path"))?;
    let icon_dir = target_icon
        .parent()
        .ok_or_else(|| anyhow!("Invalid icon target path"))?;
    let applications_dir = desktop_file_path
        .parent()
        .ok_or_else(|| anyhow!("Invalid applications target path"))?;

    // 1. Move binary
    fs::create_dir_all(executable_dir).context("Failed to create ~/.local/bin directory")?;
    fs::copy(bin_path, &target_bin).context("Failed to move binary to installation directory")?;

    // 2. Install Icon
    fs::create_dir_all(icon_dir).context("Failed to create icons directory")?;
    let source_icon = project_dir.join("icons/icon.png");
    if source_icon.exists() {
        let _ = fs::copy(&source_icon, &target_icon);
    }

    // 3. Create .desktop file
    fs::create_dir_all(applications_dir).context("Failed to create applications directory")?;

    let desktop_entry = format!(
        r#"
[Desktop Entry]
Version=1.0
Name={}
Exec={}
Icon={}
StartupWMClass={}
Terminal=false
Type=Application
Categories=Network;WebBrowser;
X-Deskify-Managed=true
X-Deskify-Backend=tauri
"#,
        args.name,
        target_bin.to_string_lossy(),
        args.internal_id, // Icon name
        args.internal_id  // Window class for wayland/x11 proper grouping
    );

    fs::write(&desktop_file_path, desktop_entry.trim()).context("Failed to write .desktop file")?;

    println!("App successfully installed!");
    println!(
        "You can now launch '{}' from your application menu.",
        args.name
    );

    Ok(())
}

fn install_chromium_app(args: &BuildArgs) -> Result<()> {
    let browser = resolve_chromium_binary(args.browser_bin.as_deref())?;
    let profile_dir = match args.profile_scope {
        ProfileScope::Isolated => Some(chromium_profile_dir(&args.internal_id)?),
        ProfileScope::Shared => None,
    };

    if matches!((args.width, args.height), (Some(_), None) | (None, Some(_))) {
        eprintln!(
            "Warning: Chromium backend requires both --width and --height for --window-size; ignoring partial size override."
        );
    }
    if args.no_decorations {
        eprintln!(
            "Warning: --no-decorations is not reliably supported in Chromium app mode and may be ignored."
        );
    }

    let (target_bin, target_icon, desktop_file_path, _data_local_dir) =
        desktop_paths(&args.internal_id)?;
    let icon_dir = target_icon
        .parent()
        .ok_or_else(|| anyhow!("Invalid icon target path"))?;
    let applications_dir = desktop_file_path
        .parent()
        .ok_or_else(|| anyhow!("Invalid applications target path"))?;

    fs::create_dir_all(icon_dir).context("Failed to create icons directory")?;
    fs::create_dir_all(applications_dir).context("Failed to create applications directory")?;
    if let Some(profile_dir) = &profile_dir {
        fs::create_dir_all(profile_dir).context("Failed to create Chromium profile directory")?;
    }

    let temp = tempdir().context("Failed to create temporary directory for icon fetch")?;
    let temp_icon = temp.path().join("icon.png");
    fetch_or_create_icon(&args.url, args.icon.as_ref(), &temp_icon)?;
    let _ = fs::copy(&temp_icon, &target_icon);

    let exec = build_chromium_exec(args, &browser, profile_dir.as_deref());
    let desktop_entry = format!(
        r#"
[Desktop Entry]
Version=1.0
Name={}
Exec={}
Icon={}
StartupWMClass={}
Terminal=false
Type=Application
Categories=Network;WebBrowser;
X-Deskify-Managed=true
X-Deskify-Backend=chromium
X-Deskify-URL={}
X-Deskify-ProfileScope={}
X-Deskify-Browser={}
"#,
        args.name,
        exec,
        args.internal_id,
        args.internal_id,
        args.url,
        profile_scope_str(args.profile_scope),
        browser.display()
    );
    fs::write(&desktop_file_path, desktop_entry.trim()).context("Failed to write .desktop file")?;

    if target_bin.exists() {
        println!(
            "Note: existing binary {} left in place (Chromium backend does not use it).",
            target_bin.display()
        );
    }

    println!("App successfully installed (Chromium backend)!");
    println!(
        "You can now launch '{}' from your application menu.",
        args.name
    );
    Ok(())
}

fn list_apps_with_options(verbose: bool) -> Result<()> {
    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;
    let applications_dir = base_dirs.data_local_dir().join("applications");

    println!("Installed Deskify Apps:");
    let mut found = false;

    if applications_dir.exists() {
        for entry in
            fs::read_dir(applications_dir).context("Failed to read applications directory")?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("desktop")
                && let Ok(content) = fs::read_to_string(&path)
                && is_deskify_desktop_entry(&content)
            {
                let name = content
                    .lines()
                    .find(|l| l.starts_with("Name="))
                    .and_then(|l| l.strip_prefix("Name="))
                    .unwrap_or("Unknown");

                let internal_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown");
                let backend = content
                    .lines()
                    .find(|l| l.starts_with("X-Deskify-Backend="))
                    .and_then(|l| l.strip_prefix("X-Deskify-Backend="))
                    .unwrap_or("legacy");

                if verbose && validate_remove_id(internal_id).is_ok() {
                    if let Some(cfg) = read_app_config(internal_id)? {
                        println!(
                            "- {} (Internal ID: {}, Backend: {}, URL: {})",
                            name,
                            internal_id,
                            backend_str(cfg.backend),
                            cfg.url
                        );
                    } else {
                        let url = content
                            .lines()
                            .find(|l| l.starts_with("X-Deskify-URL="))
                            .and_then(|l| l.strip_prefix("X-Deskify-URL="));
                        if let Some(url) = url {
                            println!(
                                "- {} (Internal ID: {}, Backend: {}, URL: {})",
                                name, internal_id, backend, url
                            );
                        } else {
                            println!(
                                "- {} (Internal ID: {}, Backend: {})",
                                name, internal_id, backend
                            );
                        }
                    }
                } else {
                    println!(
                        "- {} (Internal ID: {}, Backend: {})",
                        name, internal_id, backend
                    );
                }
                found = true;
            }
        }
    }

    if !found {
        println!("No apps found.");
    }

    Ok(())
}

fn read_desktop_entry_value(id: &str, key: &str) -> Result<Option<String>> {
    validate_remove_id(id)?;
    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;
    let desktop_path = base_dirs
        .data_local_dir()
        .join("applications")
        .join(format!("{}.desktop", id));
    if !desktop_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&desktop_path)
        .with_context(|| format!("Failed to read desktop entry {}", desktop_path.display()))?;
    if !is_deskify_desktop_entry(&content) {
        return Ok(None);
    }
    let prefix = format!("{}=", key);
    Ok(content
        .lines()
        .find(|line| line.starts_with(&prefix))
        .and_then(|line| line.strip_prefix(&prefix))
        .map(str::to_string))
}

fn read_installed_app_display_name(id: &str) -> Result<Option<String>> {
    validate_remove_id(id)?;
    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;
    let desktop_path = base_dirs
        .data_local_dir()
        .join("applications")
        .join(format!("{}.desktop", id));
    if !desktop_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&desktop_path)
        .with_context(|| format!("Failed to read desktop entry {}", desktop_path.display()))?;
    if !is_deskify_desktop_entry(&content) {
        return Ok(None);
    }
    let name = content
        .lines()
        .find(|line| line.starts_with("Name="))
        .and_then(|line| line.strip_prefix("Name="))
        .map(str::to_string);
    Ok(name)
}

fn run_doctor() -> Result<()> {
    let mut failed = false;

    let checks = vec![
        ("cargo", vec!["--version"], true),
        ("rustc", vec!["--version"], true),
        ("pkg-config", vec!["--version"], false),
    ];

    println!("Deskify doctor");
    println!("-------------");

    for (cmd, args, critical) in checks {
        match Command::new(cmd).args(args).output() {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("[ok] {} {}", cmd, version);
            }
            _ => {
                println!(
                    "[{}] {} not found or failed",
                    if critical { "fail" } else { "warn" },
                    cmd
                );
                if critical {
                    failed = true;
                }
            }
        }
    }

    match Command::new("cargo").args(["tauri", "--version"]).output() {
        Ok(output) if output.status.success() => {
            println!("[ok] {}", String::from_utf8_lossy(&output.stdout).trim());
        }
        _ => {
            println!(
                "[fail] cargo tauri not available (install with: cargo install tauri-cli --version \"^2.0.0\")"
            );
            failed = true;
        }
    }

    if let Ok(output) = Command::new("pkg-config")
        .args(["--modversion", "webkit2gtk-4.1"])
        .output()
    {
        if output.status.success() {
            println!(
                "[ok] webkit2gtk-4.1 {}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
        } else {
            println!("[warn] webkit2gtk-4.1 not found via pkg-config");
        }
    }

    match resolve_chromium_binary(None) {
        Ok(path) => println!("[ok] chromium browser found: {}", path.display()),
        Err(_) => println!("[warn] no chromium-based browser found in PATH"),
    }

    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;
    let executable_dir = base_dirs
        .executable_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| base_dirs.home_dir().join(".local/bin"));
    let applications_dir = base_dirs.data_local_dir().join("applications");
    let icons_dir = base_dirs
        .data_local_dir()
        .join("icons/hicolor/128x128/apps");
    println!("[info] executable dir: {}", executable_dir.display());
    println!("[info] applications dir: {}", applications_dir.display());
    println!("[info] icons dir: {}", icons_dir.display());
    if let Ok(exe) = env::current_exe() {
        println!("[info] deskify path: {}", exe.display());
    }

    if failed {
        Err(anyhow!("Doctor found critical issues"))
    } else {
        println!("Doctor completed: no critical issues detected.");
        Ok(())
    }
}

fn remove_app(safe_name: &str) -> Result<()> {
    remove_app_with_options(safe_name, true)
}

fn remove_app_with_options(safe_name: &str, remove_profile: bool) -> Result<()> {
    validate_remove_id(safe_name)?;

    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("Could not find system BaseDirs"))?;

    let executable_dir = base_dirs
        .executable_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| base_dirs.home_dir().join(".local/bin"));
    let data_local_dir = base_dirs.data_local_dir();

    // 1. Remove binary
    let binary_path = executable_dir.join(safe_name);
    if binary_path.exists() {
        fs::remove_file(&binary_path).context("Failed to remove binary")?;
        println!("Removed binary: {:?}", binary_path);
    } else {
        println!("Binary not found: {:?}", binary_path);
    }

    // 2. Remove .desktop file
    let desktop_path = data_local_dir
        .join("applications")
        .join(format!("{}.desktop", safe_name));
    if desktop_path.exists() {
        fs::remove_file(&desktop_path).context("Failed to remove .desktop file")?;
        println!("Removed desktop entry: {:?}", desktop_path);
    } else {
        println!("Desktop entry not found: {:?}", desktop_path);
    }

    // 3. Remove icon
    let icon_path = data_local_dir
        .join("icons/hicolor/128x128/apps")
        .join(format!("{}.png", safe_name));
    if icon_path.exists() {
        fs::remove_file(&icon_path).context("Failed to remove icon")?;
        println!("Removed icon: {:?}", icon_path);
    } else {
        println!("Icon not found: {:?}", icon_path);
    }

    // 4. Remove Chromium profile (optional)
    let profile_path = data_local_dir.join("deskify/profiles").join(safe_name);
    if remove_profile {
        if profile_path.exists() {
            fs::remove_dir_all(&profile_path).context("Failed to remove Chromium profile")?;
            println!("Removed Chromium profile: {:?}", profile_path);
        } else {
            println!("Chromium profile not found: {:?}", profile_path);
        }
    } else if profile_path.exists() {
        println!("Keeping Chromium profile: {:?}", profile_path);
    }

    println!("Successfully removed app '{}'.", safe_name);
    let _ = remove_app_config(safe_name);
    Ok(())
}

fn execute_build(args: &BuildArgs, dry_run: bool, print_config: bool) -> Result<()> {
    if print_config {
        print_generated_config(args)?;
        return Ok(());
    }
    if dry_run {
        print_build_plan("Build", args, None);
        return Ok(());
    }

    match args.backend {
        Backend::Tauri => {
            println!(
                "Generating native app '{}' for URL: {}",
                args.name, args.url
            );

            let dir = tempdir().context("Failed to create temporary directory for building")?;
            println!("Scaffolding Tauri project in {:?}", dir.path());

            generate_project(args, dir.path())?;

            let bin_path = match build_project(dir.path()) {
                Ok(path) => path,
                Err(e) => {
                    let _ = dir.keep();
                    return Err(e.context("Project architecture failed to compile"));
                }
            };

            install_tauri_app(args, &bin_path, dir.path())?;
            write_app_config_from_args(args)?;
            Ok(())
        }
        Backend::Chromium => {
            println!(
                "Installing Chromium app '{}' for URL: {}",
                args.name, args.url
            );
            install_chromium_app(args)?;
            write_app_config_from_args(args)?;
            Ok(())
        }
    }
}

fn execute_update(id: &str, args: &BuildArgs, dry_run: bool, print_config: bool) -> Result<()> {
    validate_remove_id(id)?;

    if print_config {
        print_generated_config(args)?;
        return Ok(());
    }
    if dry_run {
        print_build_plan("Update", args, Some(id));
        println!(
            "- Update action: remove existing app '{}' and reinstall",
            id
        );
        return Ok(());
    }

    println!("Updating app '{}' -> '{}' ({})", id, args.name, args.url);
    let remove_profile = match args.backend {
        Backend::Chromium => false,
        Backend::Tauri => true,
    };
    remove_app_with_options(id, remove_profile)?;
    execute_build(args, false, false)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            url,
            name,
            icon,
            fullscreen,
            no_decorations,
            user_agent,
            width,
            height,
            dark_mode,
            backend,
            browser_bin,
            profile_scope,
            print_config,
            dry_run,
        } => {
            let internal_id = sanitize_app_id(&name);
            let args = BuildArgs {
                url,
                name,
                internal_id,
                icon,
                fullscreen,
                no_decorations,
                user_agent,
                width,
                height,
                dark_mode,
                backend,
                browser_bin,
                profile_scope,
            };
            execute_build(&args, dry_run, print_config)?;
        }
        Commands::List { verbose } => {
            list_apps_with_options(verbose)?;
        }
        Commands::Doctor => {
            run_doctor()?;
        }
        Commands::Remove { id } => {
            remove_app(&id)?;
        }
        Commands::Update {
            id,
            url,
            name,
            icon,
            fullscreen,
            no_decorations,
            user_agent,
            width,
            height,
            dark_mode,
            backend,
            browser_bin,
            profile_scope,
            dry_run,
            print_config,
        } => {
            let existing_cfg = read_app_config(&id)?;
            let resolved_url = if let Some(url) = url {
                url
            } else if let Some(cfg) = &existing_cfg {
                cfg.url.clone()
            } else if let Some(url) = read_desktop_entry_value(&id, "X-Deskify-URL")? {
                url
            } else {
                return Err(anyhow!(
                    "Missing --url for update. Provide --url or rebuild once with a recent Deskify version so metadata can be persisted."
                ));
            };

            let resolved_name = if let Some(name) = name {
                name
            } else if let Some(cfg) = &existing_cfg {
                cfg.name.clone()
            } else {
                read_installed_app_display_name(&id)?.unwrap_or_else(|| id.clone())
            };

            let resolved_backend = if let Some(backend) = backend {
                backend
            } else if let Some(cfg) = &existing_cfg {
                cfg.backend
            } else if let Some(b) = read_desktop_entry_value(&id, "X-Deskify-Backend")? {
                match b.as_str() {
                    "chromium" => Backend::Chromium,
                    "tauri" => Backend::Tauri,
                    _ => Backend::Tauri,
                }
            } else {
                Backend::Tauri
            };

            let resolved_profile_scope = if let Some(scope) = profile_scope {
                scope
            } else if let Some(cfg) = &existing_cfg {
                cfg.profile_scope
            } else {
                ProfileScope::Isolated
            };

            let resolved_user_agent = if user_agent.is_some() {
                user_agent
            } else {
                existing_cfg.as_ref().and_then(|c| c.user_agent.clone())
            };
            let resolved_width = width.or_else(|| existing_cfg.as_ref().and_then(|c| c.width));
            let resolved_height = height.or_else(|| existing_cfg.as_ref().and_then(|c| c.height));
            let resolved_browser_bin =
                browser_bin.or_else(|| existing_cfg.as_ref().and_then(|c| c.browser_bin.clone()));

            let resolved_fullscreen =
                fullscreen.unwrap_or(existing_cfg.as_ref().map(|c| c.fullscreen).unwrap_or(false));
            let resolved_no_decorations = no_decorations.unwrap_or(
                existing_cfg
                    .as_ref()
                    .map(|c| c.no_decorations)
                    .unwrap_or(false),
            );
            let resolved_dark_mode =
                dark_mode.unwrap_or(existing_cfg.as_ref().map(|c| c.dark_mode).unwrap_or(false));

            let args = BuildArgs {
                url: resolved_url,
                name: resolved_name,
                internal_id: id.clone(),
                icon,
                fullscreen: resolved_fullscreen,
                no_decorations: resolved_no_decorations,
                user_agent: resolved_user_agent,
                width: resolved_width,
                height: resolved_height,
                dark_mode: resolved_dark_mode,
                backend: resolved_backend,
                browser_bin: resolved_browser_bin,
                profile_scope: resolved_profile_scope,
            };
            execute_update(&id, &args, dry_run, print_config)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        Backend, BuildArgs, ProfileScope, build_chromium_exec, chromium_window_size_arg,
        is_deskify_desktop_entry, sanitize_app_id, validate_remove_id,
    };
    use std::path::Path;

    #[test]
    fn sanitize_app_id_replaces_spaces_and_strips_symbols() {
        assert_eq!(sanitize_app_id("Chat GPT!"), "chat-gpt");
    }

    #[test]
    fn sanitize_app_id_falls_back_to_app_for_empty_result() {
        assert_eq!(sanitize_app_id("!!!"), "app");
    }

    #[test]
    fn validate_remove_id_accepts_safe_id() {
        assert!(validate_remove_id("chatgpt").is_ok());
    }

    #[test]
    fn validate_remove_id_rejects_path_traversal() {
        assert!(validate_remove_id("../x").is_err());
    }

    #[test]
    fn desktop_entry_detection_accepts_marker() {
        let content = "[Desktop Entry]\nName=ChatGPT\nX-Deskify-Managed=true\n";
        assert!(is_deskify_desktop_entry(content));
    }

    #[test]
    fn desktop_entry_detection_accepts_legacy_entries() {
        let content =
            "[Desktop Entry]\nExec=/home/user/.local/bin/chatgpt\nCategories=Network;WebBrowser;\n";
        assert!(is_deskify_desktop_entry(content));
    }

    #[test]
    fn desktop_entry_detection_rejects_unrelated_entries() {
        let content = "[Desktop Entry]\nExec=/usr/bin/firefox\nCategories=Network;WebBrowser;\n";
        assert!(!is_deskify_desktop_entry(content));
    }

    fn sample_build_args() -> BuildArgs {
        BuildArgs {
            url: "https://chat.com".to_string(),
            name: "Chat".to_string(),
            internal_id: "chat".to_string(),
            icon: None,
            fullscreen: false,
            no_decorations: false,
            user_agent: None,
            width: None,
            height: None,
            dark_mode: false,
            backend: Backend::Chromium,
            browser_bin: None,
            profile_scope: ProfileScope::Isolated,
        }
    }

    #[test]
    fn chromium_window_size_requires_both_dimensions() {
        assert_eq!(
            chromium_window_size_arg(Some(1200.0), Some(800.0)).as_deref(),
            Some("--window-size=1200,800")
        );
        assert!(chromium_window_size_arg(Some(1200.0), None).is_none());
        assert!(chromium_window_size_arg(None, Some(800.0)).is_none());
    }

    #[test]
    fn chromium_exec_contains_required_args() {
        let mut args = sample_build_args();
        args.fullscreen = true;
        args.dark_mode = true;
        args.user_agent = Some("UA Test".to_string());
        args.width = Some(1280.0);
        args.height = Some(720.0);
        let exec = build_chromium_exec(
            &args,
            Path::new("/usr/bin/chromium"),
            Some(Path::new("/tmp/profile")),
        );
        assert!(exec.contains("/usr/bin/chromium"));
        assert!(exec.contains("--app=https://chat.com"));
        assert!(exec.contains("--class=chat"));
        assert!(exec.contains("--user-data-dir=/tmp/profile"));
        assert!(exec.contains("--start-fullscreen"));
        assert!(exec.contains("--window-size=1280,720"));
        assert!(exec.contains("--force-dark-mode"));
        assert!(exec.contains("--user-agent="));
    }

    #[test]
    fn profile_scope_accepts_default_alias() {
        let scope: ProfileScope = serde_json::from_str("\"default\"").unwrap();
        assert_eq!(scope, ProfileScope::Isolated);
    }
}
