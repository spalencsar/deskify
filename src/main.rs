mod app;
mod chromium;
mod desktop;
mod doctor;
mod icon;
mod install;
mod tauri;
mod types;
mod validation;

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
            let dir = tempdir().context("Failed to create temporary directory for building")?;
            let temp_icon = dir.path().join("icon.png");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::{fs, os::unix::fs::PermissionsExt};
    use tempfile::tempdir;

    #[test]
    fn sanitize_app_id_replaces_spaces_and_strips_symbols() {
        assert_eq!(validation::sanitize_app_id("Chat GPT!"), "chat-gpt");
    }

    #[test]
    fn sanitize_app_id_falls_back_to_app_for_empty_result() {
        assert_eq!(validation::sanitize_app_id("!!!"), "app");
    }

    #[test]
    fn validate_remove_id_accepts_safe_id() {
        assert!(validation::validate_remove_id("chatgpt").is_ok());
    }

    #[test]
    fn validate_remove_id_rejects_path_traversal() {
        assert!(validation::validate_remove_id("../x").is_err());
    }

    #[test]
    fn desktop_entry_detection_accepts_marker() {
        let content = "[Desktop Entry]\nName=ChatGPT\nX-Deskify-Managed=true\n";
        assert!(validation::is_deskify_desktop_entry(content));
    }

    #[test]
    fn desktop_entry_detection_accepts_legacy_entries() {
        let content =
            "[Desktop Entry]\nExec=/home/user/.local/bin/chatgpt\nCategories=Network;WebBrowser;\n";
        assert!(validation::is_deskify_desktop_entry(content));
    }

    #[test]
    fn desktop_entry_detection_rejects_unrelated_entries() {
        let content = "[Desktop Entry]\nExec=/usr/bin/firefox\nCategories=Network;WebBrowser;\n";
        assert!(!validation::is_deskify_desktop_entry(content));
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
            chromium::chromium_window_size_arg(Some(1200.0), Some(800.0)).as_deref(),
            Some("--window-size=1200,800")
        );
        assert!(chromium::chromium_window_size_arg(Some(1200.0), None).is_none());
        assert!(chromium::chromium_window_size_arg(None, Some(800.0)).is_none());
    }

    #[test]
    fn chromium_exec_contains_required_args() {
        let mut args = sample_build_args();
        args.fullscreen = true;
        args.dark_mode = true;
        args.user_agent = Some("UA Test".to_string());
        args.width = Some(1280.0);
        args.height = Some(720.0);
        let exec = chromium::build_chromium_exec(
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

    #[test]
    fn resolve_chromium_binary_rejects_broken_explicit_path() {
        let dir = tempdir().unwrap();
        let script = dir.path().join("broken-browser");
        fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();

        let err = chromium::resolve_chromium_binary(Some(script.to_str().unwrap()))
            .err()
            .unwrap()
            .to_string();
        assert!(err.contains("failed to run") || err.contains("does not appear to be usable"));
    }

    #[test]
    fn desktop_exec_escape_empty_string() {
        assert_eq!(desktop::desktop_exec_escape(""), "\"\"");
    }

    #[test]
    fn desktop_exec_escape_plain_string() {
        assert_eq!(desktop::desktop_exec_escape("hello"), "hello");
    }

    #[test]
    fn desktop_exec_escape_with_whitespace() {
        assert_eq!(
            desktop::desktop_exec_escape("hello world"),
            "\"hello world\""
        );
    }

    #[test]
    fn desktop_exec_escape_with_quotes() {
        assert_eq!(
            desktop::desktop_exec_escape("hello\"world"),
            "\"hello\\\"world\""
        );
    }

    #[test]
    fn desktop_exec_escape_with_backslash() {
        assert_eq!(
            desktop::desktop_exec_escape("hello\\world"),
            "\"hello\\\\world\""
        );
    }

    #[test]
    fn desktop_exec_escape_mixed_special_chars() {
        assert_eq!(
            desktop::desktop_exec_escape("a b\"c\\d"),
            "\"a b\\\"c\\\\d\""
        );
    }

    #[test]
    fn desktop_exec_join_single_arg() {
        let args = vec!["/usr/bin/chromium".to_string()];
        assert_eq!(desktop::desktop_exec_join(&args), "/usr/bin/chromium");
    }

    #[test]
    fn desktop_exec_join_multiple_args() {
        let args = vec![
            "/usr/bin/chromium".to_string(),
            "--app=https://example.com".to_string(),
            "--class=myapp".to_string(),
        ];
        let result = desktop::desktop_exec_join(&args);
        assert!(result.contains("/usr/bin/chromium"));
        assert!(result.contains("--app=https://example.com"));
        assert!(result.contains("--class=myapp"));
    }

    #[test]
    fn desktop_exec_join_with_special_chars() {
        let args = vec![
            "/usr/bin/chromium".to_string(),
            "--user-agent=Mozilla/5.0 (X11; Linux x86_64)".to_string(),
        ];
        let result = desktop::desktop_exec_join(&args);
        assert!(result.contains("Mozilla/5.0"));
    }

    #[test]
    fn validate_remove_id_accepts_simple() {
        assert!(validation::validate_remove_id("chatgpt").is_ok());
        assert!(validation::validate_remove_id("my-app-123").is_ok());
    }

    #[test]
    fn validate_remove_id_rejects_uppercase() {
        assert!(validation::validate_remove_id("ChatGPT").is_err());
        assert!(validation::validate_remove_id("MY-APP").is_err());
    }

    #[test]
    fn validate_remove_id_rejects_special_chars() {
        assert!(validation::validate_remove_id("chat_gpt").is_err());
        assert!(validation::validate_remove_id("chat.gpt").is_err());
        assert!(validation::validate_remove_id("chat@gpt").is_err());
    }

    #[test]
    fn validate_remove_id_rejects_empty_after_sanitize() {
        assert!(validation::validate_remove_id("!@#").is_err());
    }

    #[test]
    fn backend_str_converts_correctly() {
        assert_eq!(chromium::backend_str(Backend::Tauri), "tauri");
        assert_eq!(chromium::backend_str(Backend::Chromium), "chromium");
    }

    #[test]
    fn profile_scope_str_converts_correctly() {
        assert_eq!(
            chromium::profile_scope_str(ProfileScope::Isolated),
            "isolated"
        );
        assert_eq!(chromium::profile_scope_str(ProfileScope::Shared), "shared");
    }

    #[test]
    fn chromium_candidates_list_non_empty() {
        let candidates = chromium::chromium_candidates();
        assert!(!candidates.is_empty());
        assert!(candidates.contains(&"chromium"));
        assert!(candidates.contains(&"google-chrome"));
        assert!(candidates.contains(&"brave-browser"));
    }

    #[test]
    fn sanitize_app_id_handles_various_inputs() {
        assert_eq!(validation::sanitize_app_id("AppName"), "appname");
        assert_eq!(validation::sanitize_app_id("App Name 123"), "app-name-123");
        assert_eq!(validation::sanitize_app_id("App!@#Name"), "appname");
        assert_eq!(validation::sanitize_app_id("   spaces   "), "---spaces---");
        assert_eq!(validation::sanitize_app_id("dash-name"), "dash-name");
    }

    #[test]
    fn app_config_roundtrip_serialization() {
        let cfg = types::DeskifyAppConfig {
            schema_version: 1,
            id: "test-app".to_string(),
            name: "Test App".to_string(),
            url: "https://example.com".to_string(),
            backend: Backend::Chromium,
            browser_bin: Some("/usr/bin/chromium".to_string()),
            profile_scope: ProfileScope::Isolated,
            fullscreen: true,
            no_decorations: false,
            user_agent: Some("TestUA".to_string()),
            width: Some(1024.0),
            height: Some(768.0),
            dark_mode: true,
        };

        let serialized = serde_json::to_string(&cfg).unwrap();
        let deserialized: types::DeskifyAppConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, cfg.id);
        assert_eq!(deserialized.name, cfg.name);
        assert_eq!(deserialized.url, cfg.url);
        assert_eq!(deserialized.backend, cfg.backend);
        assert_eq!(deserialized.fullscreen, cfg.fullscreen);
        assert_eq!(deserialized.dark_mode, cfg.dark_mode);
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

        let config = tauri::build_tauri_config_value(&args, "testapp", false);
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

        let config = tauri::build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"fullscreen\":true"));
    }

    #[test]
    fn tauri_config_no_decorations() {
        let mut args = sample_build_args();
        args.no_decorations = true;
        args.backend = Backend::Tauri;

        let config = tauri::build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"decorations\":false"));
    }

    #[test]
    fn tauri_config_dark_mode() {
        let mut args = sample_build_args();
        args.dark_mode = true;
        args.backend = Backend::Tauri;

        let config = tauri::build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"theme\":\"Dark\""));
    }

    #[test]
    fn tauri_config_user_agent() {
        let mut args = sample_build_args();
        args.user_agent = Some("Mozilla/5.0 CustomUA".to_string());
        args.backend = Backend::Tauri;

        let config = tauri::build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("Mozilla/5.0 CustomUA"));
    }

    #[test]
    fn tauri_config_window_dimensions() {
        let mut args = sample_build_args();
        args.width = Some(1920.0);
        args.height = Some(1080.0);
        args.backend = Backend::Tauri;

        let config = tauri::build_tauri_config_value(&args, "test", false);
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

        let config1 = tauri::build_tauri_config_value(&args1, "app1", false);
        let config2 = tauri::build_tauri_config_value(&args2, "app2", false);

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

        let config = tauri::build_tauri_config_value(&args, "test", false);
        let config_str = serde_json::to_string(&config).unwrap();

        assert!(config_str.contains("\"csp\":null"));
    }
}
