//! Integration tests that load compiled wasm plugins and exercise
//! the full host-plugin flow programmatically.
//!
//! Prerequisites: plugins must be pre-built via:
//!   cargo build -p journal -p track -p agile --target wasm32-unknown-unknown --release

use std::path::PathBuf;

use bea_host::db::EventLog;
use bea_host::plugin_manager::PluginRegistry;
use bea_host::skill;
use bea_sdk::{CommandStatus, PluginMetadata};

/// Locate a compiled wasm artifact by plugin name.
fn wasm_path(name: &str) -> PathBuf {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let wasm = workspace_root.join(format!(
        "target/wasm32-unknown-unknown/release/{name}.wasm"
    ));
    assert!(
        wasm.exists(),
        "{name}.wasm not found at {}. Run: cargo build -p {name} --target wasm32-unknown-unknown --release",
        wasm.display()
    );
    wasm
}

/// Create a temp .bea directory with all plugins installed.
fn setup_bea_dir() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let bea_dir = tmp.path().join(".bea");
    std::fs::create_dir_all(bea_dir.join("plugins")).unwrap();
    std::fs::create_dir_all(bea_dir.join("data")).unwrap();

    for name in &["journal", "track", "agile"] {
        let src = wasm_path(name);
        std::fs::copy(&src, bea_dir.join(format!("plugins/{name}.wasm"))).unwrap();
    }

    tmp
}

/// Create a temp .bea directory with only the journal plugin.
fn setup_journal_only() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let bea_dir = tmp.path().join(".bea");
    std::fs::create_dir_all(bea_dir.join("plugins")).unwrap();
    std::fs::create_dir_all(bea_dir.join("data")).unwrap();

    let src = wasm_path("journal");
    std::fs::copy(&src, bea_dir.join("plugins/journal.wasm")).unwrap();

    tmp
}

fn bea_dir(tmp: &tempfile::TempDir) -> PathBuf {
    tmp.path().join(".bea")
}

// ---------------------------------------------------------------------------
// Metadata tests
// ---------------------------------------------------------------------------

#[test]
fn all_plugins_load_with_correct_metadata() {
    let tmp = setup_bea_dir();
    let registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();
    let all = registry.all_metadata();

    assert_eq!(all.len(), 3);
    let names: Vec<&str> = all.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"journal"));
    assert!(names.contains(&"track"));
    assert!(names.contains(&"agile"));
}

#[test]
fn plugin_metadata_has_correct_name_and_version() {
    let tmp = setup_journal_only();
    let registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();
    let all = registry.all_metadata();

    assert_eq!(all.len(), 1);
    let meta: &PluginMetadata = &all[0];
    assert_eq!(meta.name, "journal");
    assert_eq!(meta.version, "0.1.0");
}

#[test]
fn plugin_metadata_has_all_commands() {
    let tmp = setup_journal_only();
    let registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();
    let meta = &registry.all_metadata()[0];

    let cmd_names: Vec<&str> = meta.commands.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(cmd_names, vec!["open", "log", "note", "summary", "close"]);
}

// ---------------------------------------------------------------------------
// Skill export tests
// ---------------------------------------------------------------------------

#[test]
fn skill_export_generates_markdown() {
    let tmp = setup_bea_dir();
    let dir = bea_dir(&tmp);
    let registry = PluginRegistry::load(&dir, false).unwrap();

    skill::export(&dir, &registry, false).unwrap();

    let skill_md = std::fs::read_to_string(dir.join("skill.md")).unwrap();
    assert!(skill_md.contains("# Agent Skill Manifest"));
    assert!(skill_md.contains("## Journal (v0.1.0)"));
    assert!(skill_md.contains("`bea journal open`"));
    assert!(skill_md.contains("`bea journal log <message>`"));
    assert!(skill_md.contains("`bea journal note"));
    assert!(skill_md.contains("`bea journal summary`"));
    assert!(skill_md.contains("`bea journal close`"));
}

// ---------------------------------------------------------------------------
// Journal command workflow tests
// ---------------------------------------------------------------------------

#[test]
fn journal_open_creates_session() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let output = registry.dispatch("journal", "open", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("opened"));

    let data = output.data.unwrap();
    assert!(data.get("session_id").is_some());
}

#[test]
fn journal_log_requires_open_session() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    // log without open should fail
    let output = registry.dispatch("journal", "log", &["test".into()]);
    assert!(output.is_err());
}

#[test]
fn journal_log_records_message() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();

    let output = registry
        .dispatch("journal", "log", &["hello world".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("hello world"));
}

