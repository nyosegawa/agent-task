use std::env;
use std::process::Command;

pub fn get_project() -> String {
    if let Ok(output) = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        && output.status.success()
    {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(project) = extract_project_from_url(&url) {
            return project;
        }
    }
    env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn extract_project_from_url(url: &str) -> Option<String> {
    let path = if let Some(rest) = url.strip_prefix("git@") {
        rest.split(':').nth(1)?
    } else if let Some(without_scheme) = url.strip_prefix("ssh://git@") {
        without_scheme.split_once('/')?.1
    } else if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url.splitn(3, '/').nth(2)?;
        without_scheme.split_once('/')?.1
    } else {
        return None;
    };
    let project = path.strip_suffix(".git").unwrap_or(path);
    if project.contains('/') {
        Some(project.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_at_url() {
        assert_eq!(
            extract_project_from_url("git@github.com:owner/repo.git"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn ssh_url() {
        assert_eq!(
            extract_project_from_url("ssh://git@github.com/owner/repo.git"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn https_url() {
        assert_eq!(
            extract_project_from_url("https://github.com/owner/repo.git"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn https_without_git_suffix() {
        assert_eq!(
            extract_project_from_url("https://github.com/owner/repo"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn http_url() {
        assert_eq!(
            extract_project_from_url("http://github.com/owner/repo.git"),
            Some("owner/repo".into())
        );
    }

    #[test]
    fn invalid_url() {
        assert_eq!(extract_project_from_url("not-a-url"), None);
        assert_eq!(extract_project_from_url(""), None);
    }
}
