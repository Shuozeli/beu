# beu Architecture

## Overview

`beu` is a CLI tool for agent workflows. All module logic is compiled
directly into the binary as native Rust code. No Wasm, no plugin discovery,
no runtime loading.

```
AI Agent (Claude, etc.)
    |
    | invokes `beu <module> <command> [args]`
    v
+-------------------+
|   beu CLI         |   Single Rust binary (~5MB)
|  +--------------+ |
|  | clap router  | |   Nested subcommands: 6 modules + system cmds
|  +--------------+ |
|  | journal.rs   | |   Session-based logging (5 commands)
|  | artifact.rs  | |   Deliverable progress tracking (5 commands)
|  | task.rs      | |   Work item tracking with sprint view (7 commands)
|  | state.rs     | |   Persistent project memory (5 commands)
|  | idea.rs      | |   Lightweight idea capture (6 commands)
|  | debug.rs     | |   Investigation tracking (7 commands)
|  +--------------+ |
|  | SqliteStore  | |   Single beu.db with project-scoped data
|  | EventLog     | |   events table for audit trail
|  +--------------+ |
+-------------------+
```

## Project Layout

```
src/
  main.rs                 # CLI entry, clap derive, command routing
  config.rs               # BeuConfig: YAML load/save, module gating, required docs
  store.rs                # Store traits + domain types (no SQL knowledge)
  rules.rs                # Skill file installation via npx skills add Shuozeli/beu
  time_helper.rs          # UTC timestamps, FNV hash ID generation
  cmd/
    mod.rs                # Re-exports
    artifact.rs           # Artifact commands (add, status, list, show, remove, describe, changelog, history)
    task.rs               # Task commands (add, list, update, done, show, sprint, test-status)
    journal.rs            # Journal commands (open, log, note, summary, close)
    state.rs              # State commands (set, get, list, remove, clear)
    idea.rs               # Idea commands (add, list, show, done, archive, describe)
    debug.rs              # Debug commands (open, log, symptom, cause, resolve, list, show)
    project.rs            # Cross-project commands (list, status, progress)
    system.rs             # System commands (init, status, events, export, import, reset, health, pause, resume, progress, check)
    testing.rs            # Test reference commands (patterns)
  sqlite/
    mod.rs                # SqliteStore struct, open/open_readonly, admin ops (export/import/reset/validate)
    artifact.rs           # ArtifactStore trait impl
    task.rs               # TaskStore trait impl
    journal.rs            # JournalStore trait impl
    state.rs              # StateStore trait impl
    idea.rs               # IdeaStore trait impl
    debug.rs              # DebugStore trait impl
    event_log.rs          # EventLogStore trait impl
    project.rs            # Project registration
tests/
  cli_artifact.rs         # Artifact CLI tests
  cli_task.rs             # Task CLI tests
  cli_journal.rs          # Journal CLI tests
  cli_state.rs            # State CLI tests
  cli_idea.rs             # Idea CLI tests
  cli_debug.rs            # Debug CLI tests
  cli_system.rs           # System command CLI tests
  cli_project.rs          # Cross-project CLI tests
  cli_project_scoping.rs  # Project isolation tests
  cli_testing.rs          # Test patterns CLI tests
  cli_workflows.rs        # Multi-module workflow tests
  common/mod.rs           # Shared test helpers
```

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }   # CLI argument parsing
rusqlite = { version = "0.31", features = ["bundled"] }  # SQLite (bundled, no system dep)
serde = { version = "1", features = ["derive"] }   # JSON serialization
serde_json = "1"                                    # JSON parsing
serde_yaml = "0.9"                                  # YAML config parsing
anyhow = "1"                                        # Error handling
walkdir = "2"                                        # Directory traversal (project discovery)

[dev-dependencies]
tempfile = "3"                                      # Temporary directories for tests
```

## Data Architecture

### Directory Structure

```
.beu/
  data/
    beu.db              # Single SQLite database (all modules, all projects)
  config.yml            # Module gating, compliance, project scoping
  .gitignore            # Excludes data/*.db
