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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_app_id_replaces_spaces_and_strips_symbols() {
        assert_eq!(sanitize_app_id("Chat GPT!"), "chat-gpt");
    }

    #[test]
    fn sanitize_app_id_falls_back_to_app_for_empty_result() {
        assert_eq!(sanitize_app_id("!!!"), "app");
    }

    #[test]
    fn validate_remove_id_accepts_safe_id() {
        assert!(validate_remove_id("chatgpt").is_ok());
    }

    #[test]
    fn validate_remove_id_rejects_path_traversal() {
        assert!(validate_remove_id("../x").is_err());
    }

    #[test]
    fn desktop_entry_detection_accepts_marker() {
        let content = "[Desktop Entry]\nName=ChatGPT\nX-Deskify-Managed=true\n";
        assert!(is_deskify_desktop_entry(content));
    }

    #[test]
    fn desktop_entry_detection_accepts_legacy_entries() {
        let content =
            "[Desktop Entry]\nExec=/home/user/.local/bin/chatgpt\nCategories=Network;WebBrowser;\n";
        assert!(is_deskify_desktop_entry(content));
    }

    #[test]
    fn desktop_entry_detection_rejects_unrelated_entries() {
        let content = "[Desktop Entry]\nExec=/usr/bin/firefox\nCategories=Network;WebBrowser;\n";
        assert!(!is_deskify_desktop_entry(content));
    }

    #[test]
    fn validate_remove_id_accepts_simple() {
        assert!(validate_remove_id("chatgpt").is_ok());
        assert!(validate_remove_id("my-app-123").is_ok());
    }

    #[test]
    fn validate_remove_id_rejects_uppercase() {
        assert!(validate_remove_id("ChatGPT").is_err());
        assert!(validate_remove_id("MY-APP").is_err());
    }

    #[test]
    fn validate_remove_id_rejects_special_chars() {
        assert!(validate_remove_id("chat_gpt").is_err());
        assert!(validate_remove_id("chat.gpt").is_err());
        assert!(validate_remove_id("chat@gpt").is_err());
    }

    #[test]
    fn validate_remove_id_rejects_empty_after_sanitize() {
        assert!(validate_remove_id("!@#").is_err());
    }

    #[test]
    fn sanitize_app_id_handles_various_inputs() {
        assert_eq!(sanitize_app_id("AppName"), "appname");
        assert_eq!(sanitize_app_id("App Name 123"), "app-name-123");
        assert_eq!(sanitize_app_id("App!@#Name"), "appname");
        assert_eq!(sanitize_app_id("   spaces   "), "---spaces---");
        assert_eq!(sanitize_app_id("dash-name"), "dash-name");
    }
}
