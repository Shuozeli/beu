mod common;

use common::{beu_cmd, beu_dir_path, set_required_docs, setup};

/// Helper: assert a command succeeds, return stdout.
fn run_ok(cmd: &mut std::process::Command) -> String {
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "command failed.\nstdout: {stdout}\nstderr: {stderr}"
    );
    stdout
}

/// Helper: assert a command fails, return stderr.
fn run_fail(cmd: &mut std::process::Command) -> String {
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        !output.status.success(),
        "expected failure but succeeded.\nstdout: {}\nstderr: {stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    stderr
}

/// Helper: set staleness_threshold in config.
fn set_staleness_threshold(tmp: &tempfile::TempDir, threshold: u64) {
    let config_path = tmp.path().join(".beu/config.yml");
    let content = std::fs::read_to_string(&config_path).unwrap();
    let updated = format!("{content}staleness_threshold: {threshold}\n");
    std::fs::write(&config_path, updated).unwrap();
}

// ---------------------------------------------------------------------------
// Full session lifecycle: init -> resume -> work -> check -> pause -> resume
// ---------------------------------------------------------------------------

#[test]
fn workflow_full_session_lifecycle() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // 1. Resume on fresh project -- no checkpoint, no blockers
    let stdout = run_ok(beu_cmd(&dir).args(["resume"]));
    assert!(stdout.contains("No checkpoint"));

    // 2. Open journal session
    run_ok(beu_cmd(&dir).args(["journal", "open"]));

    // 3. Plan work with tasks
    run_ok(beu_cmd(&dir).args(["task", "add", "implement auth", "--priority", "high"]));
    run_ok(beu_cmd(&dir).args(["task", "add", "write tests", "--priority", "medium"]));

    // 4. Record a decision in state
    run_ok(beu_cmd(&dir).args([
        "state",
        "set",
        "--category",
        "decision",
        "auth-method",
        "JWT",
    ]));

    // 5. Set up compliance: required docs + staleness
    set_required_docs(&tmp, &[("design", "doc")]);
    set_staleness_threshold(&tmp, 10);

    // Register and activate the required doc
    run_ok(beu_cmd(&dir).args(["artifact", "add", "design", "--type", "doc"]));
    run_ok(beu_cmd(&dir).args(["artifact", "status", "design", "in-progress"]));

    // 6. Check passes (few mutations, doc is active)
    run_ok(beu_cmd(&dir).args(["check"]));

    // 7. Do work: complete a task
    run_ok(beu_cmd(&dir).args(["task", "done", "1"]));

    // 8. Journal notes
    run_ok(beu_cmd(&dir).args(["journal", "note", "--tag", "decision", "chose JWT for auth"]));

    // 9. Progress shows cross-module summary
    let stdout = run_ok(beu_cmd(&dir).args(["progress"]));
    assert!(stdout.contains("Progress Summary"));
    assert!(stdout.contains("Tasks:"));

    // 10. Pause with checkpoint
    run_ok(beu_cmd(&dir).args(["pause", "auth implemented, tests pending"]));

    // 11. Close journal
    run_ok(beu_cmd(&dir).args(["journal", "close"]));

    // 12. Resume picks up checkpoint, state, focus
    let stdout = run_ok(beu_cmd(&dir).args(["resume"]));
    assert!(
        stdout.contains("auth implemented, tests pending"),
        "resume should show checkpoint message, got: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Staleness detection cross-module workflow
// ---------------------------------------------------------------------------

#[test]
fn workflow_staleness_cross_module() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Set up compliance with low threshold
    set_required_docs(&tmp, &[("changelog", "changelog")]);
    set_staleness_threshold(&tmp, 3);

    // Register artifact
    run_ok(beu_cmd(&dir).args(["artifact", "add", "changelog", "--type", "changelog"]));
    run_ok(beu_cmd(&dir).args(["artifact", "status", "changelog", "done"]));

    // Check passes initially
    run_ok(beu_cmd(&dir).args(["check"]));

    // Generate mutations across different modules
    run_ok(beu_cmd(&dir).args(["task", "add", "task one"])); // mutation 1
    run_ok(beu_cmd(&dir).args(["state", "set", "--category", "decision", "db", "sqlite"])); // mutation 2
    run_ok(beu_cmd(&dir).args(["idea", "add", "new feature"])); // mutation 3

    // Check should now fail: 3 mutations >= threshold 3
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("stale"), "expected stale error: {stderr}");
    assert!(
        stderr.contains("changelog"),
        "should mention which doc is stale: {stderr}"
    );

    // Fix: update the doc via changelog
    run_ok(beu_cmd(&dir).args([
        "artifact",
        "changelog",
        "changelog",
        "updated for task and state changes",
    ]));

    // Check passes again
    run_ok(beu_cmd(&dir).args(["check"]));

    // More mutations push it stale again
    run_ok(beu_cmd(&dir).args(["task", "add", "task two"]));
    run_ok(beu_cmd(&dir).args(["task", "add", "task three"]));
    run_ok(beu_cmd(&dir).args(["task", "done", "1"]));

    // Stale again
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("stale"));
}

