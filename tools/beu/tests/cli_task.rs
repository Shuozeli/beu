mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn cli_task_add_and_list() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args([
            "task",
            "add",
            "implement",
            "auth",
            "--priority",
            "high",
            "--tag",
            "backend",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("implement auth"));
    assert!(stdout.contains("high"));

    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("implement auth"));
    assert!(stdout.contains("[backend]"));
}

#[test]
fn cli_task_done_and_show() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "write", "tests"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["task", "done", "1"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("done"));

    let output = beu_cmd(&dir).args(["task", "show", "1"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("done"));
}

#[test]
fn cli_task_update() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "one"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args([
            "task",
            "update",
            "1",
            "--status",
            "in-progress",
            "--priority",
            "high",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status: open -> in-progress"));
    assert!(stdout.contains("priority: medium -> high"));
}

#[test]
fn cli_task_sprint() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "a", "--priority", "critical"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "task", "b"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "update", "1", "--status", "in-progress"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["task", "sprint"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("In Progress:"));
    assert!(stdout.contains("Open:"));
}

#[test]
fn cli_task_invalid_priority() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["task", "add", "task", "x", "--priority", "urgent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid priority"));
}

#[test]
fn cli_task_done_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["task", "done", "999"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_task_show_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["task", "show", "999"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_task_update_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["task", "update", "999", "--status", "done"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_task_update_invalid_status() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "x"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "update", "1", "--status", "bogus"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid status"));
}

#[test]
fn cli_task_update_invalid_priority() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "x"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "update", "1", "--priority", "bogus"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid priority"));
}

#[test]
fn cli_task_update_nothing_to_update() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "x"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "update", "1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nothing to update"));
}

#[test]
fn cli_task_update_tag_only() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "x"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "update", "1", "--tag", "frontend"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tag: frontend"));

    let output = beu_cmd(&dir).args(["task", "show", "1"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tag: frontend"));
}

#[test]
fn cli_task_list_with_status_filter() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "a"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "task", "b"])
        .output()
        .unwrap();
    beu_cmd(&dir).args(["task", "done", "1"]).output().unwrap();

    // Filter by done.
    let output = beu_cmd(&dir)
        .args(["task", "list", "--status", "done"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task a"));
    assert!(!stdout.contains("task b"));

    // Filter by open.
    let output = beu_cmd(&dir)
        .args(["task", "list", "--status", "open"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("task a"));
    assert!(stdout.contains("task b"));
}

#[test]
fn cli_task_list_with_tag_filter() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "backend", "auth", "--tag", "backend"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "frontend", "form", "--tag", "frontend"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "no", "tag"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "list", "--tag", "backend"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("backend auth"));
    assert!(!stdout.contains("frontend form"));
    assert!(!stdout.contains("no tag"));
}

#[test]
fn cli_task_list_empty() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No tasks"));
}

#[test]
fn cli_task_sprint_clear() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "one"])
        .output()
        .unwrap();
    beu_cmd(&dir).args(["task", "done", "1"]).output().unwrap();

    let output = beu_cmd(&dir).args(["task", "sprint"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sprint is clear"));
}

#[test]
fn cli_task_blocked_in_sprint() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "blocked", "task"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "update", "1", "--status", "blocked"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["task", "sprint"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Blocked:"));
    assert!(stdout.contains("blocked task"));
}

#[test]
fn cli_task_priority_ordering() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "low", "task", "--priority", "low"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "critical", "task", "--priority", "critical"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "high", "task", "--priority", "high"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Critical should appear before high, high before low.
    let critical_pos = stdout.find("critical task").unwrap();
    let high_pos = stdout.find("high task").unwrap();
    let low_pos = stdout.find("low task").unwrap();
    assert!(critical_pos < high_pos);
    assert!(high_pos < low_pos);
}
