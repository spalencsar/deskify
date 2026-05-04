use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::desktop::desktop_exec_escape;
use crate::types::{BuildArgs, ProfileScope};
use crate::validation::validate_remove_id;
use directories::BaseDirs;

pub fn chromium_candidates() -> &'static [&'static str] {
    &[
        "google-chrome",
        "chromium",
        "chromium-browser",
        "brave-browser",
        "vivaldi",
        "microsoft-edge",
    ]
}

pub fn find_binary_in_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn chromium_binary_works(path: &Path) -> Result<()> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .with_context(|| format!("Failed to execute browser binary {}", path.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit code {}", output.status)
    };
    Err(anyhow::anyhow!(
        "Browser binary exists but failed to run: {} ({})",
        path.display(),
        details
    ))
}

pub fn resolve_chromium_binary(explicit: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            chromium_binary_works(&candidate).with_context(|| {
                "The specified browser binary does not appear to be usable. If it's a wrapper script, ensure the underlying browser exists."
            })?;
            return Ok(candidate);
        }
        return Err(anyhow::anyhow!(
            "Specified browser binary does not exist or is not a file: {}",
            candidate.display()
        ));
    }

    for candidate in chromium_candidates() {
        if let Some(path) = find_binary_in_path(candidate)
            && chromium_binary_works(&path).is_ok()
        {
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!(
        "No Chromium-based browser found in PATH. Install Chromium/Chrome/Brave or use --browser-bin /path/to/browser."
    ))
}

pub fn chromium_profile_dir(internal_id: &str) -> Result<PathBuf> {
    validate_remove_id(internal_id)?;
    let base_dirs =
        BaseDirs::new().ok_or_else(|| anyhow::anyhow!("Could not find system BaseDirs"))?;
    Ok(base_dirs
        .data_local_dir()
        .join("deskify")
        .join("profiles")
        .join(internal_id))
}

pub fn chromium_window_size_arg(width: Option<f64>, height: Option<f64>) -> Option<String> {
    match (width, height) {
        (Some(w), Some(h)) => Some(format!("--window-size={},{}", w as i64, h as i64)),
        _ => None,
    }
}

pub fn build_chromium_exec(
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

    let escaped: Vec<String> = parts.iter().map(|s| desktop_exec_escape(s)).collect();
    escaped.join(" ")
}

pub fn profile_scope_str(scope: ProfileScope) -> &'static str {
    match scope {
        ProfileScope::Isolated => "isolated",
        ProfileScope::Shared => "shared",
    }
}

pub fn backend_str(backend: crate::types::Backend) -> &'static str {
    match backend {
        crate::types::Backend::Tauri => "tauri",
        crate::types::Backend::Chromium => "chromium",
    }
}
