use std::path::{Path, PathBuf};

use crate::config::{self, BeuConfig};
use crate::sqlite::SqliteStore;
use crate::store::{ArtifactStore, DebugStore, EventLogStore, IdeaStore, StateStore, TaskStore};

use super::state;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Format a byte count as a human-readable string (e.g., "1.2KB", "3.4MB").
pub fn format_byte_size(size: u64) -> String {
    if size >= 1024 * 1024 {
        format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        format!("{:.1}KB", size as f64 / 1024.0)
    } else {
        format!("{size}B")
    }
}

// ---------------------------------------------------------------------------
// Init / Version
// ---------------------------------------------------------------------------

pub fn cmd_init(
    root: &Path,
    quiet: bool,
    all_agents: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let beu_dir = root.join(".beu");
    if beu_dir.exists() {
        return Err(format!(".beu directory already exists at {}", beu_dir.display()).into());
    }

    // Opening the store creates the data directory and initializes all tables.
    let mut store = SqliteStore::open(&beu_dir, "default")?;

    // Register the default project.
    store.register_project()?;

    // Write default config.yml with all modules enabled.
    config::save(&beu_dir, &BeuConfig::default())?;

    // Write .gitignore to exclude the SQLite database from version control.
    std::fs::write(beu_dir.join(".gitignore"), "data/*.db\n")?;

    // Download skill rule files via npx. Non-fatal: init succeeds even if the
    // package isn't available (e.g. offline or not yet published). The user can
    // run `beu update-rules` later once the package is reachable.
    let skills_result = crate::rules::install_skills(root, all_agents);

    if !quiet {
        println!("Initialized .beu at {}", beu_dir.display());
        println!("  data/beu.db  - SQLite database (all modules)");
        println!("  config.yml   - module configuration");
        println!("  .gitignore   - excludes data/*.db");
        println!("  project:     default");
        println!();
        println!("Modules: journal, artifact, task, state, idea, debug");
        match &skills_result {
            Ok(written) if !written.is_empty() => {
                println!();
                println!("Agent rules:");
                for path in written {
                    println!("  {path}");
                }
            }
            Ok(_) => {} // nothing written (all already existed)
            Err(e) => {
                println!();
                println!("warning: could not install agent skill rules: {e}");
                println!("  run 'beu update-rules' later to install them.");
            }
        }
    }
    Ok(())
}

pub fn cmd_update_rules(root: &Path, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    let updated = crate::rules::install_skills(root, true)?;

    if !quiet {
        if updated.is_empty() {
            println!("Agent rules are already up to date.");
        } else {
            println!("Updated agent rules ({}):", updated.len());
            for path in &updated {
                println!("  {path}");
            }
        }
    }
    Ok(())
}

pub fn cmd_version(beu_dir_flag: Option<PathBuf>) {
    println!("beu {}", env!("CARGO_PKG_VERSION"));

    match super::super::resolve_beu_dir(beu_dir_flag) {
        Ok(beu_dir) => {
            println!("  beu_dir: {}", beu_dir.display());
        }
        Err(_) => {
            println!("  beu_dir: (not found)");
        }
    }
}

// ---------------------------------------------------------------------------
// Status / Events / Logs
// ---------------------------------------------------------------------------

pub fn cmd_status(
    store: &mut SqliteStore,
    config: &BeuConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let beu_dir = store.beu_dir();
    println!(
        "beu project: {}",
        beu_dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("?")
    );
    println!("  modules: {}", config.enabled_modules().join(", "));

    if let Some(size) = store.db_size() {
        println!("  data:     {}", format_byte_size(size));
    } else {
        println!("  data:     (empty)");
    }

    let recent = store.recent_events(1, None)?;
    if let Some(last) = recent.first() {
        println!(
            "  last activity: {} {} {} ({})",
            last.timestamp, last.module, last.command, last.status
        );
    } else {
        println!("  last activity: (none)");
    }
    let count = store.count_events()?;
    println!("  total events: {count}");

    Ok(())
}

