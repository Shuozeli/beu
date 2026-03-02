mod common;

use std::process::Command;
use tempfile::TempDir;

use common::{beu_cmd, beu_dir_path, disable_module, set_required_docs, setup};

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

#[test]
fn cli_init_creates_directory() {
    let tmp = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialized .beu"));
    assert!(tmp.path().join(".beu/data").is_dir());
}

#[test]
fn cli_init_fails_if_already_exists() {
    let tmp = setup();
    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn cli_init_quiet_mode() {
    let tmp = TempDir::new().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("--quiet")
        .arg("init")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // In quiet mode, should produce no output.
    assert!(stdout.is_empty());
    // But the directory should still be created.
    assert!(tmp.path().join(".beu/data").is_dir());
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

#[test]
fn cli_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .arg("version")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("beu 0.1.0"));
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

#[test]
fn cli_status() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).arg("status").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("beu project"));
    assert!(stdout.contains("journal, artifact, task"));
}

#[test]
fn cli_status_after_activity() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "test"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).arg("status").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("beu project"));
    assert!(stdout.contains("last activity:"));
    assert!(stdout.contains("journal"));
    assert!(stdout.contains("total events:"));
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[test]
fn cli_events_empty() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).arg("events").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No events") || stdout.contains("event(s) shown"));
}

#[test]
fn cli_events_after_commands() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();

    let output = beu_cmd(&dir).arg("events").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("journal"));
    assert!(stdout.contains("open"));
}

