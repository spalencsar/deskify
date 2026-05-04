use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::chromium::{
    build_chromium_exec, chromium_profile_dir, profile_scope_str, resolve_chromium_binary,
};
use crate::desktop::desktop_paths;
use crate::types::{BuildArgs, ProfileScope};

pub fn install_tauri_app(args: &BuildArgs, bin_path: &Path, icon_path: &Path) -> Result<()> {
    let (target_bin, target_icon, desktop_file_path, _data_local_dir) =
        desktop_paths(&args.internal_id)?;
    let executable_dir = target_bin
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid executable target path"))?;
    let icon_dir = target_icon
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid icon target path"))?;
    let applications_dir = desktop_file_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid applications target path"))?;

    fs::create_dir_all(executable_dir).context("Failed to create ~/.local/bin directory")?;
    fs::copy(bin_path, &target_bin).context("Failed to move binary to installation directory")?;

    fs::create_dir_all(icon_dir).context("Failed to create icons directory")?;
    if icon_path.exists() {
        let _ = fs::copy(icon_path, &target_icon);
    }

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
        args.internal_id,
        args.internal_id
    );

    fs::write(&desktop_file_path, desktop_entry.trim()).context("Failed to write .desktop file")?;

    println!("App successfully installed!");
    println!(
        "You can now launch '{}' from your application menu.",
        args.name
    );

    Ok(())
}

pub fn install_chromium_app(args: &BuildArgs, icon_path: &Path) -> Result<()> {
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
        .ok_or_else(|| anyhow::anyhow!("Invalid icon target path"))?;
    let applications_dir = desktop_file_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid applications target path"))?;

    fs::create_dir_all(icon_dir).context("Failed to create icons directory")?;
    fs::create_dir_all(applications_dir).context("Failed to create applications directory")?;
    if let Some(profile_dir) = &profile_dir {
        fs::create_dir_all(profile_dir).context("Failed to create Chromium profile directory")?;
    }

    let _ = fs::copy(icon_path, &target_icon);

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
