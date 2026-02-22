use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn task_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("task")
}

// --- write ---

#[test]
fn write_outputs_task_add_prefix() {
    task_cmd()
        .args(["write", "todo", "test task"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("TASK_ADD_"));
}

#[test]
fn write_invalid_status_fails() {
    task_cmd()
        .args(["write", "invalid", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid status"));
}

#[test]
fn write_all_valid_statuses_succeed() {
    for status in ["inbox", "todo", "doing", "blocked", "inreview", "done"] {
        task_cmd()
            .args(["write", status, "test"])
            .assert()
            .success();
    }
}

// --- doing ---

#[test]
fn doing_nonexistent_id_fails() {
    task_cmd()
        .args(["doing", "nonexist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// --- reviewing ---

#[test]
fn reviewing_nonexistent_id_fails() {
    task_cmd()
        .args(["reviewing", "nonexist", "https://pr.url"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

// --- get ---

#[test]
fn get_empty_succeeds() {
    task_cmd().args(["get"]).assert().success();
}

// --- help ---

#[test]
fn help_flag_works() {
    task_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("task management"));
}

#[test]
fn no_args_shows_usage() {
    task_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}