// ---------------------------------------------------------------------------
// Multiple required docs: mixed staleness and status
// ---------------------------------------------------------------------------

#[test]
fn workflow_multiple_docs_mixed_compliance() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    set_required_docs(&tmp, &[("design", "doc"), ("changelog", "changelog")]);
    set_staleness_threshold(&tmp, 5);

    // Only register design, not changelog
    run_ok(beu_cmd(&dir).args(["artifact", "add", "design", "--type", "doc"]));
    run_ok(beu_cmd(&dir).args(["artifact", "status", "design", "done"]));

    // Check fails: changelog missing
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("changelog"));
    assert!(stderr.contains("not registered"));

    // Register changelog but leave pending
    run_ok(beu_cmd(&dir).args(["artifact", "add", "changelog", "--type", "changelog"]));

    // Check fails: changelog is pending
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("pending"));

    // Activate changelog
    run_ok(beu_cmd(&dir).args(["artifact", "status", "changelog", "done"]));

    // Check passes
    run_ok(beu_cmd(&dir).args(["check"]));

    // Generate mutations to make both stale
    for i in 1..=6 {
        run_ok(beu_cmd(&dir).args(["task", "add", &format!("task {i}")]));
    }

    // Check fails: both stale
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("stale"));

    // Fix only design
    run_ok(beu_cmd(&dir).args(["artifact", "changelog", "design", "updated design doc"]));

    // Check still fails: changelog still stale
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("changelog"));

    // Fix changelog too
    run_ok(beu_cmd(&dir).args(["artifact", "changelog", "changelog", "updated changelog"]));

    // Now passes
    run_ok(beu_cmd(&dir).args(["check"]));
}

// ---------------------------------------------------------------------------
// Debug investigation workflow integrated with session
// ---------------------------------------------------------------------------