#[test]
fn cli_events_with_module_filter() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["artifact", "add", "some-doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["task", "add", "some", "task"])
        .output()
        .unwrap();

    // Filter by journal only.
    let output = beu_cmd(&dir)
        .args(["events", "--module", "journal"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("journal"));
    assert!(
        !stdout.contains("artifact")
            || stdout.find("artifact").map_or(true, |pos| {
                // "artifact" might appear in header only, not in data rows
                pos < stdout.find("---").unwrap_or(0)
            })
    );
}

#[test]
fn cli_events_with_limit() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Generate multiple events.
    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "msg1"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "msg2"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "msg3"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["events", "-n", "2"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2 event(s) shown"));
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Export / Import / Reset
// ---------------------------------------------------------------------------

#[test]
fn cli_export_no_data() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Export on a module with no data should succeed with empty tables.
    let output = beu_cmd(&dir).args(["export", "journal"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("journal_sessions"));
}

#[test]
fn cli_export_with_data() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "test", "entry"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["export", "journal"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sessions"));
    assert!(stdout.contains("entries"));
}

#[test]
fn cli_export_all() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Create data in multiple modules.
    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["artifact", "add", "doc1"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["export", "--all"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain both module names as keys.
    assert!(stdout.contains("sessions")); // journal tables
    assert!(stdout.contains("artifacts")); // artifact tables
}

#[test]
fn cli_export_no_args_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["export"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("specify a module") || stderr.contains("--all"));
}

#[test]
fn cli_export_module_with_all_flag_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["export", "journal", "--all"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot use --all"));
}

#[test]
fn cli_import_from_file() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Create data.
    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "important", "entry"])
        .output()
        .unwrap();

    // Export.
    let output = beu_cmd(&dir).args(["export", "journal"]).output().unwrap();
    assert!(output.status.success());
    let exported_json = String::from_utf8_lossy(&output.stdout).to_string();

    // Write export to file.
    let json_path = tmp.path().join("export.json");
    std::fs::write(&json_path, &exported_json).unwrap();

    // Reset.
    beu_cmd(&dir)
        .args(["reset", "journal", "--force"])
        .output()
        .unwrap();

    // Import.
    let output = beu_cmd(&dir)
        .args(["import", "journal", &json_path.to_string_lossy()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Imported"));

    // Verify data is back.
    let output = beu_cmd(&dir).args(["export", "journal"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("important entry"));
}

#[test]
fn cli_import_nonexistent_file() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["import", "journal", "/tmp/nonexistent_file_xyz.json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("file not found") || stderr.contains("not found"));
}

#[test]
fn cli_reset_requires_force() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();

    let output = beu_cmd(&dir).args(["reset", "journal"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--force"));
}

#[test]
fn cli_reset_with_force() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();

    let output = beu_cmd(&dir)
        .args(["reset", "journal", "--force"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Reset"));
}

#[test]
fn cli_reset_nonexistent_module() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["reset", "nonexistent", "--force"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown module"));
}

#[test]
fn cli_reset_then_use_module() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Create data.
    beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    beu_cmd(&dir)
        .args(["journal", "log", "before", "reset"])
        .output()
        .unwrap();

    // Reset.
    beu_cmd(&dir)
        .args(["reset", "journal", "--force"])
        .output()
        .unwrap();

    // Module should work again after reset (schema auto-recreated).
    let output = beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    assert!(output.status.success());

    let output = beu_cmd(&dir)
        .args(["journal", "log", "after", "reset"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = beu_cmd(&dir).args(["journal", "summary"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("after reset"));
    assert!(!stdout.contains("before reset"));
}

// ---------------------------------------------------------------------------
// Verbose / Quiet / No .beu dir
// ---------------------------------------------------------------------------

#[test]
fn cli_verbose_and_quiet_conflict() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .arg("--beu-dir")
        .arg(&dir)
        .arg("--verbose")
        .arg("--quiet")
        .arg("status")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("mutually exclusive"));
}

#[test]
fn cli_no_beu_dir_errors() {
    let tmp = TempDir::new().unwrap();
    // Don't init -- no .beu dir.

    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .args(["journal", "open"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no .beu directory"));
}

// ---------------------------------------------------------------------------
// Pause / Resume / Progress
// ---------------------------------------------------------------------------

#[test]
fn cli_pause_and_resume() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir)
        .args(["pause", "working", "on", "auth", "module"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Checkpoint saved"));

    let output = beu_cmd(&dir).args(["resume"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Checkpoint: working on auth module"));

    // Second resume should show no checkpoint.
    let output = beu_cmd(&dir).args(["resume"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No checkpoint"));
}

#[test]
fn cli_resume_shows_blockers_and_focus() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["state", "set", "--category", "blocker", "ci", "flaky tests"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args([
            "state",
            "set",
            "--category",
            "focus",
            "current-work",
            "auth module",
        ])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["resume"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Blockers:"));
    assert!(stdout.contains("ci: flaky tests"));
    assert!(stdout.contains("Focus:"));
    assert!(stdout.contains("current-work: auth module"));
}

#[test]
fn cli_progress_summary() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Add some data across modules.
    beu_cmd(&dir)
        .args(["task", "add", "task", "one"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["idea", "add", "idea", "one"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "add", "doc1"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["pause", "checkpoint", "msg"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["progress"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Progress Summary"));
    assert!(stdout.contains("Checkpoint:"));
    assert!(stdout.contains("Tasks:"));
    assert!(stdout.contains("Artifacts:"));
    assert!(stdout.contains("Ideas:"));
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

#[test]
fn cli_health_clean() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).args(["health"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[ok] data/ directory exists"));
    assert!(stdout.contains("All checks passed"));
}

#[test]
fn cli_health_with_module_dbs() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    beu_cmd(&dir)
        .args(["task", "add", "task"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).args(["health"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[ok] beu.db exists"));
    assert!(stdout.contains("[ok] integrity check passed"));
}

// ---------------------------------------------------------------------------
// Config: module gating
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires npx skills and network access to GitHub"]
fn cli_init_creates_agent_rules() {
    let tmp = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();

    assert!(tmp.path().join(".claude/rules/beu.md").exists());
    assert!(tmp.path().join(".gemini/rules/beu.md").exists());
    assert!(tmp.path().join(".agent/rules/beu.md").exists());

    let content = std::fs::read_to_string(tmp.path().join(".claude/rules/beu.md")).unwrap();
    assert!(content.contains("Session Protocol"));
    assert!(content.contains("beu resume"));
}

#[test]
#[ignore = "requires npx skills and network access to GitHub"]
fn cli_init_skips_existing_agent_rules() {
    let tmp = TempDir::new().unwrap();

    // Pre-create a custom claude rule.
    let claude_dir = tmp.path().join(".claude/rules");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(claude_dir.join("beu.md"), "custom rules").unwrap();

    Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();

    // Custom content should be preserved.
    let content = std::fs::read_to_string(tmp.path().join(".claude/rules/beu.md")).unwrap();
    assert_eq!(content, "custom rules");

    // Other agent rules should still be created.
    assert!(tmp.path().join(".gemini/rules/beu.md").exists());
    assert!(tmp.path().join(".agent/rules/beu.md").exists());
}

#[test]
fn cli_init_creates_config_yml() {
    let tmp = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();

    let config_path = tmp.path().join(".beu/config.yml");
    assert!(config_path.exists());

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("journal: true"));
    assert!(content.contains("artifact: true"));
    assert!(content.contains("task: true"));
    assert!(content.contains("state: true"));
    assert!(content.contains("idea: true"));
    assert!(content.contains("debug: true"));
}

#[test]
fn cli_disabled_journal_returns_error() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "journal");

    let output = beu_cmd(&dir).args(["journal", "open"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_disabled_artifact_returns_error() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "artifact");

    let output = beu_cmd(&dir)
        .args(["artifact", "add", "doc1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_disabled_task_returns_error() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "task");

    let output = beu_cmd(&dir)
        .args(["task", "add", "test"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_disabled_state_returns_error() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "state");

    let output = beu_cmd(&dir)
        .args(["state", "set", "--category", "note", "k", "v"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_disabled_idea_returns_error() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "idea");

    let output = beu_cmd(&dir)
        .args(["idea", "add", "test"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_disabled_debug_returns_error() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "debug");

    let output = beu_cmd(&dir)
        .args(["debug", "open", "test", "bug"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_pause_gated_on_state() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "state");

    let output = beu_cmd(&dir)
        .args(["pause", "checkpoint"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_resume_gated_on_state() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "state");

    let output = beu_cmd(&dir).args(["resume"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not enabled"));
}

#[test]
fn cli_status_shows_enabled_modules() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    disable_module(&tmp, "journal");
    disable_module(&tmp, "debug");

    let output = beu_cmd(&dir).arg("status").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show only enabled modules.
    assert!(stdout.contains("artifact"));
    assert!(stdout.contains("task"));
    assert!(stdout.contains("state"));
    assert!(stdout.contains("idea"));
    // Disabled modules should not appear in the modules line.
    // Find the "modules:" line and check it doesn't contain journal/debug.
    for line in stdout.lines() {
        if line.contains("modules:") {
            assert!(!line.contains("journal"), "journal should be disabled");
            assert!(!line.contains("debug"), "debug should be disabled");
            break;
        }
    }
}

#[test]
fn cli_system_commands_work_with_disabled_modules() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Disable a module but system commands should still work.
    disable_module(&tmp, "journal");

    let output = beu_cmd(&dir).arg("events").output().unwrap();
    assert!(output.status.success());

    let output = beu_cmd(&dir).arg("health").output().unwrap();
    assert!(output.status.success());

    let output = beu_cmd(&dir).args(["export", "--all"]).output().unwrap();
    assert!(output.status.success());
}

// ---------------------------------------------------------------------------
// Check (compliance gate)
// ---------------------------------------------------------------------------

#[test]
fn cli_check_no_required_docs() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No required docs configured"));
}

#[test]
fn cli_check_all_satisfied() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(&tmp, &[("design", "doc"), ("changelog", "changelog")]);

    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "design", "in-progress"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "add", "changelog", "--type", "changelog"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "changelog", "done"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All 2 required docs satisfied"));
}

#[test]
fn cli_check_missing_artifact_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(&tmp, &[("design", "doc"), ("changelog", "changelog")]);

    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "design", "in-progress"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required doc 'changelog' not registered"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("beu artifact add changelog --type changelog"),
        "stderr: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1/2 required docs satisfied"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_check_pending_artifact_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(&tmp, &[("design", "doc")]);

    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required doc 'design' is still 'pending'"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("beu artifact status design in-progress"),
        "stderr: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0/1 required docs satisfied"),
        "stdout: {stdout}"
    );
}

#[test]
fn cli_check_mixed_errors() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(
        &tmp,
        &[
            ("design", "doc"),
            ("changelog", "changelog"),
            ("usage-guide", "doc"),
        ],
    );

    // design: in-progress (passes)
    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "design", "in-progress"])
        .output()
        .unwrap();

    // changelog: pending (fails)
    beu_cmd(&dir)
        .args(["artifact", "add", "changelog", "--type", "changelog"])
        .output()
        .unwrap();

    // usage-guide: not registered (fails)

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("'changelog' is still 'pending'"));
    assert!(stderr.contains("'usage-guide' not registered"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1/3 required docs satisfied"));
}

#[test]
fn cli_check_artifact_module_disabled_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(&tmp, &[("design", "doc")]);
    disable_module(&tmp, "artifact");

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("artifact module"), "stderr: {stderr}");
}

#[test]
fn cli_check_review_and_done_pass() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(&tmp, &[("design", "doc"), ("changelog", "changelog")]);

    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "design", "review"])
        .output()
        .unwrap();

    beu_cmd(&dir)
        .args(["artifact", "add", "changelog", "--type", "changelog"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "changelog", "done"])
        .output()
        .unwrap();

    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("All 2 required docs satisfied"));
}

// ---------------------------------------------------------------------------
// Check -- staleness detection
// ---------------------------------------------------------------------------

#[test]
fn cli_check_stale_doc_fails() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Configure required docs + staleness threshold
    set_required_docs(&tmp, &[("design", "doc")]);
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = format!("{content}staleness_threshold: 5\n");
    std::fs::write(&config_path, updated).unwrap();

    // Add artifact and mark done
    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "design", "done"])
        .output()
        .unwrap();

    // Generate mutation events (tasks create event log entries)
    for i in 1..=6 {
        beu_cmd(&dir)
            .args(["task", "add", &format!("task {i}"), "--priority", "medium"])
            .output()
            .unwrap();
    }

    // Check should fail: 6 mutations >= threshold 5
    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("stale"),
        "expected stale error, got: {stderr}"
    );
}

#[test]
fn cli_check_stale_doc_fixed_by_changelog() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Configure required docs + staleness threshold
    set_required_docs(&tmp, &[("design", "doc")]);
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = format!("{content}staleness_threshold: 5\n");
    std::fs::write(&config_path, updated).unwrap();

    // Add artifact and mark done
    beu_cmd(&dir)
        .args(["artifact", "add", "design", "--type", "doc"])
        .output()
        .unwrap();
    beu_cmd(&dir)
        .args(["artifact", "status", "design", "done"])
        .output()
        .unwrap();

    // Generate mutation events
    for i in 1..=6 {
        beu_cmd(&dir)
            .args(["task", "add", &format!("task {i}"), "--priority", "medium"])
            .output()
            .unwrap();
    }

    // Now update the doc via changelog (refreshes updated_at)
    beu_cmd(&dir)
        .args([
            "artifact",
            "changelog",
            "design",
            "updated after task changes",
        ])
        .output()
        .unwrap();

    // Check should pass now: updated_at is after the mutation events
    let output = beu_cmd(&dir).arg("check").output().unwrap();
    assert!(
        output.status.success(),
        "expected check to pass after changelog update: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// Update-rules
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires npx skills and network access to GitHub"]
fn cli_update_rules_overwrites_all_three() {
    let tmp = TempDir::new().unwrap();

    // Init first so rule dirs exist.
    Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();

    // Stale out the content.
    std::fs::write(tmp.path().join(".claude/rules/beu.md"), "old").unwrap();
    std::fs::write(tmp.path().join(".gemini/rules/beu.md"), "old").unwrap();
    std::fs::write(tmp.path().join(".agent/rules/beu.md"), "old").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("update-rules")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Updated agent rules (3)"),
        "stdout: {stdout}"
    );

    // All three files should now have the fresh content.
    for path in &[
        ".claude/rules/beu.md",
        ".gemini/rules/beu.md",
        ".agent/rules/beu.md",
    ] {
        let content = std::fs::read_to_string(tmp.path().join(path)).unwrap();
        assert!(
            content.contains("Test Status Tracking"),
            "{path} missing Test Status Tracking section"
        );
        assert!(
            content.contains("beu update-rules"),
            "{path} missing update-rules reference"
        );
    }
}

#[test]
#[ignore = "requires npx skills and network access to GitHub"]
fn cli_update_rules_creates_missing_dirs() {
    let tmp = TempDir::new().unwrap();

    // Init first (creates .beu), but remove .claude/rules manually.
    Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();
    std::fs::remove_dir_all(tmp.path().join(".claude")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("update-rules")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(tmp.path().join(".claude/rules/beu.md").exists());
}

#[test]
#[ignore = "requires npx skills and network access to GitHub"]
fn cli_update_rules_quiet_mode() {
    let tmp = TempDir::new().unwrap();

    Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .arg("init")
        .output()
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_beu"))
        .current_dir(tmp.path())
        .args(["--quiet", "update-rules"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "quiet mode should produce no output"
    );
}
