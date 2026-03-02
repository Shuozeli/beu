mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn beu_test_patterns_shows_defaults() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["test", "patterns"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("unit"), "stdout: {stdout}");
    assert!(stdout.contains("integration"), "stdout: {stdout}");
    assert!(stdout.contains("systest"), "stdout: {stdout}");
    assert!(stdout.contains("golden"), "stdout: {stdout}");
    assert!(
        stdout.contains("planned -> designed -> implemented -> tested -> darklaunched -> launched"),
        "stdout: {stdout}"
    );
}

#[test]
fn task_show_includes_test_status() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "implement", "auth"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["task", "show", "1"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Status: planned"), "stdout: {stdout}");
}

#[test]
fn task_test_status_update_and_show() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "implement", "auth"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "test-status", "1", "implemented"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("implemented"), "stdout: {stdout}");

    let output = beu_cmd(&dir).args(["task", "show", "1"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Test Status: implemented"),
        "stdout: {stdout}"
    );
}

#[test]
fn task_test_status_invalid_value_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task", "x"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "test-status", "1", "bogus"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid test status"), "stderr: {stderr}");
}

#[test]
fn task_list_filter_by_test_status() {
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
    beu_cmd(&dir)
        .args(["task", "test-status", "1", "tested"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["task", "list", "--test-status", "tested"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task a"), "stdout: {stdout}");
    assert!(
        !stdout.contains("task b"),
        "stdout should not contain 'task b': {stdout}"
    );
}

#[test]
fn task_list_shows_test_status_inline() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "my", "task"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test:planned"), "stdout: {stdout}");
}

#[test]
fn task_test_status_all_valid_values() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "lifecycle", "task"])
        .output()
        .unwrap();

    for status in &[
        "designed",
        "implemented",
        "tested",
        "darklaunched",
        "launched",
    ] {
        let output = beu_cmd(&dir)
            .args(["task", "test-status", "1", status])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "failed for status '{status}': {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = beu_cmd(&dir).args(["task", "show", "1"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Status: launched"), "stdout: {stdout}");
}
