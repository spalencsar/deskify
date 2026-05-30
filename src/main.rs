mod app;
mod chromium;
mod desktop;
mod doctor;
mod icon;
mod install;
mod tauri;
mod types;
mod validation;
mod xdg;

use anyhow::{Context, Result};
use clap::Parser;
use tempfile::tempdir;

use types::{Backend, BuildArgs, Cli, Commands, ProfileScope};
use validation::sanitize_app_id;

fn execute_build(args: &BuildArgs, dry_run: bool, print_config: bool) -> Result<()> {
    if print_config {
        app::print_generated_config(args)?;
        return Ok(());
    }
    if dry_run {
        app::print_build_plan("Build", args, None);
        return Ok(());
    }

    match args.backend {
        Backend::Tauri => {
            app::warn_tauri_backend();

            println!(
                "Generating native app '{}' for URL: {}",
                args.name, args.url
            );

            let dir = tempdir().context("Failed to create temporary directory for building")?;
            println!("Scaffolding Tauri project in {:?}", dir.path());

            let temp_icon = dir.path().join("icon.png");
            icon::fetch_or_create_icon(&args.url, args.icon.as_ref(), &temp_icon)?;
            tauri::generate_project(args, dir.path(), &temp_icon)?;

            let bin_path = match tauri::build_project(dir.path()) {
                Ok(path) => path,
                Err(e) => {
                    let _ = dir.keep();
                    return Err(e.context("Project architecture failed to compile"));
                }
            };

            install::install_tauri_app(args, &bin_path, &temp_icon)?;
            app::write_app_config_from_args(args)?;
            Ok(())
        }
        Backend::Chromium => {
            println!(
                "Installing Chromium app '{}' for URL: {}",
                args.name, args.url
            );
            let icon_temp_dir =
                tempdir().context("Failed to create temporary directory for icon")?;
            let temp_icon = icon_temp_dir.path().join("icon.png");
            icon::fetch_or_create_icon(&args.url, args.icon.as_ref(), &temp_icon)?;
            install::install_chromium_app(args, &temp_icon)?;
            app::write_app_config_from_args(args)?;
            Ok(())
        }
    }
}

fn execute_update(id: &str, args: &BuildArgs, dry_run: bool, print_config: bool) -> Result<()> {
    validation::validate_remove_id(id)?;

    if print_config {
        app::print_generated_config(args)?;
        return Ok(());
    }
    if dry_run {
        app::print_build_plan("Update", args, Some(id));
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
    app::remove_app_with_options(id, remove_profile)?;
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
            app::list_apps_with_options(verbose)?;
        }
        Commands::Doctor => {
            doctor::run_doctor()?;
        }
        Commands::Remove { id } => {
            app::remove_app_with_options(&id, true)?;
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
            let existing_cfg = app::read_app_config(&id)?;
            let resolved_url = if let Some(url) = url {
                url
            } else if let Some(cfg) = &existing_cfg {
                cfg.url.clone()
            } else if let Some(url) = app::read_desktop_entry_value(&id, "X-Deskify-URL")? {
                url
            } else {
                return Err(anyhow::anyhow!(
                    "Missing --url for update. Provide --url or rebuild once with a recent Deskify version so metadata can be persisted."
                ));
            };

            let resolved_name = if let Some(name) = name {
                name
            } else if let Some(cfg) = &existing_cfg {
                cfg.name.clone()
            } else {
                app::read_installed_app_display_name(&id)?.unwrap_or_else(|| id.clone())
            };

            let resolved_backend = if let Some(backend) = backend {
                backend
            } else if let Some(cfg) = &existing_cfg {
                cfg.backend
            } else if let Some(b) = app::read_desktop_entry_value(&id, "X-Deskify-Backend")? {
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
