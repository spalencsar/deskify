// src/xdg.rs
//
// Centralized handling of XDG Base Directories for deskify.
// This module exists to avoid repeating BaseDirs::new() + error handling
// and XDG path construction all over the codebase.

use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::PathBuf;

use crate::validation::validate_remove_id;

/// Returns a BaseDirs handle or a clear error if the environment
/// does not provide standard directories (e.g. no $HOME).
pub fn base_dirs() -> Result<BaseDirs> {
    BaseDirs::new().context("Could not determine base directories. Is $HOME set?")
}

// -----------------------------------------------------------------------------
// Low-level XDG directories
// -----------------------------------------------------------------------------

/// Returns $XDG_DATA_HOME or the equivalent default (~/.local/share).
pub fn data_local_dir() -> Result<PathBuf> {
    Ok(base_dirs()?.data_local_dir().to_path_buf())
}

/// Returns the directory where user executables should be placed.
/// Falls back to ~/.local/bin if $XDG_BIN_HOME is not set.
pub fn executable_dir() -> Result<PathBuf> {
    let dirs = base_dirs()?;
    Ok(dirs
        .executable_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| dirs.home_dir().join(".local/bin")))
}

// -----------------------------------------------------------------------------
// Deskify-specific directories
// -----------------------------------------------------------------------------

/// ~/.local/share/deskify
pub fn deskify_data_dir() -> Result<PathBuf> {
    Ok(data_local_dir()?.join("deskify"))
}

/// ~/.local/share/deskify/apps
///
/// This function is part of the public XDG path API and may be used more
/// heavily in the future. It is currently lightly used.
#[allow(dead_code)]
pub fn app_config_dir() -> Result<PathBuf> {
    Ok(deskify_data_dir()?.join("apps"))
}

/// ~/.local/share/deskify/profiles
pub fn chromium_profiles_dir() -> Result<PathBuf> {
    Ok(deskify_data_dir()?.join("profiles"))
}

/// ~/.local/share/deskify/profiles/<id>
pub fn chromium_profile_dir(id: &str) -> Result<PathBuf> {
    validate_remove_id(id)?;
    Ok(chromium_profiles_dir()?.join(id))
}

// -----------------------------------------------------------------------------
// Standard XDG directories used for desktop integration
// -----------------------------------------------------------------------------

/// ~/.local/share/applications
pub fn applications_dir() -> Result<PathBuf> {
    Ok(data_local_dir()?.join("applications"))
}

/// ~/.local/share/icons/hicolor/128x128/apps
///
/// This function is part of the public XDG path API and may be used more
/// heavily in the future. It is currently lightly used.
#[allow(dead_code)]
pub fn icons_dir() -> Result<PathBuf> {
    Ok(data_local_dir()?.join("icons/hicolor/128x128/apps"))
}

// -----------------------------------------------------------------------------
// Combined helper for app installation/removal
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DesktopPaths {
    pub binary: PathBuf,
    pub icon: PathBuf,
    pub desktop_file: PathBuf,
    /// Internal for now. May be made public later if needed by callers.
    #[allow(dead_code)]
    data_local_dir: PathBuf,
}

/// Returns all relevant filesystem paths for a given Deskify app.
pub fn desktop_paths(internal_id: &str) -> Result<DesktopPaths> {
    validate_remove_id(internal_id)?;

    let exec_dir = executable_dir()?;
    let apps_dir = applications_dir()?;
    let icon_dir = icons_dir()?;
    let data_dir = data_local_dir()?;

    Ok(DesktopPaths {
        binary: exec_dir.join(internal_id),
        icon: icon_dir.join(format!("{}.png", internal_id)),
        desktop_file: apps_dir.join(format!("{}.desktop", internal_id)),
        data_local_dir: data_dir,
    })
}
