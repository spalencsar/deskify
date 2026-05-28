use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::BuildArgs;

pub fn build_tauri_config_value(
    args: &BuildArgs,
    safe_identifier: &str,
    bundle_active: bool,
) -> Value {
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

pub fn generate_project(args: &BuildArgs, project_dir: &Path, icon_path: &Path) -> Result<()> {
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

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

    let build_rs = r#"
fn main() {
    tauri_build::build()
}
"#;
    fs::write(project_dir.join("build.rs"), build_rs).context("Failed to write build.rs")?;

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

    let dist_dir = project_dir.join("dist");
    fs::create_dir_all(&dist_dir).context("Failed to create dist directory")?;
    fs::write(
        dist_dir.join("index.html"),
        "<!DOCTYPE html><html><body></body></html>",
    )
    .context("Failed to write dummy index.html")?;

    let icons_dir = project_dir.join("icons");
    fs::create_dir_all(&icons_dir).context("Failed to create icons directory")?;
    fs::copy(icon_path, icons_dir.join("icon.png")).context("Failed to copy icon")?;

    let tauri_conf = build_tauri_config_value(args, &args.internal_id, false);
    fs::write(
        project_dir.join("tauri.conf.json"),
        serde_json::to_string_pretty(&tauri_conf).context("Failed to serialize tauri.conf.json")?,
    )
    .context("Failed to write tauri.conf.json")?;

    Ok(())
}

pub fn build_project(project_dir: &Path) -> Result<PathBuf> {
    let tauri_check = Command::new("cargo")
        .arg("tauri")
        .arg("--version")
        .output()
        .context("Failed to execute `cargo`. Make sure rust/cargo is installed.")?;

    if !tauri_check.status.success() {
        return Err(anyhow::anyhow!(
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
        return Err(anyhow::anyhow!("Tauri build completed with an error"));
    }

    let bin_path = project_dir.join("target/release/deskify-app");
    if bin_path.exists() {
        println!("Successfully built binary at {:?}", bin_path);
        Ok(bin_path)
    } else {
        Err(anyhow::anyhow!(
            "Binary not found after build (expected at {:?})",
            bin_path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Backend, BuildArgs, ProfileScope};

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
            backend: Backend::Tauri,
            browser_bin: None,
            profile_scope: ProfileScope::Isolated,
        }
    }

    #[test]
    fn tauri_config_basic_generation() {
        let args = BuildArgs {
            url: "https://example.com".to_string(),
            name: "TestApp".to_string(),
            internal_id: "testapp".to_string(),
            icon: None,
            fullscreen: false,
            no_decorations: false,
            user_agent: None,
            width: None,
            height: None,
            dark_mode: false,
            backend: Backend::Tauri,
            browser_bin: None,
            profile_scope: ProfileScope::Isolated,
        };

        let config = build_tauri_config_value(&args, "testapp", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("TestApp"));
        assert!(config_str.contains("https://example.com"));
        assert!(config_str.contains("com.deskify.testapp"));
    }

    #[test]
    fn tauri_config_fullscreen_enabled() {
        let mut args = sample_build_args();
        args.fullscreen = true;
        args.backend = Backend::Tauri;

        let config = build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"fullscreen\":true"));
    }

    #[test]
    fn tauri_config_no_decorations() {
        let mut args = sample_build_args();
        args.no_decorations = true;
        args.backend = Backend::Tauri;

        let config = build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"decorations\":false"));
    }

    #[test]
    fn tauri_config_dark_mode() {
        let mut args = sample_build_args();
        args.dark_mode = true;
        args.backend = Backend::Tauri;

        let config = build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"theme\":\"Dark\""));
    }

    #[test]
    fn tauri_config_user_agent() {
        let mut args = sample_build_args();
        args.user_agent = Some("Mozilla/5.0 CustomUA".to_string());
        args.backend = Backend::Tauri;

        let config = build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("Mozilla/5.0 CustomUA"));
    }

    #[test]
    fn tauri_config_window_dimensions() {
        let mut args = sample_build_args();
        args.width = Some(1920.0);
        args.height = Some(1080.0);
        args.backend = Backend::Tauri;

        let config = build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"width\":1920"));
        assert!(config_str.contains("\"height\":1080"));
    }

    #[test]
    fn tauri_config_identifier_differs_by_id() {
        let args1 = BuildArgs {
            url: "https://a.com".to_string(),
            name: "App1".to_string(),
            internal_id: "app1".to_string(),
            icon: None,
            fullscreen: false,
            no_decorations: false,
            user_agent: None,
            width: None,
            height: None,
            dark_mode: false,
            backend: Backend::Tauri,
            browser_bin: None,
            profile_scope: ProfileScope::Isolated,
        };

        let args2 = BuildArgs {
            url: "https://b.com".to_string(),
            name: "App2".to_string(),
            internal_id: "app2".to_string(),
            icon: None,
            fullscreen: false,
            no_decorations: false,
            user_agent: None,
            width: None,
            height: None,
            dark_mode: false,
            backend: Backend::Tauri,
            browser_bin: None,
            profile_scope: ProfileScope::Isolated,
        };

        let config1 = build_tauri_config_value(&args1, "app1", false);
        let config2 = build_tauri_config_value(&args2, "app2", false);

        let s1 = serde_json::to_string(&config1).unwrap();
        let s2 = serde_json::to_string(&config2).unwrap();

        assert!(s1.contains("com.deskify.app1"));
        assert!(s2.contains("com.deskify.app2"));
    }

    #[test]
    fn tauri_config_csp_null() {
        let args = BuildArgs {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            internal_id: "test".to_string(),
            icon: None,
            fullscreen: false,
            no_decorations: false,
            user_agent: None,
            width: None,
            height: None,
            dark_mode: false,
            backend: Backend::Tauri,
            browser_bin: None,
            profile_scope: ProfileScope::Isolated,
        };

        let config = build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"csp\":null"));
    }
}