#[test]
fn journal_note_with_tag() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();

    let output = registry
        .dispatch(
            "journal",
            "note",
            &["--tag".into(), "decision".into(), "use Rust".into()],
        )
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("[decision]"));
    assert!(output.message.contains("use Rust"));
}

#[test]
fn journal_summary_shows_entries() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();
    registry
        .dispatch("journal", "log", &["first entry".into()])
        .unwrap();
    registry
        .dispatch(
            "journal",
            "note",
            &["--tag".into(), "blocker".into(), "need API key".into()],
        )
        .unwrap();

    let output = registry.dispatch("journal", "summary", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("first entry"));
    assert!(output.message.contains("[blocker]"));
    assert!(output.message.contains("need API key"));

    let data = output.data.unwrap();
    assert_eq!(data["entry_count"], 2);
}

#[test]
fn journal_close_ends_session() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();
    registry
        .dispatch("journal", "log", &["some work".into()])
        .unwrap();

    let output = registry.dispatch("journal", "close", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("closed"));
}

#[test]
fn journal_close_then_log_fails() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();
    registry.dispatch("journal", "close", &[]).unwrap();

    // After close, log should fail (no open session).
    let output = registry.dispatch("journal", "log", &["late entry".into()]);
    assert!(output.is_err());
}

#[test]
fn journal_full_workflow() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    // open
    let open = registry.dispatch("journal", "open", &[]).unwrap();
    assert_eq!(open.status, CommandStatus::Ok);
    let session_id = open.data.unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    // log
    let log1 = registry
        .dispatch("journal", "log", &["explored codebase".into()])
        .unwrap();
    assert_eq!(log1.status, CommandStatus::Ok);

    // note
    let note = registry
        .dispatch(
            "journal",
            "note",
            &["--tag".into(), "decision".into(), "use composition".into()],
        )
        .unwrap();
    assert_eq!(note.status, CommandStatus::Ok);

    // summary
    let summary = registry.dispatch("journal", "summary", &[]).unwrap();
    assert_eq!(summary.status, CommandStatus::Ok);
    assert!(summary.message.contains(&session_id));
    assert!(summary.message.contains("explored codebase"));
    assert!(summary.message.contains("[decision]"));
    assert_eq!(summary.data.unwrap()["entry_count"], 2);

    // close
    let close = registry.dispatch("journal", "close", &[]).unwrap();
    assert_eq!(close.status, CommandStatus::Ok);

    // verify DB file exists
    let db_path = bea_dir(&tmp).join("data/journal.db");
    assert!(db_path.exists());
}

// ---------------------------------------------------------------------------
// Error handling tests
// ---------------------------------------------------------------------------

#[test]
fn dispatch_unknown_plugin_returns_error() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let result = registry.dispatch("nonexistent", "open", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("nonexistent"));
    // With plugins loaded, error should list available plugins.
    assert!(err.contains("available:"));
    assert!(err.contains("journal"));
}

#[test]
fn journal_unknown_command_returns_error() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let output = registry
        .dispatch("journal", "nonexistent", &[])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Error);
    assert!(output.message.contains("unknown command"));
}

#[test]
fn journal_log_empty_args_returns_error() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();

    let output = registry.dispatch("journal", "log", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Error);
    assert!(output.message.contains("usage"));
}

#[test]
fn journal_note_missing_tag_returns_error() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();

    // note without --tag
    let result = registry.dispatch("journal", "note", &["just a message".into()]);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Empty plugin directory tests
// ---------------------------------------------------------------------------

/// Create a .bea dir with no plugins installed.
fn setup_empty_bea_dir() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().unwrap();
    let bea_dir = tmp.path().join(".bea");
    std::fs::create_dir_all(bea_dir.join("plugins")).unwrap();
    std::fs::create_dir_all(bea_dir.join("data")).unwrap();
    tmp
}

#[test]
fn empty_plugins_loads_ok() {
    let tmp = setup_empty_bea_dir();
    let registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();
    assert!(registry.all_metadata().is_empty());
}

#[test]
fn empty_plugins_skill_export_generates_system_commands_only() {
    let tmp = setup_empty_bea_dir();
    let dir = bea_dir(&tmp);
    let registry = PluginRegistry::load(&dir, false).unwrap();

    skill::export(&dir, &registry, false).unwrap();

    let skill_md = std::fs::read_to_string(dir.join("skill.md")).unwrap();
    assert!(skill_md.contains("# Agent Skill Manifest"));
    assert!(skill_md.contains("bea skill export"));
    // No plugin sections should be present.
    assert!(!skill_md.contains("## Journal"));
}

