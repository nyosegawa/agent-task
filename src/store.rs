use chrono::Local;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEntry {
    pub ts: String,
    pub id: String,
    pub project: String,
    pub status: String,
    pub title: String,
    pub description: String,
    pub note: String,
}

impl TaskEntry {
    pub fn new(
        id: String,
        project: String,
        status: String,
        title: String,
        description: String,
        note: String,
    ) -> Self {
        Self {
            ts: Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            id,
            project,
            status,
            title,
            description,
            note,
        }
    }

    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize TaskEntry")
    }

    pub fn from_jsonl(line: &str) -> Option<Self> {
        serde_json::from_str(line).ok()
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
        if let Ok(custom) = env::var("TASK_LOG_PATH") {
            return Self::new(PathBuf::from(custom));
        }
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
        writeln!(file, "{}", entry.to_jsonl()).expect("Failed to write");
    }

    pub fn read_entries(&self) -> Vec<TaskEntry> {
        if !self.path.exists() {
            return vec![];
        }
        let content = fs::read_to_string(&self.path).expect("Failed to read tasks.log");
        content.lines().filter_map(TaskEntry::from_jsonl).collect()
    }

    pub fn id_exists(&self, id: &str) -> bool {
        self.read_entries().iter().any(|e| e.id == id)
    }

    pub fn latest_entry(&self, id: &str) -> Option<TaskEntry> {
        self.read_entries().into_iter().rev().find(|e| e.id == id)
    }

    pub fn entries_for_id(&self, id: &str) -> Vec<TaskEntry> {
        self.read_entries()
            .into_iter()
            .filter(|e| e.id == id)
            .collect()
    }

    pub fn current_tasks(
        &self,
        project: Option<&str>,
        status_filter: Option<&str>,
    ) -> Vec<TaskEntry> {
        let entries = self.read_entries();

        let mut latest: HashMap<String, TaskEntry> = HashMap::new();
        let mut first_seen: HashMap<String, usize> = HashMap::new();
        for (i, entry) in entries.into_iter().enumerate() {
            first_seen.entry(entry.id.clone()).or_insert(i);
            latest.insert(entry.id.clone(), entry);
        }

        let mut result: Vec<TaskEntry> = latest.into_values().collect();
        result.sort_by_key(|e| first_seen.get(&e.id).copied().unwrap_or(usize::MAX));

        result
            .into_iter()
            .filter(|e| project.is_none_or(|p| e.project == p))
            .filter(|e| status_filter.is_none_or(|s| e.status == s))
            .collect()
    }

    #[cfg(test)]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.log");
        (TaskStore::new(path), dir)
    }

    fn entry(id: &str, status: &str, title: &str) -> TaskEntry {
        TaskEntry {
            ts: "2026-02-22T14:30:00+09:00".into(),
            id: id.into(),
            project: "test/proj".into(),
            status: status.into(),
            title: title.into(),
            description: String::new(),
            note: String::new(),
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

    // --- TaskEntry JSONL ---

    #[test]
    fn to_jsonl_contains_all_fields() {
        let e = entry("a1b2c3d4", "todo", "Test task");
        let json = e.to_jsonl();
        assert!(json.contains("\"id\":\"a1b2c3d4\""));
        assert!(json.contains("\"status\":\"todo\""));
        assert!(json.contains("\"title\":\"Test task\""));
        assert!(json.contains("\"ts\":"));
        assert!(json.contains("\"project\":"));
        assert!(json.contains("\"description\":"));
        assert!(json.contains("\"note\":"));
    }

    #[test]
    fn from_jsonl_roundtrip() {
        let original = TaskEntry {
            ts: "2026-02-22T14:30:00+09:00".into(),
            id: "deadbeef".into(),
            project: "owner/repo".into(),
            status: "blocked".into(),
            title: "Something broke".into(),
            description: "need help".into(),
            note: "API issue".into(),
        };
        let json = original.to_jsonl();
        let parsed = TaskEntry::from_jsonl(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn from_jsonl_invalid() {
        assert!(TaskEntry::from_jsonl("not json").is_none());
        assert!(TaskEntry::from_jsonl("").is_none());
        assert!(TaskEntry::from_jsonl("{}").is_none());
    }

    #[test]
    fn jsonl_multiline_note() {
        let mut e = entry("a1", "blocked", "Task");
        e.note = "line1\nline2".into();
        let json = e.to_jsonl();
        assert!(!json.contains('\n') || json.matches('\n').count() == 0);
        let parsed = TaskEntry::from_jsonl(&json).unwrap();
        assert_eq!(parsed.note, "line1\nline2");
    }

    #[test]
    fn new_sets_timestamp() {
        let e = TaskEntry::new(
            "id".into(),
            "proj".into(),
            "todo".into(),
            "T".into(),
            String::new(),
            String::new(),
        );
        assert!(!e.ts.is_empty());
        assert!(e.ts.contains('T'));
    }

    // --- TaskStore ---

    #[test]
    fn append_and_read() {
        let (store, _dir) = temp_store();
        let e = entry("aabbccdd", "todo", "First task");
        store.append(&e);
        let entries = store.read_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "aabbccdd");
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
    fn latest_entry_tracks_updates() {
        let (store, _dir) = temp_store();
        store.append(&entry("aabb0011", "todo", "Original"));
        let mut e2 = entry("aabb0011", "doing", "Original");
        e2.note = "started".into();
        store.append(&e2);
        let latest = store.latest_entry("aabb0011").unwrap();
        assert_eq!(latest.status, "doing");
        assert_eq!(latest.note, "started");
    }

    #[test]
    fn latest_entry_missing_returns_none() {
        let (store, _dir) = temp_store();
        assert!(store.latest_entry("nope").is_none());
    }

    #[test]
    fn entries_for_id_returns_history() {
        let (store, _dir) = temp_store();
        store.append(&entry("t1", "todo", "A"));
        store.append(&entry("t1", "doing", "A"));
        store.append(&entry("t2", "todo", "B"));
        store.append(&entry("t1", "blocked", "A"));
        let history = store.entries_for_id("t1");
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].status, "todo");
        assert_eq!(history[1].status, "doing");
        assert_eq!(history[2].status, "blocked");
    }

    #[test]
    fn current_tasks_deduplicates() {
        let (store, _dir) = temp_store();
        store.append(&entry("task0001", "todo", "A"));
        store.append(&entry("task0001", "doing", "A"));
        let tasks = store.current_tasks(None, None);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "doing");
    }

    #[test]
    fn current_tasks_filters_by_status() {
        let (store, _dir) = temp_store();
        store.append(&entry("t1", "todo", "A"));
        store.append(&entry("t2", "doing", "B"));
        assert_eq!(store.current_tasks(None, Some("todo")).len(), 1);
        assert_eq!(store.current_tasks(None, Some("todo"))[0].id, "t1");
    }

    #[test]
    fn current_tasks_filters_by_project() {
        let (store, _dir) = temp_store();
        store.append(&entry("t1", "todo", "A"));
        let mut e2 = entry("t2", "todo", "B");
        e2.project = "other/proj".into();
        store.append(&e2);
        assert_eq!(store.current_tasks(Some("test/proj"), None).len(), 1);
        assert_eq!(store.current_tasks(Some("other/proj"), None).len(), 1);
        assert_eq!(store.current_tasks(None, None).len(), 2);
    }

    #[test]
    fn current_tasks_preserves_insertion_order() {
        let (store, _dir) = temp_store();
        for id in ["c", "a", "b"] {
            store.append(&entry(id, "todo", id));
        }
        let tasks = store.current_tasks(None, None);
        let ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn empty_store() {
        let (store, _dir) = temp_store();
        assert!(store.read_entries().is_empty());
        assert!(store.current_tasks(None, None).is_empty());
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

    #[test]
    fn stored_as_valid_jsonl() {
        let (store, _dir) = temp_store();
        let mut e = entry("a1b2c3d4", "todo", "Test");
        e.description = "desc".into();
        e.note = "note".into();
        store.append(&e);
        let raw = fs::read_to_string(store.path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
        assert_eq!(parsed["id"], "a1b2c3d4");
        assert_eq!(parsed["status"], "todo");
        assert_eq!(parsed["description"], "desc");
        assert_eq!(parsed["note"], "note");
    }
}
