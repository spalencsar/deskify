use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::chromium::{
    backend_str, build_chromium_exec, profile_scope_str, resolve_chromium_binary,
};
use crate::types::{Backend, DeskifyAppConfig, ProfileScope};
use crate::validation::{is_deskify_desktop_entry, validate_remove_id};

/// Prints a warning when the user explicitly chooses the Tauri backend.
/// Chromium is now the recommended default.
pub(crate) fn warn_tauri_backend() {
    eprintln!(
        "Warning: You selected the Tauri backend (--backend tauri).\n\
         This is no longer the default and requires a full Tauri build environment.\n\
         Build times can take several minutes (or significantly longer on first builds).\n\n\
         For most users, the Chromium backend (now the default) is recommended.\n\
         You can omit --backend or explicitly use --backend chromium."
    );
}
use crate::xdg;

pub fn app_config_path(id: &str) -> Result<PathBuf> {
    validate_remove_id(id)?;
    Ok(xdg::app_config_dir()?.join(format!("{}.json", id)))
}

pub fn read_app_config(id: &str) -> Result<Option<DeskifyAppConfig>> {
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

pub fn write_app_config_from_args(args: &crate::types::BuildArgs) -> Result<()> {
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

pub fn remove_app_config(id: &str) -> Result<()> {
    let path = app_config_path(id)?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;
        println!("Removed app config: {:?}", path);
    }
    Ok(())
}

pub fn list_apps_with_options(verbose: bool) -> Result<()> {
    let applications_dir = xdg::applications_dir()?;

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

pub fn read_desktop_entry_value(id: &str, key: &str) -> Result<Option<String>> {
    validate_remove_id(id)?;
    let desktop_path = xdg::applications_dir()?.join(format!("{}.desktop", id));
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

pub fn read_installed_app_display_name(id: &str) -> Result<Option<String>> {
    validate_remove_id(id)?;
    let desktop_path = xdg::applications_dir()?.join(format!("{}.desktop", id));
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

pub fn remove_app_with_options(safe_name: &str, remove_profile: bool) -> Result<()> {
    validate_remove_id(safe_name)?;

    let executable_dir = xdg::executable_dir()?;
    let data_local_dir = xdg::data_local_dir()?;

    let binary_path = executable_dir.join(safe_name);
    if binary_path.exists() {
        fs::remove_file(&binary_path).context("Failed to remove binary")?;
        println!("Removed binary: {:?}", binary_path);
    } else {
        println!("Binary not found: {:?}", binary_path);
    }

    let desktop_path = data_local_dir
        .join("applications")
        .join(format!("{}.desktop", safe_name));
    if desktop_path.exists() {
        fs::remove_file(&desktop_path).context("Failed to remove .desktop file")?;
        println!("Removed desktop entry: {:?}", desktop_path);
    } else {
        println!("Desktop entry not found: {:?}", desktop_path);
    }

    let icon_path = data_local_dir
        .join("icons/hicolor/128x128/apps")
        .join(format!("{}.png", safe_name));
    if icon_path.exists() {
        fs::remove_file(&icon_path).context("Failed to remove icon")?;
        println!("Removed icon: {:?}", icon_path);
    } else {
        println!("Icon not found: {:?}", icon_path);
    }

    if let Ok(profile_path) = xdg::chromium_profile_dir(safe_name) {
        if profile_path.exists() {
            if remove_profile {
                let _ = fs::remove_dir_all(&profile_path);
                println!("Removed Chromium profile: {:?}", profile_path);
            } else {
                println!("Keeping Chromium profile: {:?}", profile_path);
            }
        } else if remove_profile {
            println!("Chromium profile not found: {:?}", profile_path);
        }
    }

    println!("Successfully removed app '{}'.", safe_name);
    let _ = remove_app_config(safe_name);
    Ok(())
}

pub fn print_generated_config(args: &crate::types::BuildArgs) -> Result<()> {
    match args.backend {
        Backend::Tauri => {
            let config = crate::tauri::build_tauri_config_value(args, &args.internal_id, false);
            println!(
                "{}",
                serde_json::to_string_pretty(&config)
                    .context("Failed to serialize generated config")?
            );
        }
        Backend::Chromium => {
            let browser = resolve_chromium_binary(args.browser_bin.as_deref())?;
            let profile_dir = xdg::chromium_profile_dir(&args.internal_id)?;
            let profile_dir = match args.profile_scope {
                ProfileScope::Isolated => Some(profile_dir),
                ProfileScope::Shared => None,
            };
            let exec = build_chromium_exec(args, &browser, profile_dir.as_deref());
            let value = serde_json::json!({
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

pub fn print_build_plan(action: &str, args: &crate::types::BuildArgs, existing_id: Option<&str>) {
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
            warn_tauri_backend();
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
