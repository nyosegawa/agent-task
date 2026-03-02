use std::env;
use std::process::Command;

/// Get project identifier as full path (git toplevel or cwd).
pub fn get_project() -> String {
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        && output.status.success()
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Extract "owner/repo" from various git remote URL formats.
pub fn extract_slug_from_url(url: &str) -> Option<String> {
    let path = if let Some(rest) = url.strip_prefix("git@") {
        rest.split(':').nth(1)?
    } else if let Some(rest) = url.strip_prefix("ssh://git@") {
        rest.split_once('/')?.1
    } else if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url.splitn(3, '/').nth(2)?;
        without_scheme.split_once('/')?.1
    } else {
        return None;
    };
    let slug = path.strip_suffix(".git").unwrap_or(path);
    if slug.contains('/') { Some(slug.to_string()) } else { None }
}

/// Normalize a project string for comparison.
/// Extracts slug from paths containing "github.com/owner/repo".
pub fn normalize_project(project: &str) -> String {
    // Already a slug like "owner/repo" (no path separators beyond one)
    let trimmed = project.trim().trim_end_matches('/');
    // Extract from full path: .../github.com/owner/repo
    if let Some(pos) = trimmed.to_lowercase().find("github.com/") {
        let after = &trimmed[pos + "github.com/".len()..];
        let parts: Vec<&str> = after.split('/').collect();
        if parts.len() >= 2 {
            return format!("{}/{}", parts[0], parts[1]).to_lowercase();
        }
    }
    trimmed.to_lowercase()
}

/// Shorten an absolute path to `parent/name` for display.
pub fn short_project(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(slash) => match trimmed[..slash].rfind('/') {
            Some(prev) => &trimmed[prev + 1..],
            None => trimmed,
        },
        None => trimmed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- short_project ---

    #[test]
    fn short_project_two_segments() {
        assert_eq!(
            short_project("/Users/sakasegawa/src/github.com/nyosegawa/agent-task"),
            "nyosegawa/agent-task"
        );
    }

    #[test]
    fn short_project_deep_path() {
        assert_eq!(short_project("/a/b/c/d/e"), "d/e");
    }

    #[test]
    fn short_project_single_segment() {
        assert_eq!(short_project("repo"), "repo");
    }

    #[test]
    fn short_project_trailing_slash() {
        assert_eq!(short_project("/Users/x/owner/repo/"), "owner/repo");
    }

    // --- extract_slug_from_url ---

    #[test]
    fn slug_from_ssh() {
        assert_eq!(
            extract_slug_from_url("git@github.com:nyosegawa/agent-task-web.git"),
            Some("nyosegawa/agent-task-web".into())
        );
    }

    #[test]
    fn slug_from_ssh_scheme() {
        assert_eq!(
            extract_slug_from_url("ssh://git@github.com/nyosegawa/agent-task.git"),
            Some("nyosegawa/agent-task".into())
        );
    }

    #[test]
    fn slug_from_https() {
        assert_eq!(
            extract_slug_from_url("https://github.com/owner/repo.git"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn slug_from_https_no_suffix() {
        assert_eq!(
            extract_slug_from_url("https://github.com/owner/repo"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn slug_invalid() {
        assert_eq!(extract_slug_from_url("not-a-url"), None);
        assert_eq!(extract_slug_from_url(""), None);
    }

    // --- normalize_project ---

    #[test]
    fn normalize_macos_path() {
        assert_eq!(
            normalize_project("/Users/sakasegawa/src/github.com/nyosegawa/agent-task-web"),
            "nyosegawa/agent-task-web"
        );
    }

    #[test]
    fn normalize_linux_path() {
        assert_eq!(
            normalize_project("/home/sakasegawa/src/github.com/nyosegawa/agent-task-web"),
            "nyosegawa/agent-task-web"
        );
    }

    #[test]
    fn normalize_slug_passthrough() {
        assert_eq!(
            normalize_project("nyosegawa/agent-task-web"),
            "nyosegawa/agent-task-web"
        );
    }

    #[test]
    fn normalize_macos_and_linux_match() {
        let mac = normalize_project("/Users/sakasegawa/src/github.com/nyosegawa/agent-task-web");
        let linux = normalize_project("/home/sakasegawa/src/github.com/nyosegawa/agent-task-web");
        assert_eq!(mac, linux);
    }

    #[test]
    fn normalize_non_github_path() {
        assert_eq!(
            normalize_project("/home/user/projects/my-app"),
            "/home/user/projects/my-app"
        );
    }
}