```

All tables live in a single `beu.db` file. The `project_id` column on every
table provides logical isolation between projects.

### Database Conventions

- **WAL mode**: `PRAGMA journal_mode=WAL` for concurrent read safety.
- **Foreign keys**: `PRAGMA foreign_keys=ON`.
- **Transactions**: All operations (reads included) are wrapped in transactions.
- **Schema auto-creation**: All tables are created in a single transaction in `SqliteStore::open()`.
- **Project scoping**: Every query filters by `project_id`.

### Schemas

All tables include a `project_id TEXT NOT NULL DEFAULT 'default'` column.

#### artifacts

```sql
CREATE TABLE IF NOT EXISTS artifacts (
    name TEXT NOT NULL,
    artifact_type TEXT NOT NULL DEFAULT 'doc',
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    project_id TEXT NOT NULL DEFAULT 'default',
    PRIMARY KEY (project_id, name)
);

CREATE TABLE IF NOT EXISTS artifact_changelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_name TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    project_id TEXT NOT NULL DEFAULT 'default',
    FOREIGN KEY (project_id, artifact_name) REFERENCES artifacts(project_id, name)
);
```

#### tasks

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    priority TEXT NOT NULL DEFAULT 'medium',
    tag TEXT,
    test_status TEXT NOT NULL DEFAULT 'planned',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    project_id TEXT NOT NULL DEFAULT 'default'
);
```

#### journal_sessions / journal_entries

```sql
CREATE TABLE IF NOT EXISTS journal_sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    closed_at TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    project_id TEXT NOT NULL DEFAULT 'default'
);

CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    message TEXT NOT NULL,
    tag TEXT,
    project_id TEXT NOT NULL DEFAULT 'default',
    FOREIGN KEY (session_id) REFERENCES journal_sessions(id)
);
```

#### state_entries

```sql
CREATE TABLE IF NOT EXISTS state_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    project_id TEXT NOT NULL DEFAULT 'default',
    UNIQUE(project_id, category, key)
);
```

#### ideas

```sql
CREATE TABLE IF NOT EXISTS ideas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    area TEXT NOT NULL DEFAULT 'general',
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    priority TEXT NOT NULL DEFAULT 'medium',
    project_id TEXT NOT NULL DEFAULT 'default',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

#### debug_sessions / debug_entries

```sql
CREATE TABLE IF NOT EXISTS debug_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'investigating',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    project_id TEXT NOT NULL DEFAULT 'default',
    UNIQUE(slug, project_id)
);

CREATE TABLE IF NOT EXISTS debug_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_slug TEXT NOT NULL,
    entry_type TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    project_id TEXT NOT NULL DEFAULT 'default',
    FOREIGN KEY (project_id, session_slug) REFERENCES debug_sessions(project_id, slug)
);
```

#### events

```sql
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    module TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    project_id TEXT NOT NULL DEFAULT 'default'
);

CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_module ON events(module);
```

#### projects

```sql
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL
);
```

## Store Trait Architecture

`store.rs` defines pure Rust traits for each module. `sqlite/` provides the
implementations. Command files depend only on traits and never see SQL.

```
store.rs (traits + domain types)
    ArtifactStore, TaskStore, JournalStore, StateStore,
    IdeaStore, DebugStore, EventLogStore
          |
          | implemented by
          v
sqlite/mod.rs (SqliteStore)
    Single struct that implements all 7 store traits.
    Opens one Connection to beu.db.
    project_id field scopes all queries.
```

This separation means command files receive `&mut impl XxxStore` and can
be tested with alternate implementations.

## Command Routing

`main.rs` uses clap's nested subcommand derive pattern:

```
Cli
  +-- Commands (top-level enum)
        +-- Init
        +-- UpdateRules
        +-- Journal { JournalAction }
        |     +-- Open, Log, Note, Summary, Close
        +-- Artifact { ArtifactAction }
        |     +-- Add, Status, List, Show, Remove, Describe, Changelog, History
        +-- Task { TaskAction }
        |     +-- Add, List, Update, Done, Show, Sprint, TestStatus
        +-- State { StateAction }
        |     +-- Set, Get, List, Remove, Clear
        +-- Idea { IdeaAction }
        |     +-- Add, List, Show, Done, Archive, Describe
        +-- Debug { DebugAction }
        |     +-- Open, Log, Symptom, Cause, Resolve, List, Show
        +-- Test { TestAction }
        |     +-- Patterns
        +-- Project { ProjectAction }
        |     +-- List, Status, Progress
        +-- Pause, Resume, Progress, Health, Check
        +-- Status, Events, Export, Import, Reset, Version
