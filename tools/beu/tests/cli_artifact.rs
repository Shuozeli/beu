mod common;

use common::{beu_cmd, beu_dir_path, setup};

#[test]
fn cli_artifact_add_and_list() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["artifact", "add", "design-doc"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("design-doc"));
    assert!(stdout.contains("pending"));

    let output = beu_cmd(&dir).args(["artifact", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("design-doc"));
}

#[test]
fn cli_artifact_status_and_show() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["artifact", "add", "architecture"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["artifact", "status", "architecture", "in-progress"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pending -> in-progress"));

    let output = beu_cmd(&dir)
        .args(["artifact", "show", "architecture"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("in-progress"));
}

#[test]
fn cli_artifact_remove() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["artifact", "add", "temp-doc"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["artifact", "remove", "temp-doc"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Removed"));

    // Verify it's gone.
    let output = beu_cmd(&dir).args(["artifact", "list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("temp-doc"));
}

#[test]
fn cli_artifact_invalid_status() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["artifact", "add", "x"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["artifact", "status", "x", "bogus"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid status"));
}

#[test]
fn cli_artifact_duplicate_add() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["artifact", "add", "design-doc"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir)
        .args(["artifact", "add", "design-doc"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn cli_artifact_show_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["artifact", "show", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_artifact_status_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["artifact", "status", "nonexistent", "done"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_artifact_remove_nonexistent() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["artifact", "remove", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

#[test]
fn cli_artifact_custom_type() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["artifact", "add", "integration-tests", "--type", "test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("integration-tests"));
    assert!(stdout.contains("test"));

    let output = beu_cmd(&dir)
        .args(["artifact", "show", "integration-tests"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Type: test"));
}

#[test]
fn cli_artifact_list_with_filter() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["artifact", "add", "doc-a"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "add", "doc-b"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "doc-a", "done"])
        .output()
        .unwrap();

    // Filter by done -> only doc-a.
    let output = beu_cmd(&dir)
        .args(["artifact", "list", "--filter", "done"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("doc-a"));
    assert!(!stdout.contains("doc-b"));

    // Filter by pending -> only doc-b.
    let output = beu_cmd(&dir)
        .args(["artifact", "list", "--filter", "pending"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("doc-a"));
    assert!(stdout.contains("doc-b"));
}

#[test]
fn cli_artifact_list_empty() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["artifact", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No artifacts"));
}

#[test]
fn cli_artifact_full_lifecycle() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Add.
    let output = beu_cmd(&dir)
        .args(["artifact", "add", "api-spec", "--type", "spec"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Status transitions.
    let output = beu_cmd(&dir)
        .args(["artifact", "status", "api-spec", "in-progress"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pending -> in-progress"));

    let output = beu_cmd(&dir)
        .args(["artifact", "status", "api-spec", "review"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("in-progress -> review"));

    let output = beu_cmd(&dir)
        .args(["artifact", "status", "api-spec", "done"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("review -> done"));

    // Show final state.
    let output = beu_cmd(&dir)
        .args(["artifact", "show", "api-spec"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status: done"));
    assert!(stdout.contains("Type: spec"));

    // Remove.
    let output = beu_cmd(&dir)
        .args(["artifact", "remove", "api-spec"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Verify gone.
    let output = beu_cmd(&dir).args(["artifact", "list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("api-spec"));
}
