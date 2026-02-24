//! CLI-level integration tests that spawn the actual `bea` binary.
//!
//! Prerequisites: plugins must be pre-built via:
//!   cargo build -p journal -p track -p agile --target wasm32-unknown-unknown --release
//! And the host binary must be built:
//!   cargo build -p bea-host

use std::path::PathBuf;
use std::process::Command;

/// Locate the bea binary built by cargo.
fn bea_bin() -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    // Use the debug build (since tests build in debug mode).
    let bin = workspace_root.join("target/debug/bea");
    assert!(
        bin.exists(),
        "bea binary not found at {}. Run: cargo build -p bea-host",
        bin.display()
    );
    bin
}

/// Locate a compiled wasm artifact by plugin name.
fn wasm_path(name: &str) -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let wasm = workspace_root.join(format!(
        "target/wasm32-unknown-unknown/release/{name}.wasm"
    ));
    assert!(
        wasm.exists(),
        "{name}.wasm not found. Run: cargo build -p {name} --target wasm32-unknown-unknown --release",
    );
    wasm
}

/// Run bea with args in the given directory.
fn run_bea(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(bea_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute bea")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Set up a temp dir with .bea initialized and plugins installed.
fn setup_cli_dir() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();

    let output = run_bea(tmp.path(), &["init", "-q"]);
    assert!(output.status.success(), "init failed: {}", stderr(&output));

    for name in &["journal", "track", "agile"] {
        let src = wasm_path(name);
        std::fs::copy(
            &src,
            tmp.path()
                .join(format!(".bea/plugins/{name}.wasm")),
        )
        .unwrap();
    }

    tmp
}

// ---------------------------------------------------------------------------
// Init tests
// ---------------------------------------------------------------------------

#[test]
fn cli_init_creates_bea_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = run_bea(tmp.path(), &["init"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Initialized .bea"));
    assert!(tmp.path().join(".bea/plugins").is_dir());
    assert!(tmp.path().join(".bea/data").is_dir());
    assert!(tmp.path().join(".bea/config").is_dir());
    assert!(tmp.path().join(".bea/skill.md").exists());
}

#[test]
fn cli_init_fails_if_already_exists() {
    let tmp = tempfile::TempDir::new().unwrap();
    run_bea(tmp.path(), &["init", "-q"]);
    let output = run_bea(tmp.path(), &["init"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("already exists"));
}

// ---------------------------------------------------------------------------
// List tests
// ---------------------------------------------------------------------------

#[test]
fn cli_list_shows_plugins() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["list"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("journal"));
    assert!(out.contains("track"));
    assert!(out.contains("agile"));
    assert!(out.contains("PLUGIN"));
}

#[test]
fn cli_list_empty_shows_message() {
    let tmp = tempfile::TempDir::new().unwrap();
    run_bea(tmp.path(), &["init", "-q"]);
    let output = run_bea(tmp.path(), &["list"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("No plugins installed"));
}

// ---------------------------------------------------------------------------
// Install / Uninstall tests
// ---------------------------------------------------------------------------

#[test]
fn cli_install_and_uninstall() {
    let tmp = tempfile::TempDir::new().unwrap();
    run_bea(tmp.path(), &["init", "-q"]);

    let journal_wasm = wasm_path("journal");
    let output = run_bea(
        tmp.path(),
        &["install", journal_wasm.to_str().unwrap()],
    );
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Installed journal"));
    assert!(out.contains("Updated skill.md"));

    // Verify skill.md now has journal commands.
    let skill = std::fs::read_to_string(tmp.path().join(".bea/skill.md")).unwrap();
    assert!(skill.contains("bea journal open"));

    // Uninstall.
    let output = run_bea(tmp.path(), &["uninstall", "journal"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Removed journal.wasm"));

    // List should show no plugins.
    let output = run_bea(tmp.path(), &["list"]);
    let out = stdout(&output);
    assert!(out.contains("No plugins installed"));
}

// ---------------------------------------------------------------------------
// Plugin dispatch tests
// ---------------------------------------------------------------------------

#[test]
fn cli_journal_workflow() {
    let tmp = setup_cli_dir();

    let output = run_bea(tmp.path(), &["journal", "open"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("opened"));

    let output = run_bea(tmp.path(), &["journal", "log", "test message"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Logged: test message"));

    let output = run_bea(
        tmp.path(),
        &["journal", "note", "--tag", "decision", "use Rust"],
    );
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("[decision]"));

    let output = run_bea(tmp.path(), &["journal", "summary"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("test message"));
    assert!(out.contains("[decision]"));

    let output = run_bea(tmp.path(), &["journal", "close"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("closed"));
}

#[test]
fn cli_unknown_plugin_fails() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["nonexistent", "cmd"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("nonexistent"));
}

// ---------------------------------------------------------------------------
// Plugin help (no command) tests
// ---------------------------------------------------------------------------

#[test]
fn cli_plugin_help_shows_commands() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["journal"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("journal v0.1.0"));
    assert!(out.contains("Commands:"));
    assert!(out.contains("bea journal open"));
    assert!(out.contains("bea journal log"));
}

// ---------------------------------------------------------------------------
// Status tests
// ---------------------------------------------------------------------------

#[test]
fn cli_status_shows_overview() {
    let tmp = setup_cli_dir();

    // Run a command to create some data.
    run_bea(tmp.path(), &["journal", "open"]);

    let output = run_bea(tmp.path(), &["status"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("plugins:"));
    assert!(out.contains("16 commands"));
    assert!(out.contains("total events:"));
}

// ---------------------------------------------------------------------------
// Events tests
// ---------------------------------------------------------------------------

#[test]
fn cli_events_shows_log() {
    let tmp = setup_cli_dir();

    run_bea(tmp.path(), &["journal", "open"]);
    run_bea(tmp.path(), &["journal", "close"]);

    let output = run_bea(tmp.path(), &["events"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("journal"));
    assert!(out.contains("open"));
    assert!(out.contains("close"));
    assert!(out.contains("event(s) shown"));
}

#[test]
fn cli_events_filter_by_plugin() {
    let tmp = setup_cli_dir();

    run_bea(tmp.path(), &["journal", "open"]);
    run_bea(tmp.path(), &["track", "add", "doc"]);

    let output = run_bea(tmp.path(), &["events", "--plugin", "track"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("track"));
    assert!(!out.contains("journal"));
}

// ---------------------------------------------------------------------------
// Skill export tests
// ---------------------------------------------------------------------------

#[test]
fn cli_skill_export_generates_markdown() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["skill", "export"]);
    assert!(output.status.success());

    let skill = std::fs::read_to_string(tmp.path().join(".bea/skill.md")).unwrap();
    assert!(skill.contains("# Agent Skill Manifest"));
    assert!(skill.contains("bea journal open"));
    assert!(skill.contains("bea track add"));
    assert!(skill.contains("bea agile add"));
}

#[test]
fn cli_skill_info_outputs_json() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["skill", "info"]);
    assert!(output.status.success());
    let out = stdout(&output);
    // Should be valid JSON.
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["plugins"].is_array());
    assert!(parsed["system_commands"].is_array());
}

// ---------------------------------------------------------------------------
// Completions test
// ---------------------------------------------------------------------------

#[test]
fn cli_completions_generates_output() {
    let tmp = tempfile::TempDir::new().unwrap();
    // Completions don't need .bea to exist.
    let output = run_bea(tmp.path(), &["completions", "bash"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("_bea"));
    assert!(out.contains("COMPREPLY"));
}

// ---------------------------------------------------------------------------
// Config tests
// ---------------------------------------------------------------------------

#[test]
fn cli_config_show_empty() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["config", "journal"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("No configuration"));
}

#[test]
fn cli_config_set_and_get() {
    let tmp = setup_cli_dir();

    // Set a value.
    let output = run_bea(tmp.path(), &["config", "journal", "greeting", "hello"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Set journal.greeting = hello"));

    // Get the value.
    let output = run_bea(tmp.path(), &["config", "journal", "greeting"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert_eq!(out.trim(), "hello");

    // Show all.
    let output = run_bea(tmp.path(), &["config", "journal"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("greeting = hello"));
}

#[test]
fn cli_config_nested_key() {
    let tmp = setup_cli_dir();

    let output = run_bea(tmp.path(), &["config", "journal", "db.pool_size", "5"]);
    assert!(output.status.success());

    let output = run_bea(tmp.path(), &["config", "journal", "db.pool_size"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert_eq!(out.trim(), "5");
}

#[test]
fn cli_config_delete_key() {
    let tmp = setup_cli_dir();

    run_bea(tmp.path(), &["config", "journal", "foo", "bar"]);
    let output = run_bea(tmp.path(), &["config", "journal", "foo", "--delete"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Deleted journal.foo"));

    // Getting deleted key should fail.
    let output = run_bea(tmp.path(), &["config", "journal", "foo"]);
    assert!(!output.status.success());
}

#[test]
fn cli_config_unknown_plugin_fails() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["config", "nonexistent"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("not installed"));
}

// ---------------------------------------------------------------------------
// Export tests
// ---------------------------------------------------------------------------

#[test]
fn cli_export_plugin_data() {
    let tmp = setup_cli_dir();

    // Create some data via track.
    run_bea(tmp.path(), &["track", "add", "my-doc"]);
    run_bea(tmp.path(), &["track", "add", "my-spec", "--type", "spec"]);

    let output = run_bea(tmp.path(), &["export", "track"]);
    assert!(output.status.success(), "export failed: {}", stderr(&output));
    let out = stdout(&output);

    // Should be valid JSON.
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let artifacts = parsed["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 2);
    assert!(artifacts.iter().any(|a| a["name"] == "my-doc"));
    assert!(artifacts.iter().any(|a| a["name"] == "my-spec"));
}

#[test]
fn cli_export_all_plugins() {
    let tmp = setup_cli_dir();

    // Create data in two plugins.
    run_bea(tmp.path(), &["track", "add", "doc-a"]);
    run_bea(tmp.path(), &["journal", "open"]);

    let output = run_bea(tmp.path(), &["export", "--all"]);
    assert!(output.status.success(), "export --all failed: {}", stderr(&output));
    let out = stdout(&output);

    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["track"].is_object());
    assert!(parsed["journal"].is_object());
}

#[test]
fn cli_export_no_args_fails() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["export"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("specify a plugin name") || err.contains("required"));
}

// ---------------------------------------------------------------------------
// Import tests
// ---------------------------------------------------------------------------

#[test]
fn cli_import_restores_exported_data() {
    let tmp = setup_cli_dir();

    // Create some data.
    run_bea(tmp.path(), &["track", "add", "doc-a"]);
    run_bea(tmp.path(), &["track", "add", "doc-b"]);

    // Export to file.
    let export_output = run_bea(tmp.path(), &["export", "track"]);
    assert!(export_output.status.success());
    let json_data = stdout(&export_output);
    let export_file = tmp.path().join("track_backup.json");
    std::fs::write(&export_file, &json_data).unwrap();

    // Reset the plugin.
    let output = run_bea(tmp.path(), &["reset", "track", "--force"]);
    assert!(output.status.success(), "reset failed: {}", stderr(&output));

    // Import the backup.
    let output = run_bea(
        tmp.path(),
        &["import", "track", export_file.to_str().unwrap()],
    );
    assert!(output.status.success(), "import failed: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("Imported"));
    assert!(out.contains("rows"));

    // Verify data is back.
    let output = run_bea(tmp.path(), &["track", "list"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("doc-a"));
    assert!(out.contains("doc-b"));
}

#[test]
fn cli_import_missing_file_fails() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["import", "track", "/nonexistent.json"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("not found"));
}

// ---------------------------------------------------------------------------
// Reset tests
// ---------------------------------------------------------------------------

#[test]
fn cli_reset_clears_data() {
    let tmp = setup_cli_dir();

    run_bea(tmp.path(), &["track", "add", "doc-x"]);

    let output = run_bea(tmp.path(), &["reset", "track", "--force"]);
    assert!(output.status.success(), "reset failed: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("Reset"));
    assert!(out.contains("dropped"));

    // Data should be gone -- track list should show nothing.
    let output = run_bea(tmp.path(), &["track", "list"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("No artifacts"));
}

#[test]
fn cli_reset_without_force_fails() {
    let tmp = setup_cli_dir();
    run_bea(tmp.path(), &["track", "add", "doc-y"]);

    let output = run_bea(tmp.path(), &["reset", "track"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("--force"));
}

// ---------------------------------------------------------------------------
// Run (batch script) tests
// ---------------------------------------------------------------------------

#[test]
fn cli_run_executes_script() {
    let tmp = setup_cli_dir();

    let script = tmp.path().join("test.bea");
    std::fs::write(
        &script,
        "# Open a journal session and log some entries\n\
         journal open\n\
         journal log hello from script\n\
         \n\
         # Track an artifact\n\
         track add my-doc\n",
    )
    .unwrap();

    let output = run_bea(tmp.path(), &["run", script.to_str().unwrap()]);
    assert!(output.status.success(), "run failed: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains(">> journal open"));
    assert!(out.contains(">> journal log"));
    assert!(out.contains(">> track add"));
    assert!(out.contains("Script complete: 3 executed, 0 error(s)"));
}

#[test]
fn cli_run_fail_fast_stops_on_error() {
    let tmp = setup_cli_dir();

    let script = tmp.path().join("fail.bea");
    // journal log without an open session should fail
    std::fs::write(
        &script,
        "journal log this will fail\n\
         journal open\n",
    )
    .unwrap();

    let output = run_bea(
        tmp.path(),
        &["run", "--fail-fast", script.to_str().unwrap()],
    );
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("script failed at line 1"));
}

#[test]
fn cli_run_missing_script_fails() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["run", "/nonexistent/script.bea"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("not found"));
}

// ---------------------------------------------------------------------------
// Logs tests
// ---------------------------------------------------------------------------

#[test]
fn cli_logs_shows_plugin_output() {
    let tmp = setup_cli_dir();

    // Run some commands that produce host_log calls.
    run_bea(tmp.path(), &["journal", "open"]);
    run_bea(tmp.path(), &["track", "add", "my-doc"]);

    let output = run_bea(tmp.path(), &["logs"]);
    assert!(output.status.success(), "logs failed: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("PLUGIN"));
    assert!(out.contains("LEVEL"));
    assert!(out.contains("log entry"));
}

#[test]
fn cli_logs_filter_by_plugin() {
    let tmp = setup_cli_dir();

    run_bea(tmp.path(), &["journal", "open"]);
    run_bea(tmp.path(), &["track", "add", "doc-x"]);

    let output = run_bea(tmp.path(), &["logs", "--plugin", "track"]);
    assert!(output.status.success());
    let out = stdout(&output);
    // Should only show track logs.
    if out.contains("track") {
        // Lines should not contain journal (except in the header).
        let data_lines: Vec<&str> = out.lines().skip(2).collect(); // Skip header + separator
        for line in &data_lines {
            if line.starts_with('-') || line.is_empty() || line.contains("log entry") {
                continue;
            }
            assert!(
                line.contains("track"),
                "expected only track entries, got: {line}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Version test
// ---------------------------------------------------------------------------

#[test]
fn cli_version_shows_info() {
    let tmp = setup_cli_dir();
    let output = run_bea(tmp.path(), &["version"]);
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("bea 0.1.0"));
    assert!(out.contains("plugins:"));
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn cli_no_bea_dir_fails() {
    let tmp = tempfile::TempDir::new().unwrap();
    let output = run_bea(tmp.path(), &["list"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("no .bea directory found"));
}

#[test]
fn cli_verbose_and_quiet_conflict() {
    let tmp = tempfile::TempDir::new().unwrap();
    run_bea(tmp.path(), &["init", "-q"]);
    let output = run_bea(tmp.path(), &["-v", "-q", "list"]);
    assert!(!output.status.success());
    let err = stderr(&output);
    assert!(err.contains("mutually exclusive"));
}
