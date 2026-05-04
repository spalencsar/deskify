use anyhow::{Result, anyhow};
use regex::Regex;

pub fn sanitize_app_id(name: &str) -> String {
    let re = Regex::new(r"[^a-z0-9-]").unwrap();
    let lower_name = name.to_lowercase().replace(' ', "-");
    let sanitized = re.replace_all(&lower_name, "");
    if sanitized.is_empty() {
        "app".to_string()
    } else {
        sanitized.to_string()
    }
}

pub fn validate_remove_id(id: &str) -> Result<()> {
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

pub fn is_deskify_desktop_entry(content: &str) -> bool {
    content.contains("X-Deskify-Managed=true")
        || (content.contains("Categories=Network;WebBrowser;")
            && (content.contains(".local/bin/") || content.contains("deskify")))
}