#[test]
fn empty_plugins_dispatch_gives_helpful_error() {
    let tmp = setup_empty_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let result = registry.dispatch("anything", "cmd", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no plugins loaded"));
}

// ---------------------------------------------------------------------------
// Config tests
// ---------------------------------------------------------------------------

#[test]
fn plugin_loads_with_config_file() {
    let tmp = setup_journal_only();
    let dir = bea_dir(&tmp);
    // Create a config file for the journal plugin.
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::write(
        dir.join("config/journal.toml"),
        "default_tag = \"observation\"\n",
    )
    .unwrap();

    // Plugin should still load successfully with config present.
    let registry = PluginRegistry::load(&dir, false).unwrap();
    assert_eq!(registry.all_metadata().len(), 1);
    assert_eq!(registry.all_metadata()[0].name, "journal");
}

#[test]
fn plugin_loads_without_config_file() {
    // Default setup has no config dir - should work fine.
    let tmp = setup_journal_only();
    let registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();
    assert_eq!(registry.all_metadata().len(), 1);
}

// ---------------------------------------------------------------------------
// Track plugin tests
// ---------------------------------------------------------------------------

#[test]
fn track_add_and_list() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let output = registry
        .dispatch("track", "add", &["architecture".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("architecture"));
    assert!(output.message.contains("pending"));

    let output = registry
        .dispatch("track", "add", &["design-doc".into(), "--type".into(), "spec".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("design-doc"));

    let output = registry.dispatch("track", "list", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("architecture"));
    assert!(output.message.contains("design-doc"));

    let data = output.data.unwrap();
    assert_eq!(data["count"], 2);
}

#[test]
fn track_status_update() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry
        .dispatch("track", "add", &["readme".into()])
        .unwrap();

    let output = registry
        .dispatch("track", "status", &["readme".into(), "in-progress".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("in-progress"));

    let output = registry
        .dispatch("track", "show", &["readme".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("in-progress"));
}

#[test]
fn track_remove() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry
        .dispatch("track", "add", &["temp-doc".into()])
        .unwrap();

    let output = registry
        .dispatch("track", "remove", &["temp-doc".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("Removed"));

    // Show should now fail.
    let output = registry
        .dispatch("track", "show", &["temp-doc".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Error);
}

#[test]
fn track_duplicate_add_fails() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry
        .dispatch("track", "add", &["arch".into()])
        .unwrap();
    let output = registry
        .dispatch("track", "add", &["arch".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Error);
    assert!(output.message.contains("already exists"));
}

// ---------------------------------------------------------------------------
// Agile plugin tests
// ---------------------------------------------------------------------------

#[test]
fn agile_add_and_list() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let output = registry
        .dispatch("agile", "add", &["implement login".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    let data = output.data.unwrap();
    assert_eq!(data["status"], "open");
    assert_eq!(data["priority"], "medium");
    let id = data["id"].as_i64().unwrap();
    assert!(id > 0);

    let output = registry
        .dispatch(
            "agile",
            "add",
            &["fix bug".into(), "--priority".into(), "high".into(), "--tag".into(), "bug".into()],
        )
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("[bug]"));
    assert!(output.message.contains("high"));

    let output = registry.dispatch("agile", "list", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("implement login"));
    assert!(output.message.contains("fix bug"));
    let data = output.data.unwrap();
    assert_eq!(data["count"], 2);
}

#[test]
fn agile_update_status_and_priority() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let output = registry
        .dispatch("agile", "add", &["task one".into()])
        .unwrap();
    let id = output.data.unwrap()["id"].as_i64().unwrap();

    let output = registry
        .dispatch(
            "agile",
            "update",
            &[
                id.to_string(),
                "--status".into(),
                "in-progress".into(),
                "--priority".into(),
                "high".into(),
            ],
        )
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("in-progress"));
    assert!(output.message.contains("high"));

    let output = registry
        .dispatch("agile", "show", &[id.to_string()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("in-progress"));
    assert!(output.message.contains("high"));
}

#[test]
fn agile_done_marks_complete() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let output = registry
        .dispatch("agile", "add", &["quick task".into()])
        .unwrap();
    let id = output.data.unwrap()["id"].as_i64().unwrap();

    let output = registry
        .dispatch("agile", "done", &[id.to_string()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("done"));

    let output = registry
        .dispatch("agile", "show", &[id.to_string()])
        .unwrap();
    assert!(output.message.contains("done"));
}

#[test]
fn agile_sprint_shows_active_tasks() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    let o1 = registry
        .dispatch("agile", "add", &["task a".into(), "--priority".into(), "high".into()])
        .unwrap();
    let id1 = o1.data.unwrap()["id"].as_i64().unwrap();

    registry
        .dispatch("agile", "add", &["task b".into()])
        .unwrap();

    registry
        .dispatch("agile", "update", &[id1.to_string(), "--status".into(), "in-progress".into()])
        .unwrap();

    let output = registry.dispatch("agile", "sprint", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("In Progress:"));
    assert!(output.message.contains("task a"));
    assert!(output.message.contains("Open:"));
    assert!(output.message.contains("task b"));

    let data = output.data.unwrap();
    assert_eq!(data["in_progress"], 1);
    assert_eq!(data["open"], 1);
    assert_eq!(data["total"], 2);
}

#[test]
fn agile_list_with_filters() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    registry
        .dispatch("agile", "add", &["bug fix".into(), "--tag".into(), "bug".into()])
        .unwrap();
    registry
        .dispatch("agile", "add", &["new feature".into(), "--tag".into(), "feature".into()])
        .unwrap();

    let output = registry
        .dispatch("agile", "list", &["--tag".into(), "bug".into()])
        .unwrap();
    assert_eq!(output.status, CommandStatus::Ok);
    assert!(output.message.contains("bug fix"));
    assert!(!output.message.contains("new feature"));
    assert_eq!(output.data.unwrap()["count"], 1);
}

// ---------------------------------------------------------------------------
// Graceful DB error handling tests
// ---------------------------------------------------------------------------

#[test]
fn db_error_returns_plugin_error_not_trap() {
    let tmp = setup_bea_dir();
    let mut registry = PluginRegistry::load(&bea_dir(&tmp), false).unwrap();

    // track add with same name twice - second should return CommandStatus::Error
    // (not a wasm trap) because the host now returns SQL errors gracefully.
    registry
        .dispatch("track", "add", &["dup-test".into()])
        .unwrap();
    let output = registry
        .dispatch("track", "add", &["dup-test".into()])
        .unwrap();

    // Should be an error response, not a panic/trap.
    assert_eq!(output.status, CommandStatus::Error);
    assert!(output.message.contains("already exists"));
}

// ---------------------------------------------------------------------------
// Event log tests
// ---------------------------------------------------------------------------

#[test]
fn dispatch_records_events_in_log() {
    let tmp = setup_bea_dir();
    let dir = bea_dir(&tmp);
    let mut registry = PluginRegistry::load(&dir, false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();
    registry
        .dispatch("journal", "log", &["test message".into()])
        .unwrap();
    registry.dispatch("journal", "close", &[]).unwrap();

    // Open the event log independently and verify entries.
    let mut log = EventLog::open(&dir).unwrap();
    let events = log.recent(10, None).unwrap();
    assert_eq!(events.len(), 3);

    // Most recent first.
    assert_eq!(events[0].command, "close");
    assert_eq!(events[1].command, "log");
    assert_eq!(events[2].command, "open");

    // All should be from journal plugin.
    assert!(events.iter().all(|e| e.plugin == "journal"));
    // All should have ok status.
    assert!(events.iter().all(|e| e.status == "ok"));
}

#[test]
fn event_log_captures_error_status() {
    let tmp = setup_bea_dir();
    let dir = bea_dir(&tmp);
    let mut registry = PluginRegistry::load(&dir, false).unwrap();

    // journal log without open session - plugin returns error status.
    let output = registry.dispatch("journal", "log", &[]).unwrap();
    assert_eq!(output.status, CommandStatus::Error);

    let mut log = EventLog::open(&dir).unwrap();
    let events = log.recent(10, None).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, "error");
    assert_eq!(events[0].command, "log");
}

#[test]
fn event_log_records_across_plugins() {
    let tmp = setup_bea_dir();
    let dir = bea_dir(&tmp);
    let mut registry = PluginRegistry::load(&dir, false).unwrap();

    registry.dispatch("journal", "open", &[]).unwrap();
    registry
        .dispatch("track", "add", &["readme".into()])
        .unwrap();
    registry
        .dispatch("agile", "add", &["task one".into()])
        .unwrap();

    let mut log = EventLog::open(&dir).unwrap();

    // All events.
    let all = log.recent(10, None).unwrap();
    assert_eq!(all.len(), 3);

    // Filter by plugin.
    let journal_events = log.recent(10, Some("journal")).unwrap();
    assert_eq!(journal_events.len(), 1);
    assert_eq!(journal_events[0].command, "open");

    let track_events = log.recent(10, Some("track")).unwrap();
    assert_eq!(track_events.len(), 1);
    assert_eq!(track_events[0].command, "add");
}
