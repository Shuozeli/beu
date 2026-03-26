# Code Quality Findings

Audit date: 2026-03-26
Fix pass: 2026-03-26
Re-audit date: 2026-03-26

## 1. Stringly-Typed APIs (Missing Enums)

### 1.1 Status, Priority, Area, and Category Fields Use Raw Strings
- **Status: SKIPPED** -- Full enum conversion (with `clap::ValueEnum`, `Display`/`FromStr` for DB round-tripping, and store trait signature changes) is a large refactor that touches every module, every store trait, every SQLite implementation, and every test fake. The partial fix applied in 1.2 and 2.1 (module-level constants, validation for artifact type) addresses the immediate duplication and missing-validation issues. The full enum refactor is deferred as a separate tracked task.
- **Location:** `src/store.rs:11-18` (Artifact), `src/store.rs:72-81` (Task), `src/store.rs:128-138` (Idea), `src/store.rs:220-227` (StateEntry), `src/store.rs:263-270` (DebugSession)
- **Also at:** validation scattered across `src/cmd/artifact.rs`, `src/cmd/task.rs`, `src/cmd/idea.rs`, `src/cmd/state.rs`, `src/cmd/debug.rs`
- **Problem:** Every domain type uses `String` for status, priority, area, category, and test_status fields. Validation is performed ad hoc via `&[&str]` arrays in each command handler, duplicated across files.
- **Fix:** Define enums for each domain concept (`TaskStatus`, `TaskPriority`, `IdeaArea`, `StateCategory`, `ArtifactStatus`, `TestStatus`, `DebugStatus`). Derive `clap::ValueEnum` for CLI parsing and implement `Display`/`FromStr` for database round-tripping.

### 1.2 Artifact Type Is Unchecked
- **Status: DONE** -- Added `VALID_TYPES` constant in `src/cmd/artifact.rs` with validation in `cmd_add`. Also promoted the existing local `valid` array in `cmd_status` to a module-level `VALID_STATUSES` constant.
- **Location:** `src/cmd/artifact.rs:3-16` (cmd_add)
- **Problem:** Unlike status, the `artifact_type` field was never validated. The CLI doc comment mentions valid types, but the code accepted any string.
- **Fix:** Added `VALID_TYPES` and `VALID_STATUSES` module-level constants with validation.

## 2. Duplication

### 2.1 Duplicated Priority Validation Arrays
- **Status: DONE** -- Replaced all local validation arrays in `src/cmd/task.rs` with module-level constants: `VALID_STATUSES`, `VALID_PRIORITIES`, `VALID_TEST_STATUSES`. The three local arrays (`cmd_add`, `cmd_update` x2, `cmd_test_status`) are now single constants.
- **Location:** `src/cmd/task.rs`
- **Problem:** The valid priority and status values were defined as local arrays in multiple places.
- **Note:** The idea module intentionally uses `["low", "medium", "high"]` (no "critical") which is a deliberate design choice for lightweight ideas vs tasks.

### 2.2 Repeated Event-Logging Boilerplate in main.rs
- **Status: DONE** -- Extracted `run_timed()` helper that wraps timing and event logging. Also extracted `require_message()` to deduplicate the repeated empty-message validation pattern. Store is now opened once at the top of `run_with_project` instead of per-branch. Eliminated ~80 lines of boilerplate.
- **Location:** `src/main.rs`

### 2.3 Duplicated `setup()` Helper Across SQLite Test Modules
- **Status: SKIPPED** -- Low priority cosmetic issue. The 8 identical `setup()` functions are 5 lines each and tightly scoped to their test modules. The maintenance cost is minimal for a single-user CLI tool.
- **Location:** `src/sqlite/artifact.rs`, `src/sqlite/task.rs`, etc.

### 2.4 Duplicated FakeStore Implementations in cmd/ Tests
- **Status: SKIPPED** -- Medium priority but the consolidation would be a large refactor (~600 lines across 6 files). Each fake is specialized to its module's trait and tightly scoped. Deferring to a separate task.
- **Location:** `src/cmd/artifact.rs`, `src/cmd/task.rs`, etc.

## 3. Missing Abstractions

### 3.1 Repeated "Fetch-or-404 Then Update" Pattern in SQLite Implementations
- **Status: SKIPPED** -- Low-medium priority. The pattern is correct and the duplication is within the SQLite layer only (4 functions). Extracting a generic helper requires dynamic SQL table/column names which adds complexity. Deferring.
- **Location:** `src/sqlite/task.rs`, `src/sqlite/idea.rs`

### 3.2 list_artifacts / list_sessions / list_ideas Duplicate the "Optional Filter" SQL Pattern
- **Status: SKIPPED** -- Low-medium priority. The branching is localized and correct. Dynamic SQL construction (as in task.rs) would improve it but is not urgent.
- **Location:** `src/sqlite/artifact.rs`, `src/sqlite/debug.rs`, `src/sqlite/state.rs`, `src/sqlite/idea.rs`