```

## .beu Directory Resolution

`resolve_beu_dir()` follows this precedence:

1. If `--beu-dir <path>` is provided, use that path directly.
2. Otherwise, walk up from CWD checking each ancestor for a `.beu/` directory.
3. If no `.beu/` is found, return an error suggesting `beu init`.

This allows running `beu` commands from any subdirectory of the project.

## Event Logging

Every module command is automatically event-logged after execution:

```rust
let start = std::time::Instant::now();
let result = cmd::artifact::cmd_add(&mut store, name, artifact_type, description);
let duration_ms = start.elapsed().as_millis() as i64;
let status = if result.is_ok() { "ok" } else { "error" };
log_event(&mut store, "artifact", "add", &args_str, status, duration_ms);
```

This provides a complete audit trail queryable via `beu events`.

## ID Generation (time_helper.rs)

Unique IDs are generated using:

1. Current UTC timestamp (ISO 8601 with millisecond precision).
2. Atomic counter (monotonically increasing across the process lifetime).
3. FNV-1a hash of timestamp bytes + counter bytes.
4. Result formatted as `<prefix>-<16 hex digits>` (e.g., `j-a1b2c3d4e5f6a7b8`).

This produces IDs that are:
- Unique within a process (atomic counter prevents collisions).
- Human-readable prefix indicates type (`j` = journal session, `e` = entry).
- Fixed length (18 chars including prefix and dash).

## Skill Rule Delivery

Agent rules are not embedded in the binary. They are installed by shelling
out to `npx skills add Shuozeli/beu --all --copy`:

- `beu init` calls `install_skills(root, false)` -- non-fatal if it fails.
- `beu update-rules` calls `install_skills(root, true)` -- fatal on error.

The `skills` CLI clones the `Shuozeli/beu` repo, discovers the root `SKILL.md`,
and installs it into agent rule directories (`.claude/`, `.gemini/`, `.agent/`).

## Test Architecture

| Suite | Count | Scope |
|---|---|---|
| Unit tests (store, sqlite, config, time_helper, rules) | ~200 | Trait impls, project isolation, admin ops, edge cases |
| `cli_artifact.rs` | 12 | Artifact CRUD via CLI |
| `cli_task.rs` | 18 | Task CRUD, sprint, test-status via CLI |
| `cli_journal.rs` | 11 | Journal session lifecycle via CLI |
| `cli_state.rs` | 14 | State CRUD, categories via CLI |
| `cli_idea.rs` | 9 | Idea CRUD via CLI |
| `cli_debug.rs` | 8 | Debug investigation lifecycle via CLI |
| `cli_system.rs` | 48 | Init, status, events, export/import, reset, health, pause/resume, progress, check |
| `cli_project.rs` | 6 | Cross-project discovery via CLI |
| `cli_project_scoping.rs` | 8 | Project isolation via CLI |
| `cli_testing.rs` | 7 | Test patterns via CLI |
| `cli_workflows.rs` | 9 | Multi-module integration workflows |
| **Total** | **~350** | |

CLI tests use `tempfile::TempDir` for isolation and `env!("CARGO_BIN_EXE_beu")`
to locate the compiled binary. Each test initializes its own `.beu/` directory.

## Key Design Decisions

1. **Native modules over Wasm** -- Eliminates ~20MB of wasmtime dependency and
   plugin loading overhead. Trades extensibility for speed.
2. **Single database** -- All tables live in one `beu.db` file. `project_id`
   column on every table provides logical project isolation.
3. **Store trait pattern** -- `store.rs` defines pure traits; `sqlite/` implements
   them. Command files depend only on traits for testability.
4. **Schema auto-creation** -- All tables are created in `SqliteStore::open()` in
   a single transaction. No migration system needed.
5. **Atomic ID generation** -- FNV hash of timestamp + atomic counter provides
   unique IDs without external dependencies (no UUID crate).
6. **Fail-fast** -- Missing `.beu/` directory, invalid status values, nonexistent
   tasks/artifacts all produce immediate clear errors with exit code 1.
7. **Event logging** -- Every module command is automatically event-logged for
   debugging and auditing.
8. **Transactional everything** -- All database operations (including reads) are
   wrapped in transactions per project convention.
9. **Single crate** -- No workspace, no sub-crates. Everything in one `Cargo.toml`
   for simplicity.
10. **External skill delivery** -- Agent rules are delivered via npm package,
    not embedded in the binary. This allows updating rules without recompiling.
