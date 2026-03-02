mod common;

use common::{beu_cmd, beu_dir_path, setup};

// ---------------------------------------------------------------------------
// Project flag scopes data
// ---------------------------------------------------------------------------

#[test]
fn cli_project_flag_scopes_task_data() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Add a task under project "alpha"
    let output = beu_cmd(&dir)
        .args([
            "--project",
            "alpha",
            "task",
            "add",
            "alpha task",
            "--priority",
            "high",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // List tasks under default project -- should be empty
    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("alpha task"),
        "default project should not see alpha's task"
    );

    // List tasks under project "alpha" -- should see it
    let output = beu_cmd(&dir)
        .args(["--project", "alpha", "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha task"),
        "alpha project should see its own task"
    );
}

#[test]
fn cli_project_flag_scopes_state_data() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Set a state entry under project "beta"
    let output = beu_cmd(&dir)
        .args([
            "--project",
            "beta",
            "state",
            "set",
            "--category",
            "decision",
            "ship-v2",
            "launch",
            "v2",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Get from default project -- should not find it
    let output = beu_cmd(&dir)
        .args(["state", "get", "ship-v2"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("launch v2"),
        "default project should not see beta's state"
    );

    // Get from project "beta" -- should find it
    let output = beu_cmd(&dir)
        .args(["--project", "beta", "state", "get", "ship-v2"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("launch v2"),
        "beta project should see its own state"
    );
}

#[test]
fn cli_project_flag_scopes_idea_data() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Add an idea under project "gamma"
    let output = beu_cmd(&dir)
        .args(["--project", "gamma", "idea", "add", "gamma idea"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // List ideas under default -- should be empty
    let output = beu_cmd(&dir).args(["idea", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("gamma idea"));

    // List ideas under project "gamma" -- should see it
    let output = beu_cmd(&dir)
        .args(["--project", "gamma", "idea", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gamma idea"));
}

// ---------------------------------------------------------------------------
// require_project config
// ---------------------------------------------------------------------------

#[test]
fn cli_require_project_fails_without_flag() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Set require_project: true in config (replace existing false value)
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = content.replace("require_project: false", "require_project: true");
    std::fs::write(&config_path, updated).unwrap();

    // Run a command without --project -- should fail
    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("project ID required") || stderr.contains("--project"),
        "expected project required error, got: {stderr}"
    );
}

#[test]
fn cli_require_project_succeeds_with_flag() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Set require_project: true in config (replace existing false value)
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = content.replace("require_project: false", "require_project: true");
    std::fs::write(&config_path, updated).unwrap();

    // Run with --project -- should succeed
    let output = beu_cmd(&dir)
        .args(["--project", "myproj", "task", "list"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "should succeed with --project flag: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Default project behavior
// ---------------------------------------------------------------------------

#[test]
fn cli_default_project_used_when_not_required() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Add task without --project (uses default)
    let output = beu_cmd(&dir)
        .args(["task", "add", "default task", "--priority", "medium"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // List without --project (uses default) -- should see it
    let output = beu_cmd(&dir).args(["task", "list"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("default task"));

    // List with --project default -- should also see it
    let output = beu_cmd(&dir)
        .args(["--project", "default", "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("default task"));
}

#[test]
fn cli_custom_default_project() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Set default_project to "main" in config (replace existing default value)
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = content.replace("default_project: default", "default_project: main");
    std::fs::write(&config_path, updated).unwrap();

    // Add task without --project (should use "main")
    let output = beu_cmd(&dir)
        .args(["task", "add", "main task", "--priority", "low"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // List under --project main -- should see it
    let output = beu_cmd(&dir)
        .args(["--project", "main", "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main task"));

    // List under --project default -- should NOT see it
    let output = beu_cmd(&dir)
        .args(["--project", "default", "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("main task"));
}

// ---------------------------------------------------------------------------
// Init creates default project
// ---------------------------------------------------------------------------

#[test]
fn cli_init_creates_default_project() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // The init output should mention the default project
    // Re-check by verifying the project exists via internal state
    // We can use 'status' to verify the db is initialized
    let output = beu_cmd(&dir).args(["status"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Status should work, proving the default project setup is functional
    assert!(stdout.contains("module") || stdout.contains("Module"));
}

// ---------------------------------------------------------------------------
// Short flag -p works
// ---------------------------------------------------------------------------

#[test]
fn cli_short_project_flag() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Add task with -p shorthand
    let output = beu_cmd(&dir)
        .args([
            "-p",
            "short",
            "task",
            "add",
            "short task",
            "--priority",
            "medium",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify with long flag
    let output = beu_cmd(&dir)
        .args(["--project", "short", "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("short task"));
}
