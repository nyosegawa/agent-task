use rand::RngExt;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct TaskEntry {
    pub id: String,
    pub project: String,
    pub status: String,
    pub title: String,
    pub description: String,
}

impl TaskEntry {
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(" | ").collect();
        if parts.len() < 4 {
            return None;
        }
        Some(Self {
            id: parts[0].to_string(),
            project: parts[1].to_string(),
            status: parts[2].to_string(),
            title: parts[3].to_string(),
            description: if parts.len() >= 5 {
                parts[4..].join(" | ")
            } else {
                String::new()
            },
        })
    }

    pub fn format_line(&self) -> String {
        if self.description.is_empty() {
            format!(
                "{} | {} | {} | {}",
                self.id, self.project, self.status, self.title
            )
        } else {
            format!(
                "{} | {} | {} | {} | {}",
                self.id, self.project, self.status, self.title, self.description
            )
        }
    }
}

pub fn gen_id() -> String {
    let mut rng = rand::rng();
    let bytes: [u8; 4] = rng.random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct TaskStore {
    path: PathBuf,
}

impl TaskStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> Self {
        let home = env::var("HOME").expect("HOME not set");
        Self::new(PathBuf::from(home).join(".local/share/tasks/tasks.log"))
    }

    fn ensure_dir(&self) {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).expect("Failed to create tasks directory");
        }
    }

    pub fn append(&self, entry: &TaskEntry) {
        self.ensure_dir();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .expect("Failed to open tasks.log");
        writeln!(file, "{}", entry.format_line()).expect("Failed to write");
    }

    pub fn id_exists(&self, id: &str) -> bool {
        self.read_entries().iter().any(|entry| entry.id == id)
    }

    pub fn latest_title(&self, id: &str) -> String {
        self.read_entries()
            .iter()
            .rev()
            .find(|e| e.id == id)
            .map(|e| e.title.clone())
            .unwrap_or_default()
    }

    pub fn current_tasks(&self, status_filter: Option<&str>) -> Vec<TaskEntry> {
        let entries = self.read_entries();

        let mut latest: HashMap<String, TaskEntry> = HashMap::new();
        let mut first_seen: HashMap<String, usize> = HashMap::new();
        for (i, entry) in entries.into_iter().enumerate() {
            first_seen.entry(entry.id.clone()).or_insert(i);
            latest.insert(entry.id.clone(), entry);
        }

        let mut result: Vec<TaskEntry> = latest.into_values().collect();
        result.sort_by_key(|e| first_seen.get(&e.id).copied().unwrap_or(usize::MAX));

        let mut seen = HashSet::new();
        result
            .into_iter()
            .filter(|e| seen.insert(e.id.clone()) && status_filter.is_none_or(|f| e.status == f))
            .collect()
    }

    pub fn read_entries(&self) -> Vec<TaskEntry> {
        if !self.path.exists() {
            return vec![];
        }
        let content = fs::read_to_string(&self.path).expect("Failed to read tasks.log");
        content.lines().filter_map(TaskEntry::parse).collect()
    }

    #[cfg(test)]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.log");
        (TaskStore::new(path), dir)
    }

    fn entry(id: &str, status: &str, title: &str) -> TaskEntry {
        TaskEntry {
            id: id.into(),
            project: "test/proj".into(),
            status: status.into(),
            title: title.into(),
            description: String::new(),
        }
    }

    // --- gen_id ---

    #[test]
    fn gen_id_is_8_char_hex() {
        let id = gen_id();
        assert_eq!(id.len(), 8);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn gen_id_is_unique() {
        let ids: HashSet<String> = (0..100).map(|_| gen_id()).collect();
        assert_eq!(ids.len(), 100);
    }

    // --- TaskEntry::parse ---

    #[test]
    fn parse_4_fields() {
        let e = TaskEntry::parse("abc12345 | myproject | todo | Do something").unwrap();
        assert_eq!(e.id, "abc12345");
        assert_eq!(e.project, "myproject");
        assert_eq!(e.status, "todo");
        assert_eq!(e.title, "Do something");
        assert_eq!(e.description, "");
    }

    #[test]
    fn parse_5_fields() {
        let e =
            TaskEntry::parse("abc12345 | myproject | inreview | Task | https://pr.url").unwrap();
        assert_eq!(e.description, "https://pr.url");
    }

    #[test]
    fn parse_description_with_pipes() {
        let e =
            TaskEntry::parse("abc12345 | proj | blocked | Task | reason | more detail").unwrap();
        assert_eq!(e.description, "reason | more detail");
    }

    #[test]
    fn parse_too_few_fields() {
        assert!(TaskEntry::parse("abc12345 | proj | todo").is_none());
        assert!(TaskEntry::parse("").is_none());
    }

    // --- TaskEntry::format_line ---

    #[test]
    fn format_without_description() {
        assert_eq!(
            entry("a1", "todo", "T").format_line(),
            "a1 | test/proj | todo | T"
        );
    }

    #[test]
    fn format_with_description() {
        let mut e = entry("a1", "inreview", "T");
        e.description = "https://url".into();
        assert_eq!(
            e.format_line(),
            "a1 | test/proj | inreview | T | https://url"
        );
    }

    #[test]
    fn parse_roundtrip() {
        let original = TaskEntry {
            id: "deadbeef".into(),
            project: "owner/repo".into(),
            status: "blocked".into(),
            title: "Something broke".into(),
            description: "need help".into(),
        };
        let parsed = TaskEntry::parse(&original.format_line()).unwrap();
        assert_eq!(original, parsed);
    }

    // --- TaskStore ---

    #[test]
    fn append_and_read() {
        let (store, _dir) = temp_store();
        let e = entry("aabbccdd", "todo", "First task");
        store.append(&e);
        let entries = store.read_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], e);
    }

    #[test]
    fn append_is_additive() {
        let (store, _dir) = temp_store();
        for i in 0..3 {
            store.append(&entry(&format!("id{i:07}"), "todo", &format!("Task {i}")));
        }
        assert_eq!(store.read_entries().len(), 3);
    }

    #[test]
    fn id_exists_check() {
        let (store, _dir) = temp_store();
        assert!(!store.id_exists("nonexist"));
        store.append(&entry("exist123", "todo", "T"));
        assert!(store.id_exists("exist123"));
        assert!(!store.id_exists("other456"));
    }

    #[test]
    fn latest_title_tracks_updates() {
        let (store, _dir) = temp_store();
        store.append(&entry("aabb0011", "todo", "Original"));
        store.append(&entry("aabb0011", "doing", "Updated"));
        assert_eq!(store.latest_title("aabb0011"), "Updated");
    }

    #[test]
    fn latest_title_missing_returns_empty() {
        let (store, _dir) = temp_store();
        assert_eq!(store.latest_title("nope"), "");
    }

    #[test]
    fn current_tasks_deduplicates() {
        let (store, _dir) = temp_store();
        store.append(&entry("task0001", "todo", "A"));
        store.append(&entry("task0001", "doing", "A"));
        let tasks = store.current_tasks(None);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "doing");
    }

    #[test]
    fn current_tasks_filters_by_status() {
        let (store, _dir) = temp_store();
        store.append(&entry("t1", "todo", "A"));
        store.append(&entry("t2", "doing", "B"));
        assert_eq!(store.current_tasks(Some("todo")).len(), 1);
        assert_eq!(store.current_tasks(Some("todo"))[0].id, "t1");
        assert_eq!(store.current_tasks(Some("doing"))[0].id, "t2");
    }

    #[test]
    fn current_tasks_preserves_insertion_order() {
        let (store, _dir) = temp_store();
        for id in ["c", "a", "b"] {
            store.append(&entry(id, "todo", id));
        }
        let tasks = store.current_tasks(None);
        let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn empty_store() {
        let (store, _dir) = temp_store();
        assert!(store.read_entries().is_empty());
        assert!(store.current_tasks(None).is_empty());
        assert!(!store.id_exists("any"));
    }

    #[test]
    fn append_only_file_grows() {
        let (store, _dir) = temp_store();
        store.append(&entry("x", "todo", "T"));
        store.append(&entry("x", "doing", "T"));
        let raw = fs::read_to_string(store.path()).unwrap();
        assert_eq!(raw.lines().count(), 2);
    }
}
