use std::fs;
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

const SNIPPET: &str = r#"REQUIRED: manage tasks with `task` CLI before, during, and after work. Never skip.
Commands: `task list [status]` | `task create "<title>" ["<desc>"]` | `task update <id> <status> ["<note>"]` | `task get <id>`
Session start: `task list doing` (resume) → `task list todo` (pick) → `task create` (if none). Always `task update <id> doing` before work.
During: human input needed → `task update <id> blocked "<reason>"`. Resumed → `task update <id> doing`.
End: PR → `task update <id> inreview "<pr_url>"`. Direct commit → `task update <id> done`. Unfinished → `blocked`. Unnecessary → `done`.
Limits: title ≤ 80, desc ≤ 200, note ≤ 200 chars."#;

const SNIPPET_DETECT: &str = "task update <id> doing";

struct InjectionTarget {
    path: PathBuf,
    header: String,
    create_file: bool,
    frontmatter: Option<String>,
}

fn inject_into(target: &InjectionTarget) -> Option<String> {
    let path = &target.path;

    if target.create_file {
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            return None;
        }
    } else if !path.exists() {
        return None;
    }

    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing.contains(SNIPPET_DETECT) {
        return None;
    }

    let content = if let Some(fm) = &target.frontmatter {
        format!("{fm}\n{}\n{SNIPPET}\n", target.header)
    } else {
        format!("\n\n{}\n{SNIPPET}\n", target.header)
    };

    if target.create_file && !path.exists() {
        fs::write(path, content).ok()?;
    } else {
        let mut full = existing;
        full.push_str(&content);
        fs::write(path, full).ok()?;
    }

    Some(path.to_string_lossy().to_string())
}

pub struct InitResult {
    pub injected: Vec<String>,
    pub candidates: Vec<String>,
}

pub fn run_init(global: bool) -> InitResult {
    let targets = if global {
        global_targets()
    } else {
        local_targets()
    };

    let candidates: Vec<String> = targets
        .iter()
        .filter(|t| !t.create_file)
        .map(|t| {
            t.path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    let injected = targets.iter().filter_map(inject_into).collect();

    InitResult {
        injected,
        candidates,
    }
}

fn local_targets() -> Vec<InjectionTarget> {
    let cwd = std::env::current_dir().unwrap_or_default();
    vec![
        InjectionTarget {
            path: cwd.join("CLAUDE.md"),
            header: "## Task Management".into(),
            create_file: false,
            frontmatter: None,
        },
        InjectionTarget {
            path: cwd.join("AGENTS.md"),
            header: "## Task Management".into(),
            create_file: false,
            frontmatter: None,
        },
        InjectionTarget {
            path: cwd.join("GEMINI.md"),
            header: "## Task Management".into(),
            create_file: false,
            frontmatter: None,
        },
        InjectionTarget {
            path: cwd.join(".cursor/rules/task-management.mdc"),
            header: String::new(),
            create_file: true,
            frontmatter: Some(
                "---\ndescription: Task management workflow using the task CLI\nglobs:\nalwaysApply: true\n---\n".into(),
            ),
        },
        InjectionTarget {
            path: cwd.join(".clinerules/task-management.md"),
            header: "# Task Management".into(),
            create_file: true,
            frontmatter: None,
        },
    ]
}

fn global_targets() -> Vec<InjectionTarget> {
    let home = home_dir();
    vec![
        InjectionTarget {
            path: home.join(".claude/CLAUDE.md"),
            header: "## Task Management".into(),
            create_file: true,
            frontmatter: None,
        },
        InjectionTarget {
            path: home.join(".codex/AGENTS.md"),
            header: "## Task Management".into(),
            create_file: true,
            frontmatter: None,
        },
        InjectionTarget {
            path: home.join(".gemini/GEMINI.md"),
            header: "## Task Management".into(),
            create_file: true,
            frontmatter: None,
        },
        InjectionTarget {
            path: home.join(".config/cline/rules/task-management.md"),
            header: "# Task Management".into(),
            create_file: true,
            frontmatter: None,
        },
        InjectionTarget {
            path: home.join(".config/opencode/AGENTS.md"),
            header: "## Task Management".into(),
            create_file: true,
            frontmatter: None,
        },
    ]
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME not set"))
}

/// Testable version: inject into specific paths
#[cfg(test)]
pub fn inject_into_file(path: &Path, header: &str, frontmatter: Option<&str>) -> Option<String> {
    let target = InjectionTarget {
        path: path.to_path_buf(),
        header: header.to_string(),
        create_file: path.exists() || frontmatter.is_some(),
        frontmatter: frontmatter.map(|s| s.to_string()),
    };
    inject_into(&target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn inject_into_existing_file() {
        let dir = temp_dir();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# My Project\n").unwrap();
        let result = inject_into_file(&path, "## Task Management", None);
        assert!(result.is_some());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("## Task Management"));
        assert!(content.contains(SNIPPET_DETECT));
        assert!(content.starts_with("# My Project\n"));
    }

    #[test]
    fn inject_is_idempotent() {
        let dir = temp_dir();
        let path = dir.path().join("AGENTS.md");
        fs::write(&path, "# Agents\n").unwrap();
        inject_into_file(&path, "## Task Management", None);
        let first = fs::read_to_string(&path).unwrap();
        let result = inject_into_file(&path, "## Task Management", None);
        assert!(result.is_none());
        let second = fs::read_to_string(&path).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn inject_skips_nonexistent_file() {
        let dir = temp_dir();
        let path = dir.path().join("NONEXIST.md");
        let result = inject_into_file(&path, "## Task Management", None);
        assert!(result.is_none());
        assert!(!path.exists());
    }

    #[test]
    fn inject_with_frontmatter_creates_file() {
        let dir = temp_dir();
        let path = dir.path().join("task-management.mdc");
        let fm = "---\ndescription: test\n---\n";
        let result = inject_into_file(&path, "", Some(fm));
        assert!(result.is_some());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains(SNIPPET_DETECT));
    }

    #[test]
    fn inject_appends_with_double_newline() {
        let dir = temp_dir();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "existing content").unwrap();
        inject_into_file(&path, "## Task Management", None);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("existing content\n\n## Task Management"));
    }

    #[test]
    fn snippet_content_matches_readme() {
        assert!(SNIPPET.contains("task list [status]"));
        assert!(SNIPPET.contains("task create"));
        assert!(SNIPPET.contains("task update"));
        assert!(SNIPPET.contains("task get"));
        assert!(SNIPPET.contains("REQUIRED"));
        assert!(SNIPPET.contains("Never skip"));
        assert!(SNIPPET.contains("Limits:"));
    }
}
