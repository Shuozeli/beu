mod common;

use std::process::Command;
use tempfile::TempDir;

use common::beu_cmd;

/// Create a fake git repo with multiple subprojects.
fn setup_monorepo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create .git directory to mark git root.
    std::fs::create_dir_all(root.join(".git")).unwrap();

    // Init subprojects.
    for subdir in &["frontend", "backend", "tools/api"] {
        let dir = root.join(subdir);
        std::fs::create_dir_all(&dir).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_beu"))
            .current_dir(&dir)
            .arg("init")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "init failed for {subdir}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    tmp
}

fn beu_cmd_at(dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_beu"));
    cmd.current_dir(dir);
    cmd
}

// ---------------------------------------------------------------------------
// project list
// ---------------------------------------------------------------------------

#[test]
fn project_list_discovers_all() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("frontend"), "stdout: {stdout}");
    assert!(stdout.contains("backend"), "stdout: {stdout}");
    assert!(stdout.contains("tools/api"), "stdout: {stdout}");
    assert!(stdout.contains("3 project(s)"), "stdout: {stdout}");
}

#[test]
fn project_list_with_name_filter() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "list", "--name", "frontend"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("frontend"));
    assert!(stdout.contains("1 project(s)"));
}

#[test]
fn project_list_no_match() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "list", "--name", "nonexistent"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No matching project"));
}

// ---------------------------------------------------------------------------
// project status
// ---------------------------------------------------------------------------

#[test]
fn project_status_shows_all() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "status"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- frontend ---"), "stdout: {stdout}");
    assert!(stdout.contains("--- backend ---"), "stdout: {stdout}");
    assert!(stdout.contains("--- tools/api ---"), "stdout: {stdout}");
    assert!(stdout.contains("3 project(s)"), "stdout: {stdout}");
}

#[test]
fn project_status_single_project() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "status", "--name", "backend"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- backend ---"));
    assert!(stdout.contains("1 project(s)"));
}

#[test]
fn project_status_nonexistent_fails() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "status", "--name", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// ---------------------------------------------------------------------------
// project progress
// ---------------------------------------------------------------------------

#[test]
fn project_progress_shows_all() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "progress"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Repo Projects"), "stdout: {stdout}");
    assert!(stdout.contains("--- frontend ---"), "stdout: {stdout}");
    assert!(stdout.contains("--- backend ---"), "stdout: {stdout}");
}

#[test]
fn project_progress_with_data() {
    let tmp = setup_monorepo();
    let frontend_beu = tmp
        .path()
        .join("frontend/.beu")
        .to_string_lossy()
        .to_string();

    // Add a task to frontend.
    let output = beu_cmd(&frontend_beu)
        .args(["task", "add", "build login page"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = beu_cmd_at(tmp.path())
        .args(["project", "progress"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tasks:"), "stdout: {stdout}");
}

#[test]
fn project_progress_single_project() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "progress", "--name", "backend"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- backend ---"));
    assert!(stdout.contains("1 project(s)"));
}

#[test]
fn project_progress_nonexistent_fails() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "progress", "--name", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn project_commands_work_from_subdirectory() {
    let tmp = setup_monorepo();
    // Run from within a subproject directory.
    let output = beu_cmd_at(&tmp.path().join("frontend"))
        .args(["project", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should still discover all projects (walks up to git root).
    assert!(stdout.contains("frontend"));
    assert!(stdout.contains("backend"));
    assert!(stdout.contains("3 project(s)"));
}

#[test]
fn project_list_no_git_root_fails() {
    let tmp = TempDir::new().unwrap();
    // No .git directory.
    let output = beu_cmd_at(tmp.path())
        .args(["project", "list"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not inside a git repository"));
}

#[test]
fn project_list_empty_repo() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No beu projects"));
}

#[test]
fn project_status_shows_modules() {
    let tmp = setup_monorepo();
    let output = beu_cmd_at(tmp.path())
        .args(["project", "status", "--name", "frontend"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Default config has all modules enabled.
    assert!(stdout.contains("modules:"), "stdout: {stdout}");
    assert!(stdout.contains("journal"), "stdout: {stdout}");
    assert!(stdout.contains("task"), "stdout: {stdout}");
}
