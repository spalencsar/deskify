use clap::{Parser, Subcommand};

#[derive(
    clap::ValueEnum, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Tauri,
    Chromium,
}

#[derive(
    clap::ValueEnum, serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq,
)]
#[serde(rename_all = "lowercase")]
pub enum ProfileScope {
    #[serde(alias = "default")]
    Isolated,
    Shared,
}

#[derive(Debug, Clone)]
pub struct BuildArgs {
    pub url: String,
    pub name: String,
    pub internal_id: String,
    pub icon: Option<String>,
    pub fullscreen: bool,
    pub no_decorations: bool,
    pub user_agent: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub dark_mode: bool,
    pub backend: Backend,
    pub browser_bin: Option<String>,
    pub profile_scope: ProfileScope,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DeskifyAppConfig {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub url: String,
    pub backend: Backend,
    pub browser_bin: Option<String>,
    pub profile_scope: ProfileScope,
    pub fullscreen: bool,
    pub no_decorations: bool,
    pub user_agent: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub dark_mode: bool,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Build {
        #[arg(short, long)]
        url: String,

        #[arg(short, long)]
        name: String,

        #[arg(short, long)]
        icon: Option<String>,

        #[arg(short, long)]
        fullscreen: bool,

        #[arg(long)]
        no_decorations: bool,

        #[arg(short = 'A', long)]
        user_agent: Option<String>,

        #[arg(short = 'W', long)]
        width: Option<f64>,

        #[arg(short = 'H', long)]
        height: Option<f64>,

        #[arg(short, long)]
        dark_mode: bool,

        #[arg(long, value_enum, default_value_t = Backend::Chromium)]
        backend: Backend,

        #[arg(long)]
        browser_bin: Option<String>,

        #[arg(long, value_enum, default_value_t = ProfileScope::Isolated)]
        profile_scope: ProfileScope,

        #[arg(long)]
        print_config: bool,

        #[arg(long)]
        dry_run: bool,
    },
    List {
        #[arg(long)]
        verbose: bool,
    },
    Doctor,
    Remove {
        id: String,
    },
    Update {
        id: String,

        #[arg(short, long)]
        url: Option<String>,

        #[arg(short, long)]
        name: Option<String>,

        #[arg(short, long)]
        icon: Option<String>,

        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        fullscreen: Option<bool>,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        no_fullscreen: bool,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        no_decorations: Option<bool>,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        decorations: bool,

        #[arg(short = 'A', long)]
        user_agent: Option<String>,

        #[arg(short = 'W', long)]
        width: Option<f64>,

        #[arg(short = 'H', long)]
        height: Option<f64>,

        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        dark_mode: Option<bool>,

        #[arg(long, action = clap::ArgAction::SetTrue)]
        light_mode: bool,

        #[arg(long, value_enum)]
        backend: Option<Backend>,

        #[arg(long)]
        browser_bin: Option<String>,

        #[arg(long, value_enum)]
        profile_scope: Option<ProfileScope>,

        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        print_config: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_scope_accepts_default_alias() {
        let scope: ProfileScope = serde_json::from_str("\"default\"").unwrap();
        assert_eq!(scope, ProfileScope::Isolated);
    }

    #[test]
    fn app_config_roundtrip_serialization() {
        let cfg = DeskifyAppConfig {
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
        let deserialized: DeskifyAppConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.id, cfg.id);
        assert_eq!(deserialized.name, cfg.name);
        assert_eq!(deserialized.url, cfg.url);
        assert_eq!(deserialized.backend, cfg.backend);
        assert_eq!(deserialized.fullscreen, cfg.fullscreen);
        assert_eq!(deserialized.dark_mode, cfg.dark_mode);
    }

    #[test]
    fn update_accepts_disable_flags() {
        let cli = Cli::try_parse_from([
            "deskify",
            "update",
            "chat",
            "--no-fullscreen",
            "--decorations",
            "--light-mode",
        ])
        .unwrap();

        match cli.command {
            Commands::Update {
                no_fullscreen,
                decorations,
                light_mode,
                ..
            } => {
                assert!(no_fullscreen);
                assert!(decorations);
                assert!(light_mode);
            }
            _ => panic!("expected update command"),
        }
    }
}
