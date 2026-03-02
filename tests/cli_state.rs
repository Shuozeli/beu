mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn cli_state_set_and_get() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args([
            "state",
            "set",
            "--category",
            "decision",
            "auth-method",
            "JWT",
            "with",
            "RS256",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[decision] auth-method: JWT with RS256"));

    let output = beu_cmd(&dir)
        .args(["state", "get", "auth-method"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Key: auth-method"));
    assert!(stdout.contains("Category: decision"));
    assert!(stdout.contains("Value: JWT with RS256"));
}

#[test]
fn cli_state_set_upsert() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["state", "set", "--category", "blocker", "ci", "flaky tests"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["state", "set", "--category", "blocker", "ci", "resolved"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["state", "get", "ci"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Value: resolved"));
}

#[test]
fn cli_state_list_all_and_by_category() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["state", "set", "--category", "decision", "db", "postgres"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["state", "set", "--category", "blocker", "ci", "slow"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["state", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[decision] db: postgres"));
    assert!(stdout.contains("[blocker] ci: slow"));

    let output = beu_cmd(&dir)
        .args(["state", "list", "--category", "decision"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[decision] db: postgres"));
    assert!(!stdout.contains("blocker"));
}

#[test]
fn cli_state_invalid_category() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["state", "set", "--category", "invalid", "key", "val"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid category"));
}

#[test]
fn cli_state_remove() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["state", "set", "--category", "note", "foo", "bar"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["state", "remove", "foo"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Removed"));

    let output = beu_cmd(&dir)
        .args(["state", "get", "foo"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_state_clear_requires_force() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["state", "set", "--category", "note", "x", "y"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["state", "clear", "--category", "note"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--force"));
}

#[test]
fn cli_state_clear_with_force() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["state", "set", "--category", "note", "a", "1"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["state", "set", "--category", "note", "b", "2"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["state", "clear", "--category", "note", "--force"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cleared"));

    let output = beu_cmd(&dir)
        .args(["state", "list", "--category", "note"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No state entries"));
}

#[test]
fn cli_state_get_all() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args([
            "state",
            "set",
            "--category",
            "focus",
            "current",
            "auth module",
        ])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["state", "get"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[focus] current: auth module"));
}