pub fn cmd_events(
    store: &mut impl EventLogStore,
    limit: usize,
    module_filter: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let events = store.recent_events(limit, module_filter)?;

    if events.is_empty() {
        println!("No events recorded yet.");
        return Ok(());
    }

    println!(
        "{:<5} {:<22} {:<12} {:<12} {:<20} {:<8} {:>6}",
        "ID", "TIMESTAMP", "MODULE", "COMMAND", "ARGS", "STATUS", "MS"
    );
    println!("{}", "-".repeat(87));

    for event in &events {
        let args_display = if event.args.len() > 18 {
            format!("{}...", &event.args[..18])
        } else {
            event.args.clone()
        };
        println!(
            "{:<5} {:<22} {:<12} {:<12} {:<20} {:<8} {:>6}",
            event.id,
            event.timestamp,
            event.module,
            event.command,
            args_display,
            event.status,
            event.duration_ms
        );
    }

    println!("\n{} event(s) shown", events.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// Export / Import / Reset
// ---------------------------------------------------------------------------

pub fn cmd_export(
    store: &mut SqliteStore,
    module: Option<&str>,
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match (module, all) {
        (Some(name), false) => {
            let data = store.export_module(name)?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(())
        }
        (None, true) => {
            let modules = SqliteStore::list_modules();
            let mut result = serde_json::Map::new();
            for name in modules {
                match store.export_module(name) {
                    Ok(data) => {
                        result.insert(name.to_string(), data);
                    }
                    Err(e) => {
                        eprintln!("warning: failed to export {name}: {e}");
                    }
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::Value::Object(result))?
            );
            Ok(())
        }
        (None, false) => Err("specify a module name or use --all".into()),
        (Some(_), true) => Err("cannot use --all with a specific module name".into()),
    }
}

pub fn cmd_import(
    store: &mut SqliteStore,
    module: &str,
    file: &Path,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("file not found: {}", file.display()).into());
    }

    let content = std::fs::read_to_string(file)?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in {}: {e}", file.display()))?;

    let (tables, rows) = store.import_module(module, &data)?;

    if !quiet {
        println!("Imported {rows} rows across {tables} tables into '{module}'.");
    }
    Ok(())
}

