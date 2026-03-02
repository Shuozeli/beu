mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn cli_idea_add_and_list() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args([
            "idea",
            "add",
            "add",
            "rate",
            "limiting",
            "--area",
            "api",
            "--priority",
            "high",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("add rate limiting"));
    assert!(stdout.contains("[api]"));
    assert!(stdout.contains("(high)"));

    let output = beu_cmd(&dir).args(["idea", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("add rate limiting"));
}

#[test]
fn cli_idea_list_empty() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["idea", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No ideas"));
}

#[test]
fn cli_idea_show() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["idea", "add", "test", "item"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["idea", "show", "1"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Idea #1: test item"));
    assert!(stdout.contains("Status: pending"));
}

#[test]
fn cli_idea_done_and_archive() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["idea", "add", "task", "one"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["idea", "add", "task", "two"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["idea", "done", "1"]).output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("#1 done"));

    let output = beu_cmd(&dir)
        .args(["idea", "archive", "2"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("#2 archived"));

    // Default list excludes archived.
    let output = beu_cmd(&dir).args(["idea", "list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task one"));
    assert!(!stdout.contains("task two"));
}

#[test]
fn cli_idea_describe() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["idea", "add", "design", "api"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["idea", "describe", "1", "Detailed", "design", "document"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("description updated"));

    let output = beu_cmd(&dir).args(["idea", "show", "1"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Detailed design document"));
}

#[test]
fn cli_idea_invalid_area() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["idea", "add", "task", "--area", "invalid"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid area"));
}

#[test]
fn cli_idea_list_with_filters() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["idea", "add", "api task", "--area", "api"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["idea", "add", "ui task", "--area", "ui"])
        .output()
        .unwrap();
    beu_cmd(&dir).args(["idea", "done", "1"]).output().unwrap();

    let output = beu_cmd(&dir)
        .args(["idea", "list", "--area", "api"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("api task"));
    assert!(!stdout.contains("ui task"));

    let output = beu_cmd(&dir)
        .args(["idea", "list", "--status", "pending"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ui task"));
    assert!(!stdout.contains("api task"));
}

#[test]
fn cli_idea_show_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["idea", "show", "999"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}