## 4. Unused / Dead Code

### 4.1 `_repair` Parameter in cmd_health Is Unused
- **Status: DONE** -- Changed `_repair` to `repair` and added `eprintln!("warning: --repair is not yet implemented")` when the flag is passed. Users are no longer silently misled.
- **Location:** `src/cmd/system.rs`

### 4.2 `serde_json` Dependency Used Only for Admin Operations
- **Status: SKIPPED** -- No action needed per findings doc. The dependency is used for export/import which is valid functionality.

### 4.3 `walkdir` Dependency Used Only for Project Discovery
- **Status: SKIPPED** -- No action needed per findings doc. The dependency is used for project discovery which is valid functionality.

## 5. Unsafe Patterns / Robustness

### 5.1 `unwrap_or_default` on SystemTime Can Silently Return Epoch
- **Status: DONE** -- Changed `.unwrap_or_default()` to `.expect("system clock is before UNIX epoch")`. A pre-epoch clock is now a loud panic instead of silent data corruption.
- **Location:** `src/time_helper.rs:8-9`

### 5.2 generate_id Has Collision Risk Due to 24-bit Truncation
- **Status: DONE** -- Increased ID from 24 bits (6 hex digits) to 48 bits (12 hex digits) and added process PID to the hash input. Birthday-paradox 50% collision threshold moved from ~4000 IDs to ~16 million IDs.
- **Location:** `src/time_helper.rs:23-32`

### 5.3 `ref _cmd` Catch-All Borrows CLI Object Unnecessarily
- **Status: DONE** -- Replaced the `ref _cmd` catch-all in `run()` with explicit listing of all 16 variants that require project resolution. If a new `Commands` variant is added, the compiler will now force the developer to handle it in `run()`.
- **Location:** `src/main.rs` (run function)

## 6. Noise / Code Style

### 6.1 Excessive Section Dividers
- **Status: SKIPPED** -- Low priority cosmetic issue. The dividers are consistent with the project's existing style.

### 6.2 Redundant `let result = { ... }; result` Pattern
- **Status: SKIPPED** -- Low priority. The pattern is idiomatic for rusqlite borrow management.

### 6.3 Clippy: `require_message` Takes `Vec<String>` by Value
- **Status: FIXED** -- Changed `require_message(words: Vec<String>, ...)` to `require_message(words: &[String], ...)` to avoid needless ownership transfer. Updated all 9 call sites.
- **Location:** `src/main.rs:811-821`
- **Problem:** clippy::needless_pass_by_value flagged that the function joins the strings without consuming them, so a slice reference suffices.

### 6.4 Clippy: Manual `let...else` Patterns
- **Status: FIXED** -- Converted two match-to-early-return patterns to idiomatic `let...else` syntax.
- **Location:** `src/cmd/project.rs:115-118` (SqliteStore open_readonly), `src/sqlite/mod.rs:192-195` (import table row iteration)

## 7. Error Handling

### 7.1 Box<dyn Error> Used Throughout Instead of a Project Error Type
- **Status: PARTIALLY DONE** -- Removed the unused `anyhow` dependency from `Cargo.toml`. The error type decision (switching to `anyhow::Result` or a custom `BeuError` enum) is deferred as a larger architectural choice.
- **Location:** `Cargo.toml`

### 7.2 `log_event` Silently Discards Errors
- **Status: DONE** -- Changed `let _ = store.log_event(...)` to `if let Err(e) = store.log_event(...) { eprintln!("warning: failed to log event: {e}"); }`. Added doc comment explaining the intentional best-effort semantics.
- **Location:** `src/cmd/system.rs`

## 8. Potential Improvements (Not Bugs)

### 8.1 All Store Trait Methods Take `&mut self` Even for Reads
- **Status: SKIPPED** -- Medium priority architectural improvement. Would require changing every trait signature, every SQLite implementation, and every test fake. Deferring.

### 8.2 No Input Sanitization on User-Provided Strings
- **Status: SKIPPED** -- Low priority. The tool is used by agents and developers, not adversarial users.

## Summary

| Category | Total | Done | Skipped |
|---|---|---|---|
| Stringly-Typed APIs | 2 | 1 | 1 |
| Duplication | 4 | 2 | 2 |
| Missing Abstractions | 2 | 0 | 2 |
| Unused / Dead Code | 3 | 1 | 2 |
| Unsafe Patterns | 3 | 3 | 0 |
| Code Style | 4 | 2 | 2 |
| Error Handling | 2 | 1.5 | 0.5 |
| Potential Improvements | 2 | 0 | 2 |
| **Total** | **22** | **10.5** | **11.5** |

All findings marked DONE have been verified by `cargo test`, `cargo clippy`, and `cargo fmt`.