pub fn cmd_reset(
    store: &mut SqliteStore,
    module: &str,
    force: bool,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !force {
        return Err(
            format!("this will delete all data for '{module}'. Use --force to confirm.").into(),
        );
    }

    let tables = store.reset_module(module)?;

    if !quiet {
        println!("Reset '{module}': dropped {tables} table(s).");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

pub fn cmd_health(
    store: &mut SqliteStore,
    _repair: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut issues = 0;

    // Check data directory.
    let data_dir = store.beu_dir().join("data");
    if data_dir.is_dir() {
        println!("[ok] data/ directory exists");
    } else {
        println!("[error] data/ directory missing");
        issues += 1;
    }

    // Check database file.
    let db_path = data_dir.join("beu.db");
    if db_path.exists() {
        println!("[ok] beu.db exists");
    } else {
        println!("[error] beu.db missing");
        issues += 1;
    }

    // Integrity check.
    match store.validate() {
        Ok(()) => println!("[ok] integrity check passed"),
        Err(e) => {
            println!("[error] integrity check: {e}");
            issues += 1;
        }
    }

    if issues == 0 {
        println!("\nAll checks passed.");
    } else {
        println!("\n{issues} issue(s) found.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pause / Resume / Progress
// ---------------------------------------------------------------------------

pub fn cmd_pause(
    store: &mut impl StateStore,
    message: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let checkpoint = message.unwrap_or("(paused with no message)");
    state::cmd_set(store, "focus", "_checkpoint", checkpoint)?;
    println!("Checkpoint saved. Run 'beu resume' to pick up where you left off.");
    Ok(())
}

pub fn cmd_resume(store: &mut impl StateStore) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(msg) = store.get_checkpoint()? {
        println!("Checkpoint: {msg}");
        store.clear_checkpoint()?;
    } else {
        println!("No checkpoint found.");
    }

    let blockers = store.list_blockers()?;
    if !blockers.is_empty() {
        println!("\nBlockers:");
        for (key, value) in &blockers {
            println!("  [blocker] {key}: {value}");
        }
    }

    let focus = store.list_focus_items()?;
    if !focus.is_empty() {
        println!("\nFocus:");
        for (key, value) in &focus {
            println!("  [focus] {key}: {value}");
        }
    }

    Ok(())
}

pub fn cmd_progress(
    store: &mut (impl StateStore + TaskStore + ArtifactStore + IdeaStore + DebugStore),
    config: &BeuConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Progress Summary ===\n");

    if config.is_module_enabled("state") {
        if let Some(msg) = store.get_checkpoint()? {
            println!("Checkpoint: {msg}");
        }

        let blocker_count = store.count_by_category("blocker")?;
        if blocker_count > 0 {
            println!("Blockers: {blocker_count}");
        }
    }

    if config.is_module_enabled("task") {
        let task_counts = store.count_tasks_by_status()?;
        if !task_counts.is_empty() {
            println!("\nTasks:");
            for (status, count) in &task_counts {
                println!("  {status}: {count}");
            }
        }
    }

    if config.is_module_enabled("artifact") {
        let artifacts = crate::store::ArtifactStore::list_artifacts(store, None)?;
        if !artifacts.is_empty() {
            let mut counts: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for a in &artifacts {
                *counts.entry(&a.status).or_insert(0) += 1;
            }
            println!("\nArtifacts:");
            for (status, count) in &counts {
                println!("  {status}: {count}");
            }
        }
    }

    if config.is_module_enabled("idea") {
        let idea_counts = store.count_ideas_by_status()?;
        if !idea_counts.is_empty() {
            println!("\nIdeas:");
            for (status, count) in &idea_counts {
                println!("  {status}: {count}");
            }
        }
    }

    if config.is_module_enabled("debug") {
        let active_debug = store.count_active()?;
        if active_debug > 0 {
            println!("\nActive debug sessions: {active_debug}");
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Check (compliance gate)
// ---------------------------------------------------------------------------

/// Verify all required docs from config are registered, not pending,
/// and not stale (if staleness_threshold is configured).
pub fn cmd_check(
    store: &mut (impl ArtifactStore + EventLogStore),
    config: &BeuConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config.is_module_enabled("artifact") {
        return Err("'beu check' requires the artifact module to be enabled".into());
    }

    let required = &config.required_docs;
    if required.is_empty() {
        println!("No required docs configured.");
        return Ok(());
    }

    let total = required.len();
    let mut errors = 0usize;
    let threshold = config.staleness_threshold;

    for doc in required {
        match store.get_artifact(&doc.name)? {
            None => {
                eprintln!("ERROR: required doc '{}' not registered", doc.name);
                eprintln!("  -> beu artifact add {} --type {}", doc.name, doc.doc_type);
                eprintln!();
                errors += 1;
            }
            Some(artifact) if artifact.status == "pending" => {
                eprintln!("ERROR: required doc '{}' is still 'pending'", doc.name);
                eprintln!("  -> beu artifact status {} in-progress", doc.name);
                eprintln!();
                errors += 1;
            }
            Some(artifact) => {
                if let Some(t) = threshold {
                    let mutations = store.count_mutation_events_since(&artifact.updated_at)?;
                    if mutations >= t as i64 {
                        eprintln!(
                            "ERROR: required doc '{}' is stale ({} changes since last update)",
                            doc.name, mutations
                        );
                        eprintln!(
                            "  -> update doc, then: beu artifact changelog {} \"<summary>\"",
                            doc.name
                        );
                        eprintln!();
                        errors += 1;
                    }
                }
            }
        }
    }

    let satisfied = total - errors;
    if errors > 0 {
        println!("{satisfied}/{total} required docs satisfied.");
        Err(format!("{errors} required doc(s) failed compliance check").into())
    } else {
        println!("All {total} required docs satisfied.");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Event logging helper
// ---------------------------------------------------------------------------

pub fn log_event(
    store: &mut impl EventLogStore,
    module: &str,
    command: &str,
    status: &str,
    duration_ms: i64,
) {
    let _ = store.log_event(module, command, "", status, duration_ms);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BeuConfig, ModuleConfig, RequiredDoc};
    use crate::store::{Artifact, ArtifactChangelog, ArtifactStore, Event, EventLogStore};
    use std::collections::HashMap;

    struct FakeStore {
        artifacts: HashMap<String, Artifact>,
        mutation_count: i64,
    }

    impl FakeStore {
        fn new() -> Self {
            Self {
                artifacts: HashMap::new(),
                mutation_count: 0,
            }
        }

        fn with_artifact(mut self, name: &str, status: &str) -> Self {
            self.artifacts.insert(
                name.to_string(),
                Artifact {
                    name: name.to_string(),
                    artifact_type: "doc".to_string(),
                    description: None,
                    status: status.to_string(),
                    created_at: "2026-01-01T00:00:00.000Z".to_string(),
                    updated_at: "2026-01-01T00:00:00.000Z".to_string(),
                },
            );
            self
        }

        fn with_mutation_count(mut self, count: i64) -> Self {
            self.mutation_count = count;
            self
        }
    }

    impl ArtifactStore for FakeStore {
        fn add_artifact(
            &mut self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn get_artifact(
            &mut self,
            name: &str,
        ) -> Result<Option<Artifact>, Box<dyn std::error::Error>> {
            Ok(self.artifacts.get(name).cloned())
        }
        fn update_artifact_status(
            &mut self,
            _: &str,
            _: &str,
        ) -> Result<String, Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn describe_artifact(
            &mut self,
            _: &str,
            _: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn list_artifacts(
            &mut self,
            _: Option<&str>,
        ) -> Result<Vec<Artifact>, Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn remove_artifact(&mut self, _: &str) -> Result<bool, Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn add_changelog_entry(
            &mut self,
            _: &str,
            _: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn list_changelog(
            &mut self,
            _: &str,
        ) -> Result<Vec<ArtifactChangelog>, Box<dyn std::error::Error>> {
            unimplemented!()
        }
    }

    impl EventLogStore for FakeStore {
        fn log_event(
            &mut self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: i64,
        ) -> Result<(), Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn recent_events(
            &mut self,
            _: usize,
            _: Option<&str>,
        ) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn count_events(&mut self) -> Result<i64, Box<dyn std::error::Error>> {
            unimplemented!()
        }
        fn count_mutation_events_since(
            &mut self,
            _since: &str,
        ) -> Result<i64, Box<dyn std::error::Error>> {
            Ok(self.mutation_count)
        }
    }

    fn config_with_docs(docs: Vec<RequiredDoc>) -> BeuConfig {
        BeuConfig {
            modules: ModuleConfig::default(),
            required_docs: docs,
            ..BeuConfig::default()
        }
    }

    fn rdoc(name: &str, doc_type: &str) -> RequiredDoc {
        RequiredDoc {
            name: name.to_string(),
            doc_type: doc_type.to_string(),
        }
    }

    #[test]
    fn check_no_required_docs_succeeds() {
        let mut store = FakeStore::new();
        let config = config_with_docs(vec![]);
        assert!(cmd_check(&mut store, &config).is_ok());
    }

    #[test]
    fn check_all_satisfied() {
        let mut store = FakeStore::new()
            .with_artifact("design", "in-progress")
            .with_artifact("changelog", "done");
        let config = config_with_docs(vec![rdoc("design", "doc"), rdoc("changelog", "changelog")]);
        assert!(cmd_check(&mut store, &config).is_ok());
    }

    #[test]
    fn check_missing_artifact_fails() {
        let mut store = FakeStore::new();
        let config = config_with_docs(vec![rdoc("design", "doc")]);
        assert!(cmd_check(&mut store, &config).is_err());
    }

    #[test]
    fn check_pending_artifact_fails() {
        let mut store = FakeStore::new().with_artifact("design", "pending");
        let config = config_with_docs(vec![rdoc("design", "doc")]);
        assert!(cmd_check(&mut store, &config).is_err());
    }

    #[test]
    fn check_review_status_passes() {
        let mut store = FakeStore::new().with_artifact("design", "review");
        let config = config_with_docs(vec![rdoc("design", "doc")]);
        assert!(cmd_check(&mut store, &config).is_ok());
    }

    #[test]
    fn check_artifact_module_disabled_fails() {
        let mut store = FakeStore::new();
        let mut config = config_with_docs(vec![rdoc("design", "doc")]);
        config.modules.artifact = false;
        let err = cmd_check(&mut store, &config).unwrap_err();
        assert!(err.to_string().contains("artifact module"));
    }

    #[test]
    fn check_mixed_pass_and_fail() {
        let mut store = FakeStore::new()
            .with_artifact("design", "in-progress")
            .with_artifact("changelog", "pending");
        let config = config_with_docs(vec![
            rdoc("design", "doc"),
            rdoc("changelog", "changelog"),
            rdoc("usage-guide", "doc"),
        ]);
        let err = cmd_check(&mut store, &config).unwrap_err();
        assert!(err.to_string().contains("2 required doc(s)"));
    }

    #[test]
    fn check_stale_doc_fails() {
        let mut store = FakeStore::new()
            .with_artifact("design", "done")
            .with_mutation_count(15);
        let mut config = config_with_docs(vec![rdoc("design", "doc")]);
        config.staleness_threshold = Some(10);
        let err = cmd_check(&mut store, &config).unwrap_err();
        assert!(err.to_string().contains("1 required doc(s)"));
    }

    #[test]
    fn check_fresh_doc_passes() {
        let mut store = FakeStore::new()
            .with_artifact("design", "done")
            .with_mutation_count(3);
        let mut config = config_with_docs(vec![rdoc("design", "doc")]);
        config.staleness_threshold = Some(10);
        assert!(cmd_check(&mut store, &config).is_ok());
    }

    #[test]
    fn check_staleness_disabled_by_default() {
        let mut store = FakeStore::new()
            .with_artifact("design", "done")
            .with_mutation_count(100);
        let config = config_with_docs(vec![rdoc("design", "doc")]);
        // staleness_threshold is None by default, so even 100 mutations pass
        assert!(cmd_check(&mut store, &config).is_ok());
    }

    #[test]
    fn check_staleness_only_applies_to_existing_docs() {
        let mut store = FakeStore::new().with_mutation_count(50);
        let mut config = config_with_docs(vec![rdoc("design", "doc")]);
        config.staleness_threshold = Some(5);
        // Missing doc should report "missing", not "stale"
        let err = cmd_check(&mut store, &config).unwrap_err();
        assert!(err.to_string().contains("1 required doc(s)"));
    }
}