#[test]
fn workflow_debug_investigation_with_journal() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Open journal for the session
    run_ok(beu_cmd(&dir).args(["journal", "open"]));

    // User reports a bug, create a task
    run_ok(beu_cmd(&dir).args(["task", "add", "fix login timeout", "--priority", "high"]));

    // Open debug investigation
    run_ok(beu_cmd(&dir).args(["debug", "open", "login", "timeout", "after", "deploy"]));

    // Log symptoms and evidence
    run_ok(beu_cmd(&dir).args([
        "debug",
        "symptom",
        "login-timeout-after-deploy",
        "401 after 30s",
    ]));
    run_ok(beu_cmd(&dir).args([
        "debug",
        "log",
        "login-timeout-after-deploy",
        "redis connection pool exhausted",
    ]));

    // Record a finding in journal
    run_ok(beu_cmd(&dir).args([
        "journal",
        "note",
        "--tag",
        "finding",
        "redis pool maxed at 10 connections",
    ]));

    // Found the root cause
    run_ok(beu_cmd(&dir).args([
        "debug",
        "cause",
        "login-timeout-after-deploy",
        "redis pool too small for new traffic",
    ]));

    // Verify debug timeline shows full investigation
    let stdout = run_ok(beu_cmd(&dir).args(["debug", "show", "login-timeout-after-deploy"]));
    assert!(stdout.contains("Timeline:"));
    assert!(stdout.contains("[symptom]"));
    assert!(stdout.contains("[evidence]"));
    assert!(stdout.contains("[cause]"));

    // Resolve the investigation
    run_ok(beu_cmd(&dir).args(["debug", "resolve", "login-timeout-after-deploy"]));

    // Complete the task
    run_ok(beu_cmd(&dir).args(["task", "done", "1"]));

    // Sprint shows clear (all tasks completed)
    let stdout = run_ok(beu_cmd(&dir).args(["task", "sprint"]));
    assert!(
        stdout.contains("clear") || stdout.contains("no open"),
        "sprint should be clear after completing all tasks: {stdout}"
    );

    // Progress reflects all the activity
    let stdout = run_ok(beu_cmd(&dir).args(["progress"]));
    assert!(stdout.contains("Tasks:"));
}

// ---------------------------------------------------------------------------
// Project-scoped workflow: two projects, independent data
// ---------------------------------------------------------------------------

#[test]
fn workflow_project_scoped_independent_sessions() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // --- Project "api" ---
    run_ok(beu_cmd(&dir).args([
        "--project",
        "api",
        "task",
        "add",
        "implement endpoint",
        "--priority",
        "high",
    ]));
    run_ok(beu_cmd(&dir).args([
        "--project",
        "api",
        "state",
        "set",
        "--category",
        "decision",
        "framework",
        "axum",
    ]));
    run_ok(beu_cmd(&dir).args([
        "--project",
        "api",
        "artifact",
        "add",
        "api-spec",
        "--type",
        "doc",
    ]));

    // --- Project "web" ---
    run_ok(beu_cmd(&dir).args([
        "--project",
        "web",
        "task",
        "add",
        "build login page",
        "--priority",
        "medium",
    ]));
    run_ok(beu_cmd(&dir).args([
        "--project",
        "web",
        "state",
        "set",
        "--category",
        "decision",
        "framework",
        "react",
    ]));

    // Verify isolation: api tasks not visible in web
    let stdout = run_ok(beu_cmd(&dir).args(["--project", "web", "task", "list"]));
    assert!(
        !stdout.contains("implement endpoint"),
        "web should not see api tasks"
    );
    assert!(stdout.contains("build login page"));

    // Verify isolation: web state not visible in api
    let stdout = run_ok(beu_cmd(&dir).args(["--project", "api", "state", "list"]));
    assert!(stdout.contains("axum"));
    assert!(!stdout.contains("react"), "api should not see web state");

    // Pause with different messages per project
    run_ok(beu_cmd(&dir).args(["--project", "api", "pause", "endpoint scaffolded"]));
    run_ok(beu_cmd(&dir).args(["--project", "web", "pause", "login page wireframed"]));

    // Resume each project independently
    let stdout = run_ok(beu_cmd(&dir).args(["--project", "api", "resume"]));
    assert!(
        stdout.contains("endpoint scaffolded"),
        "api resume should show api checkpoint: {stdout}"
    );

    let stdout = run_ok(beu_cmd(&dir).args(["--project", "web", "resume"]));
    assert!(
        stdout.contains("login page wireframed"),
        "web resume should show web checkpoint: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// Idea capture and triage workflow
// ---------------------------------------------------------------------------

#[test]
fn workflow_idea_capture_and_triage() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Capture ideas during a session
    run_ok(beu_cmd(&dir).args(["idea", "add", "add dark mode"]));
    run_ok(beu_cmd(&dir).args(["idea", "add", "refactor auth module"]));
    run_ok(beu_cmd(&dir).args(["idea", "add", "improve error messages"]));
    run_ok(beu_cmd(&dir).args(["idea", "add", "add export feature"]));

    // List all ideas
    let stdout = run_ok(beu_cmd(&dir).args(["idea", "list"]));
    assert!(stdout.contains("add dark mode"));
    assert!(stdout.contains("refactor auth module"));
    assert!(stdout.contains("improve error messages"));
    assert!(stdout.contains("add export feature"));

    // Triage: promote one idea to a task
    run_ok(beu_cmd(&dir).args(["task", "add", "refactor auth module", "--priority", "high"]));
    run_ok(beu_cmd(&dir).args(["idea", "done", "2"]));

    // Archive a low-priority idea
    run_ok(beu_cmd(&dir).args(["idea", "archive", "4"]));

    // List active ideas
    let stdout = run_ok(beu_cmd(&dir).args(["idea", "list"]));
    assert!(stdout.contains("add dark mode"));
    assert!(stdout.contains("improve error messages"));
}

