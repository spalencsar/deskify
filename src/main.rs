use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use directories::BaseDirs;
use image::{DynamicImage, ImageFormat};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use url::Url;

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
    },
    /// List all installed apps created by deskify
    List,
    /// Remove a specific app by its internal ID
    Remove {
        /// The safe name/ID of the app (e.g., "youtube")
        id: String,
    },
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

    if let Ok(parsed_url) = Url::parse(website_url)
        && let Some(host) = parsed_url.host_str()
    {
        println!("Downloading icon for {}...", host);
        let api_url = format!("https://www.google.com/s2/favicons?domain={}&sz=128", host);
        if let Ok(response) = ureq::get(&api_url).call() {
            let mut bytes = Vec::new();
            if response.into_reader().read_to_end(&mut bytes).is_ok() && !bytes.is_empty() {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    write_rgba_png(img, output_path)?;
                    return Ok(());
                } else {
                    eprintln!(
                        "Warning: Downloaded favicon could not be decoded as an image, falling back to dummy icon."
                    );
                }
            }
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

#[allow(clippy::too_many_arguments)]
fn generate_project(
    url: &str,
    name: &str,
    icon: Option<&String>,
    fullscreen: bool,
    no_decorations: bool,
    user_agent: Option<&String>,
    width: Option<f64>,
    height: Option<f64>,
    dark_mode: bool,
    project_dir: &Path,
) -> Result<()> {
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
    let safe_identifier = sanitize_app_id(name);

    let dist_dir = project_dir.join("dist");
    fs::create_dir_all(&dist_dir).context("Failed to create dist directory")?;
    fs::write(
        dist_dir.join("index.html"),
        "<!DOCTYPE html><html><body></body></html>",
    )
    .context("Failed to write dummy index.html")?;

    let icons_dir = project_dir.join("icons");
    fs::create_dir_all(&icons_dir).context("Failed to create icons directory")?;

    fetch_or_create_icon(url, icon, &icons_dir.join("icon.png"))?;

    let mut window_config = Map::<String, Value>::new();
    window_config.insert("title".to_string(), json!(name));
    window_config.insert("url".to_string(), json!(url));

    if fullscreen {
        window_config.insert("fullscreen".to_string(), json!(true));
    }
    if no_decorations {
        window_config.insert("decorations".to_string(), json!(false));
    }
    if let Some(ua) = user_agent {
        window_config.insert("userAgent".to_string(), json!(ua));
    }
    if let Some(w) = width {
        window_config.insert("width".to_string(), json!(w));
    }
    if let Some(h) = height {
        window_config.insert("height".to_string(), json!(h));
    }
    if dark_mode {
        window_config.insert("theme".to_string(), json!("Dark"));
    }

    let tauri_conf = json!({
        "$schema": "https://schema.tauri.app/config/2",
        "productName": name,
        "version": "0.1.0",
        "identifier": format!("com.deskify.{}", safe_identifier),
        "build": {
            "frontendDist": "dist"
        },
        "app": {
            "windows": [Value::Object(window_config)],
            "security": {
                "csp": null
            }
        },
        "bundle": {
            "active": false,
            "targets": "all",
            "icon": ["icons/icon.png"]
        }
    });
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

fn install_app(name: &str, bin_path: &Path, project_dir: &Path) -> Result<()> {
    let safe_name = sanitize_app_id(name);

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

    // 1. Move binary
    fs::create_dir_all(&executable_dir).context("Failed to create ~/.local/bin directory")?;
    let target_bin = executable_dir.join(&safe_name);
    fs::copy(bin_path, &target_bin).context("Failed to move binary to installation directory")?;

    // 2. Install Icon
    let icon_dir = data_local_dir.join("icons/hicolor/128x128/apps");
    fs::create_dir_all(&icon_dir).context("Failed to create icons directory")?;
    let target_icon = icon_dir.join(format!("{}.png", safe_name));
    let source_icon = project_dir.join("icons/icon.png");
    if source_icon.exists() {
        let _ = fs::copy(&source_icon, &target_icon);
    }

    // 3. Create .desktop file
    let applications_dir = data_local_dir.join("applications");
    fs::create_dir_all(&applications_dir).context("Failed to create applications directory")?;

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
"#,
        name,
        target_bin.to_string_lossy(),
        safe_name, // Icon name
        safe_name  // Window class for wayland/x11 proper grouping
    );

    let desktop_file_path = applications_dir.join(format!("{}.desktop", safe_name));
    fs::write(&desktop_file_path, desktop_entry.trim()).context("Failed to write .desktop file")?;

    println!("App successfully installed!");
    println!("You can now launch '{}' from your application menu.", name);

    Ok(())
}

fn list_apps() -> Result<()> {
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

                let exec = content
                    .lines()
                    .find(|l| l.starts_with("Exec="))
                    .and_then(|l| l.strip_prefix("Exec="))
                    .unwrap_or("Unknown");

                let bin_name = Path::new(exec)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");

                println!("- {} (Internal ID: {})", name, bin_name);
                found = true;
            }
        }
    }

    if !found {
        println!("No apps found.");
    }

    Ok(())
}

fn remove_app(safe_name: &str) -> Result<()> {
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

    println!("Successfully removed app '{}'.", safe_name);
    Ok(())
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
        } => {
            println!("Generating native app '{}' for URL: {}", name, url);

            let dir = tempdir().context("Failed to create temporary directory for building")?;
            println!("Scaffolding Tauri project in {:?}", dir.path());

            generate_project(
                &url,
                &name,
                icon.as_ref(),
                fullscreen,
                no_decorations,
                user_agent.as_ref(),
                width,
                height,
                dark_mode,
                dir.path(),
            )?;

            let bin_path = match build_project(dir.path()) {
                Ok(path) => path,
                Err(e) => {
                    let _ = dir.keep();
                    return Err(e.context("Project architecture failed to compile"));
                }
            };

            install_app(&name, &bin_path, dir.path())?;
        }
        Commands::List => {
            list_apps()?;
        }
        Commands::Remove { id } => {
            remove_app(&id)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_deskify_desktop_entry, sanitize_app_id, validate_remove_id};

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
}
