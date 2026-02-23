use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn task_cmd_with_log() -> (assert_cmd::Command, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("tasks.log");
    let mut cmd = cargo_bin_cmd!("task");
    cmd.env("TASK_LOG_PATH", log_path.to_str().unwrap());
    (cmd, dir)
}

fn task_cmd_env(dir: &tempfile::TempDir) -> assert_cmd::Command {
    let log_path = dir.path().join("tasks.log");
    let mut cmd = cargo_bin_cmd!("task");
    cmd.env("TASK_LOG_PATH", log_path.to_str().unwrap());
    cmd
}

// --- create ---

#[test]
fn create_outputs_task_add_prefix() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["create", "test task"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("TASK_ADD_"));
}

#[test]
fn create_with_description() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["create", "my task", "detailed description"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("TASK_ADD_"));
}

#[test]
fn create_with_status_flag() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["create", "inbox task", "--status", "inbox"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("TASK_ADD_"));
}

#[test]
fn create_any_status_accepted() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["create", "custom", "--status", "custom_status"])
        .assert()
        .success();
}

#[test]
fn create_title_too_long_fails() {
    let long_title = "x".repeat(51);
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["create", &long_title])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds 50 chars"));
}

#[test]
fn create_description_too_long_fails() {
    let long_desc = "x".repeat(501);
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["create", "title", &long_desc])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds 500 chars"));
}

// --- update ---

#[test]
fn update_nonexistent_id_fails() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["update", "nonexist", "doing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn update_outputs_status_prefix() {
    let (mut cmd, dir) = task_cmd_with_log();
    let create_output = cmd
        .args(["create", "test"])
        .output()
        .expect("create failed");
    let stdout = String::from_utf8_lossy(&create_output.stdout);
    let id = stdout.trim().strip_prefix("TASK_ADD_").unwrap();

    task_cmd_env(&dir)
        .args(["update", id, "doing"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with(&format!("TASK_DOING_{id}")));
}

#[test]
fn update_with_note() {
    let (mut cmd, dir) = task_cmd_with_log();
    let create_output = cmd
        .args(["create", "test"])
        .output()
        .expect("create failed");
    let stdout = String::from_utf8_lossy(&create_output.stdout);
    let id = stdout.trim().strip_prefix("TASK_ADD_").unwrap();

    task_cmd_env(&dir)
        .args(["update", id, "blocked", "API not ready"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with(&format!("TASK_BLOCKED_{id}")));
}

#[test]
fn update_note_too_long_fails() {
    let (mut cmd, dir) = task_cmd_with_log();
    let create_output = cmd
        .args(["create", "test"])
        .output()
        .expect("create failed");
    let stdout = String::from_utf8_lossy(&create_output.stdout);
    let id = stdout.trim().strip_prefix("TASK_ADD_").unwrap();

    let long_note = "x".repeat(201);
    task_cmd_env(&dir)
        .args(["update", id, "blocked", &long_note])
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds 200 chars"));
}

// --- list ---

#[test]
fn list_empty_succeeds() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["list", "--all"]).assert().success();
}

// --- get ---

#[test]
fn get_nonexistent_id_fails() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["get", "nonexist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn get_shows_history() {
    let (mut cmd, dir) = task_cmd_with_log();
    let create_output = cmd
        .args(["create", "history test"])
        .output()
        .expect("create failed");
    let stdout = String::from_utf8_lossy(&create_output.stdout);
    let id = stdout.trim().strip_prefix("TASK_ADD_").unwrap();

    task_cmd_env(&dir)
        .args(["update", id, "doing"])
        .assert()
        .success();

    task_cmd_env(&dir)
        .args(["get", id])
        .assert()
        .success()
        .stdout(predicate::str::contains("history test"))
        .stdout(predicate::str::contains("todo"))
        .stdout(predicate::str::contains("doing"));
}

// --- init ---

#[test]
fn init_no_targets_shows_message() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.args(["init"]).assert().success().stdout(
        predicate::str::contains("No instruction files found")
            .or(predicate::str::contains("Already up-to-date"))
            .or(predicate::str::contains("Injected")),
    );
}

// --- help ---

#[test]
fn help_flag_works() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("task management"));
}

#[test]
fn no_args_shows_usage() {
    let (mut cmd, _dir) = task_cmd_with_log();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}