// ---------------------------------------------------------------------------
// Pause -> resume round-trip with blockers and focus
// ---------------------------------------------------------------------------

#[test]
fn workflow_pause_resume_with_context() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Set up session context
    run_ok(beu_cmd(&dir).args(["journal", "open"]));
    run_ok(beu_cmd(&dir).args(["task", "add", "fix memory leak", "--priority", "high"]));
    run_ok(beu_cmd(&dir).args([
        "state",
        "set",
        "--category",
        "blocker",
        "ci-flaky",
        "integration tests timeout",
    ]));
    run_ok(beu_cmd(&dir).args([
        "state",
        "set",
        "--category",
        "focus",
        "current",
        "profiling memory usage",
    ]));
    run_ok(beu_cmd(&dir).args(["debug", "open", "memory", "leak", "in", "worker"]));
    run_ok(beu_cmd(&dir).args([
        "debug",
        "log",
        "memory-leak-in-worker",
        "heap grows 10MB per hour",
    ]));

    // Pause: save checkpoint
    run_ok(beu_cmd(&dir).args(["pause", "profiling memory, suspect worker thread leak"]));

    // Close journal (simulating end of session)
    run_ok(beu_cmd(&dir).args(["journal", "close"]));

    // Resume: should recover all context
    let stdout = run_ok(beu_cmd(&dir).args(["resume"]));
    assert!(
        stdout.contains("profiling memory, suspect worker thread leak"),
        "should show checkpoint: {stdout}"
    );
    assert!(
        stdout.contains("ci-flaky"),
        "should show blockers: {stdout}"
    );
    assert!(
        stdout.contains("current: profiling memory usage"),
        "should show focus: {stdout}"
    );

    // Progress should also reflect the state (checkpoint consumed by resume above)
    let stdout = run_ok(beu_cmd(&dir).args(["progress"]));
    assert!(stdout.contains("Tasks:"));
}

// ---------------------------------------------------------------------------
// Event log captures all module activity
// ---------------------------------------------------------------------------

