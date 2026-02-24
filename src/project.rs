use std::env;
use std::process::Command;

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

    #[test]
    fn short_project_two_segments() {
        assert_eq!(
            short_project("/Users/sakasegawa/src/github.com/nyosegawa/agent-task"),
            "nyosegawa/agent-task"
        );
    }

    #[test]
    fn short_project_deep_path() {
        assert_eq!(
            short_project("/a/b/c/d/e"),
            "d/e"
        );
    }

    #[test]
    fn short_project_single_segment() {
        assert_eq!(short_project("repo"), "repo");
    }

    #[test]
    fn short_project_trailing_slash() {
        assert_eq!(
            short_project("/Users/x/owner/repo/"),
            "owner/repo"
        );
    }
}
