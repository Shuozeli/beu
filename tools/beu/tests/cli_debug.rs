mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn cli_debug_full_lifecycle() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Open.
    let output = beu_cmd(&dir)
        .args(["debug", "open", "login", "fails", "after", "refresh"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("login-fails-after-refresh"));
    assert!(stdout.contains("opened"));

    // Symptom.
    let output = beu_cmd(&dir)
        .args([
            "debug",
            "symptom",
            "login-fails-after-refresh",
            "401",
            "after",
            "token",
            "refresh",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("symptom"));

    // Log evidence.
    let output = beu_cmd(&dir)
        .args([
            "debug",
            "log",
            "login-fails-after-refresh",
            "token",
            "TTL",
            "is",
            "0",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("evidence"));

    // Cause.
    let output = beu_cmd(&dir)
        .args([
            "debug",
            "cause",
            "login-fails-after-refresh",
            "config",
            "deployed",
            "with",
            "TTL=0",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("root cause"));

    // Show timeline.
    let output = beu_cmd(&dir)
        .args(["debug", "show", "login-fails-after-refresh"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Timeline:"));
    assert!(stdout.contains("[symptom]"));
    assert!(stdout.contains("[evidence]"));
    assert!(stdout.contains("[cause]"));

    // Resolve.
    let output = beu_cmd(&dir)
        .args(["debug", "resolve", "login-fails-after-refresh"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("resolved"));
}

#[test]
fn cli_debug_list() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["debug", "open", "bug", "one"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["debug", "open", "bug", "two"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["debug", "resolve", "bug-one"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["debug", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bug-one"));
    assert!(stdout.contains("bug-two"));

    let output = beu_cmd(&dir)
        .args(["debug", "list", "--status", "investigating"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("bug-one"));
    assert!(stdout.contains("bug-two"));
}

#[test]
fn cli_debug_resolve_already_resolved() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["debug", "open", "test", "bug"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["debug", "resolve", "test-bug"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["debug", "resolve", "test-bug"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already resolved"));
}

#[test]
fn cli_debug_log_on_resolved_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["debug", "open", "closed", "bug"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["debug", "resolve", "closed-bug"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["debug", "log", "closed-bug", "new", "evidence"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("resolved"));
}

#[test]
fn cli_debug_show_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["debug", "show", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_debug_slug_dedup() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["debug", "open", "same", "title"])
        .output()
        .unwrap();
    let output = beu_cmd(&dir)
        .args(["debug", "open", "same", "title"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("same-title-2"));
}