#[test]
fn workflow_event_log_captures_cross_module_activity() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Activity across multiple modules
    run_ok(beu_cmd(&dir).args(["journal", "open"]));
    run_ok(beu_cmd(&dir).args(["task", "add", "test task"]));
    run_ok(beu_cmd(&dir).args(["artifact", "add", "test-doc"]));
    run_ok(beu_cmd(&dir).args(["state", "set", "--category", "note", "k", "v"]));
    run_ok(beu_cmd(&dir).args(["idea", "add", "test idea"]));
    run_ok(beu_cmd(&dir).args(["debug", "open", "test", "bug"]));
    run_ok(beu_cmd(&dir).args(["journal", "log", "session entry"]));

    // Events should capture all modules
    let stdout = run_ok(beu_cmd(&dir).args(["events"]));
    assert!(
        stdout.contains("journal"),
        "events should have journal: {stdout}"
    );
    assert!(stdout.contains("task"), "events should have task: {stdout}");
    assert!(
        stdout.contains("artifact"),
        "events should have artifact: {stdout}"
    );
    assert!(
        stdout.contains("state"),
        "events should have state: {stdout}"
    );
    assert!(stdout.contains("idea"), "events should have idea: {stdout}");
    assert!(
        stdout.contains("debug"),
        "events should have debug: {stdout}"
    );

    // Filter by specific module
    let stdout = run_ok(beu_cmd(&dir).args(["events", "--module", "task"]));
    assert!(stdout.contains("task"));
}

// ---------------------------------------------------------------------------
// Compliance pipeline: continuous check during development
// ---------------------------------------------------------------------------

#[test]
fn workflow_continuous_compliance_pipeline() {
    let tmp = setup();
    let dir = beu_dir_path(&tmp);

    // Configure compliance
    set_required_docs(&tmp, &[("design", "doc"), ("changelog", "changelog")]);
    set_staleness_threshold(&tmp, 4);

    // Phase 1: Bootstrap -- register docs
    run_ok(beu_cmd(&dir).args(["artifact", "add", "design", "--type", "doc"]));
    run_ok(beu_cmd(&dir).args(["artifact", "status", "design", "in-progress"]));
    run_ok(beu_cmd(&dir).args(["artifact", "add", "changelog", "--type", "changelog"]));
    run_ok(beu_cmd(&dir).args(["artifact", "status", "changelog", "done"]));

    // Check after bootstrap
    run_ok(beu_cmd(&dir).args(["check"]));

    // Phase 2: Development sprint -- add tasks and work
    run_ok(beu_cmd(&dir).args(["task", "add", "implement auth"]));
    run_ok(beu_cmd(&dir).args(["task", "add", "implement api"]));
    run_ok(beu_cmd(&dir).args(["task", "done", "1"]));

    // Mid-sprint check: 3 mutations, still under threshold 4
    run_ok(beu_cmd(&dir).args(["check"]));

    // More work pushes over threshold
    run_ok(beu_cmd(&dir).args(["task", "done", "2"]));

    // Check fails: 4 mutations >= threshold 4
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("stale"));

    // Phase 3: Doc maintenance -- update both docs
    run_ok(beu_cmd(&dir).args([
        "artifact",
        "changelog",
        "design",
        "updated for auth and api implementation",
    ]));
    run_ok(beu_cmd(&dir).args([
        "artifact",
        "changelog",
        "changelog",
        "logged auth and api completion",
    ]));

    // Check passes after doc updates
    run_ok(beu_cmd(&dir).args(["check"]));

    // Phase 4: More development
    run_ok(beu_cmd(&dir).args(["task", "add", "add tests"]));
    run_ok(beu_cmd(&dir).args(["task", "add", "fix edge case"]));
    run_ok(beu_cmd(&dir).args(["task", "done", "3"]));
    run_ok(beu_cmd(&dir).args(["task", "done", "4"]));

    // Stale again
    let stderr = run_fail(beu_cmd(&dir).args(["check"]));
    assert!(stderr.contains("stale"));

    // Fix and verify
    run_ok(beu_cmd(&dir).args([
        "artifact",
        "changelog",
        "design",
        "updated for test additions",
    ]));
    run_ok(beu_cmd(&dir).args([
        "artifact",
        "changelog",
        "changelog",
        "logged test additions",
    ]));
    run_ok(beu_cmd(&dir).args(["check"]));

    // Pause with final progress check
    let stdout = run_ok(beu_cmd(&dir).args(["progress"]));
    assert!(stdout.contains("Tasks:"));
    run_ok(beu_cmd(&dir).args(["pause", "sprint complete, all docs updated"]));
}
