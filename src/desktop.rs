use anyhow::Result;
use directories::BaseDirs;
use std::path::PathBuf;

pub fn desktop_exec_escape(arg: &str) -> String {
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

#[allow(dead_code)]
pub fn desktop_exec_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| desktop_exec_escape(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn desktop_paths(internal_id: &str) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Could not find system BaseDirs (e.g. $HOME)"))?;
    let data_local_dir = base_dirs.data_local_dir();
    let executable_dir = base_dirs
        .executable_dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| base_dirs.home_dir().join(".local/bin"));

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
