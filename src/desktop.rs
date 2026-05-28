pub use crate::xdg::desktop_paths;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_exec_escape_empty_string() {
        assert_eq!(desktop_exec_escape(""), "\"\"");
    }

    #[test]
    fn desktop_exec_escape_plain_string() {
        assert_eq!(desktop_exec_escape("hello"), "hello");
    }

    #[test]
    fn desktop_exec_escape_with_whitespace() {
        assert_eq!(desktop_exec_escape("hello world"), "\"hello world\"");
    }

    #[test]
    fn desktop_exec_escape_with_quotes() {
        assert_eq!(desktop_exec_escape("hello\"world"), "\"hello\\\"world\"");
    }

    #[test]
    fn desktop_exec_escape_with_backslash() {
        assert_eq!(desktop_exec_escape("hello\\world"), "\"hello\\\\world\"");
    }

    #[test]
    fn desktop_exec_escape_mixed_special_chars() {
        assert_eq!(desktop_exec_escape("a b\"c\\d"), "\"a b\\\"c\\\\d\"");
    }

    #[test]
    fn desktop_exec_join_single_arg() {
        let args = vec!["/usr/bin/chromium".to_string()];
        assert_eq!(desktop_exec_join(&args), "/usr/bin/chromium");
    }

    #[test]
    fn desktop_exec_join_multiple_args() {
        let args = vec![
            "/usr/bin/chromium".to_string(),
            "--app=https://example.com".to_string(),
            "--class=myapp".to_string(),
        ];
        let result = desktop_exec_join(&args);
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
        let result = desktop_exec_join(&args);
        assert!(result.contains("Mozilla/5.0"));
    }
}
