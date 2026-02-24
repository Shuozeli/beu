# bea Architecture

## Overview

`bea` is a universal sandboxed WebAssembly plugin host for AI agents. It loads
`.wasm` plugins at runtime, grants them capabilities through explicit host
functions, and generates skill manifests that AI agents can consume to discover
available commands.

```
AI Agent (Claude, etc.)
    |
    | reads skill.md
    | invokes `bea <plugin> <command> [args]`
    v
+-------------------+
|   bea CLI (host)  |   Rust binary (~25MB, includes wasmtime)
|  +--------------+ |
|  |  clap router | |   20 built-in commands + dynamic plugin dispatch
|  +--------------+ |
|  | PluginRegis- | |   Discovers *.wasm in .bea/plugins/
|  | try          | |   Calls metadata(), caches PluginMetadata
|  +--------------+ |
|  | Host Funcs   | |   6 host functions registered per plugin
|  +--------------+ |
|  | PluginDb     | |   Per-plugin SQLite with connection pooling
|  | EventLog     | |   Shared _events.db for audit trail
|  | PluginLog    | |   Shared _logs.db for persistent logging
|  +--------------+ |
+-------------------+
    |         |
    v         v
+--------+ +--------+ +--------+
|journal | |track   | |agile   |   wasm32-unknown-unknown (~255KB each)
|.wasm   | |.wasm   | |.wasm   |   Built with extism-pdk
+--------+ +--------+ +--------+
```

## Workspace Layout

```
tools/bea/
  Cargo.toml                    # Workspace root (resolver = "2")
  CLAUDE.md                     # Project rules and conventions
  docs/
    architecture.md             # This file
    tasks.md                    # Phase-based task tracking
    journal/                    # Session journals
  bea-sdk/                      # Shared ABI types (pure data crate)
    Cargo.toml
    src/lib.rs
  bea-host/                     # CLI binary + library
    Cargo.toml
    src/
      main.rs                   # Entry point, clap CLI, 20 commands
      lib.rs                    # Library re-exports (db, plugin_manager, skill, host_functions)
      host_functions.rs         # 6 host functions + HostContext + path sandboxing
      plugin_manager.rs         # Plugin discovery, loading, dispatch, event logging
      skill.rs                  # Skill export (Markdown + JSON)
      db.rs                     # PluginDb, EventLog, PluginLog
    tests/
      integration.rs            # 34 Extism-level integration tests
      cli.rs                    # 34 binary-level CLI tests
  plugins/
    journal/                    # Agent interaction ledger (5 commands)
      Cargo.toml
      src/lib.rs
    track/                      # Artifact progress tracking (5 commands)
      Cargo.toml
      src/lib.rs
    agile/                      # Task and issue engine (6 commands)
      Cargo.toml
      src/lib.rs
```

## Crate Dependency Graph

```
bea-sdk (pure types, no runtime deps)
   ^               ^
   |               |
bea-host        plugins/*
(extism,        (extism-pdk,
 rusqlite,       bea-sdk)
 clap,
 bea-sdk)
```

- `bea-sdk` has zero host or plugin runtime dependencies. Both sides import it.
- `bea-host` depends on `extism` (Wasm runtime), `rusqlite` (SQLite), `clap` (CLI).
- Plugins depend on `extism-pdk` (plugin development kit) and `bea-sdk`.
- Plugins compile to `wasm32-unknown-unknown` (no WASI).

## Security Model

### WASI Disabled

All plugins run with `with_wasi(false)`. They have **zero** default access to
the filesystem, network, environment, or system clock. Every capability is
granted through explicit host functions.

### Path Sandboxing

`host_fs_read` and `host_fs_write` resolve paths relative to the project root.
The implementation:

1. Joins the user path with the project root.
2. Canonicalizes the result (resolves symlinks and `..` components).
3. Verifies the resolved path starts with the canonicalized project root.
4. Rejects any path that escapes the sandbox.

### Per-Plugin Database Isolation

Each plugin gets its own SQLite database at `.bea/data/<plugin_name>.db`.
Plugins cannot access each other's data. The host routes `host_db_exec` calls
to the correct database based on the `plugin_name` in the `HostContext`.

### Host Function Allowlist

Plugins can only call the 6 registered host functions. There is no way for a
plugin to execute arbitrary code, spawn processes, or make network requests.

## Host Functions

| Function | Signature | Purpose |
|---|---|---|
| `host_log` | `(level: PTR, message: PTR) -> PTR` | Structured logging to stderr + persistent storage in `_logs.db` |
| `host_fs_read` | `(path: PTR) -> PTR` | Read file (sandboxed to project root) |
| `host_fs_write` | `(path: PTR, content: PTR) -> PTR` | Write file (sandboxed to project root) |
| `host_db_exec` | `(request_json: PTR) -> PTR` | Execute SQL on per-plugin SQLite. Returns error responses (not traps) on SQL failures |
| `host_time` | `() -> PTR` | Get current UTC timestamp (ISO 8601 with millisecond precision) |
| `host_env` | `(key: PTR) -> PTR` | Read environment context: `project_root`, `plugin_name`, `bea_version` |

All host functions use `UserData<HostContext>` for shared state:

```rust
pub struct HostContext {
    pub project_root: PathBuf,
    pub plugin_name: String,
    pub db: Arc<Mutex<PluginDb>>,
    pub plugin_log: Option<Arc<Mutex<PluginLog>>>,
}
```

## Plugin ABI Contract

Every `.wasm` plugin must export two functions:

| Export | Signature | Returns |
|---|---|---|
| `metadata()` | `() -> String` | `PluginMetadata` JSON |
| `run_command(input)` | `(String) -> String` | `CommandOutput` JSON |

Plugin filename must match the `name` field in its metadata
(e.g., `journal.wasm` -> `"name": "journal"`).

### SDK Types (bea-sdk)

```
PluginMetadata { name, version, description, commands: [CommandDef] }
CommandDef { name, description, args: [ArgDef] }
ArgDef { name, description, required }
CommandInput { command, args: [String] }
CommandOutput { status: Ok|Error, message, data?: Value }
DbExecRequest { sql, params: [Value] }
DbExecResponse { columns, rows, rows_affected, error? }
```

## Data Architecture

### Directory Structure

```
.bea/
  plugins/                      # Wasm plugin files
    journal.wasm
    track.wasm
    agile.wasm
  data/                         # SQLite databases
    journal.db                  # Per-plugin data
    track.db
    agile.db
    _events.db                  # Shared event log (audit trail)
    _logs.db                    # Shared plugin log (from host_log calls)
  config/                       # Per-plugin TOML configuration
    journal.toml                # Optional
    track.toml
  skill.md                      # Generated agent skill manifest
```

### Database Conventions

- **WAL mode**: All databases use `PRAGMA journal_mode=WAL` for concurrent read safety.
- **Transactions**: All operations (reads included) are wrapped in transactions.
- **Connection pooling**: `PluginDb` caches connections per plugin name in a `HashMap`.
- **Internal DBs**: Files starting with `_` (e.g., `_events.db`, `_logs.db`) are internal and excluded from plugin listings.

### Event Log Schema (`_events.db`)

```sql
events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,       -- ISO 8601 with milliseconds
    plugin TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,          -- "ok" or "error"
    duration_ms INTEGER NOT NULL DEFAULT 0
)
```

Every `dispatch()` call auto-records an event. Query with `bea events`.

### Plugin Log Schema (`_logs.db`)

```sql
logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    plugin TEXT NOT NULL,
    level TEXT NOT NULL,           -- info, warn, error, debug
    message TEXT NOT NULL
)
```

Every `host_log` call persists here alongside stderr output. Query with `bea logs`.

## Plugin Loading Flow

```
1. PluginRegistry::load(bea_dir)
2.   Scan .bea/plugins/*.wasm (sorted by name)
3.   For each .wasm:
4.     Create HostContext { project_root, plugin_name, db, plugin_log }
5.     Build 6 host functions with HostContext as UserData
6.     Load plugin config from .bea/config/<name>.toml (flatten to dot-separated keys)
7.     Create Manifest with wasm file + config
8.     Build Plugin via PluginBuilder (wasi=false, with_functions)
9.     Call metadata() -> validate name matches filename
10.    Cache PluginMetadata
11.  Open EventLog (non-fatal on failure)
12.  Return PluginRegistry
```

## Command Dispatch Flow

```
1. CLI parses: bea <plugin> <command> [args]
2. PluginRegistry::dispatch(plugin_name, command, args)
3.   Serialize CommandInput { command, args } to JSON
4.   Call plugin.run_command(json) via Extism
5.   Plugin processes:
6.     Match command name
7.     Call host functions as needed (host_db_exec, host_time, host_log, etc.)
8.     Return CommandOutput JSON
9.   Deserialize CommandOutput
10.  Record event in EventLog (timestamp, plugin, command, args, status, duration_ms)
11.  Return output to CLI for display
```

## Configuration System

Per-plugin configuration lives in `.bea/config/<plugin_name>.toml`. The host:

1. Reads the TOML file on plugin load.
2. Flattens nested tables into dot-separated keys (e.g., `[db] pool_size = "5"` becomes `db.pool_size = "5"`).
3. Passes the flat map via `Manifest::with_config()`.
4. Plugins read with `extism_pdk::config::get("key")`.

CLI management: `bea config <plugin> [key] [value] [--delete]`.

## Skill Export

`bea skill export` generates `.bea/skill.md` containing:

1. System commands (all 20 built-in CLI commands).
2. Per-plugin sections with command signatures and descriptions.

This file is designed to be consumed by AI agents (e.g., included in a CLAUDE.md
or system prompt) so they know what commands are available.

`bea skill info` outputs the same data as JSON for programmatic consumption.

## Plugins

### journal (5 commands)

Agent interaction ledger. Tracks sessions with timestamped entries.

| Command | Description |
|---|---|
| `open` | Create a new session |
| `log <message>` | Record a message in the current session |
| `note --tag <tag> <message>` | Record a categorized entry (decision, blocker, observation) |
| `summary` | Show all entries for the current session |
| `close` | Close the current session |

Schema: `sessions(id, started_at, closed_at, status)`, `entries(id, session_id, created_at, message, tag)`.

### track (5 commands)

Artifact progress tracking. Monitors the lifecycle of deliverables.

| Command | Description |
|---|---|
| `add <name> [--type TYPE]` | Add a new artifact to track |
| `status <name> <status>` | Update artifact status (pending/in-progress/review/done) |
| `list [--filter STATUS]` | List artifacts, optionally filtered by status |
| `show <name>` | Show artifact details |
| `remove <name>` | Remove an artifact |

Schema: `artifacts(name PK, artifact_type, status, created_at, updated_at)`.

### agile (6 commands)

Task and issue engine for sprint-style project management.

| Command | Description |
|---|---|
| `add <title> [--priority P] [--tag T]` | Create a new task |
| `list [--status S] [--tag T]` | List tasks with optional filters |
| `update <id> [--status S] [--priority P] [--tag T]` | Update task fields |
| `done <id>` | Mark task as done |
| `show <id>` | Show task details |
| `sprint` | Sprint view grouped by status, ordered by priority |

Schema: `tasks(id AUTOINCREMENT, title, status, priority, tag, created_at, updated_at)`.

## CLI Commands (20 total)

| Command | Category | Description |
|---|---|---|
| `bea init` | Setup | Initialize `.bea` directory structure |
| `bea install <path.wasm>` | Plugin Mgmt | Install plugin with validation, auto-update skill.md |
| `bea uninstall <name> [--purge]` | Plugin Mgmt | Remove plugin, optionally purge DB |
| `bea list` | Plugin Mgmt | List plugins with version, command count, DB size |
| `bea status` | Observability | Project overview (plugins, data, recent activity) |
| `bea events [-n N] [--plugin P]` | Observability | Query event log |
| `bea logs [-n N] [--plugin P] [--level L]` | Observability | Query plugin log |
| `bea config <plugin> [key] [value] [--delete]` | Configuration | View/set/delete plugin config |
| `bea export <plugin>` / `--all` | Data Mgmt | Export plugin data as JSON |
| `bea import <plugin> <file.json>` | Data Mgmt | Import data from JSON |
| `bea reset <plugin> --force` | Data Mgmt | Drop all tables in plugin DB |
| `bea run <script> [--fail-fast]` | Automation | Batch execute plugin commands from script |
| `bea version` | Info | Show version and build info |
| `bea completions <shell>` | Info | Generate shell completions (bash/zsh/fish) |
| `bea skill export` | Skill | Generate skill.md from loaded plugins |
| `bea skill info` | Skill | Print all commands as JSON |
| `bea <plugin>` | Plugin | Show plugin help (available commands) |
| `bea <plugin> <cmd> [args]` | Plugin | Run a plugin command |

## Test Architecture

| Suite | Count | Scope |
|---|---|---|
| `bea-sdk` unit tests | 4 | Serialization roundtrips for SDK types |
| `bea-host` unit tests | 22 | Path sandboxing, skill rendering, DB ops, EventLog, PluginLog, import/export roundtrip, titlecase |
| Integration tests | 34 | Extism-level: metadata, skill export, plugin workflows, error handling, config, graceful DB errors |
| CLI tests | 34 | Binary-level via `std::process::Command`: all CLI commands, error cases, exit codes |
| **Total** | **94** | |

## Build Commands

```bash
# Build all plugins (wasm32-unknown-unknown)
cargo build -p journal -p track -p agile --target wasm32-unknown-unknown --release

# Run all tests
cargo test -p bea-sdk -p bea-host

# Build the host binary
cargo build -p bea-host --release
```

## Key Design Decisions

1. **No WASI** -- Security-first. Plugins get zero default access. All capabilities are explicit.
2. **JSON ABI** -- Simple, debuggable, language-agnostic. Future plugins in Go/JS/etc use the same JSON contract.
3. **Per-plugin SQLite** -- Complete data isolation. No cross-plugin data collisions.
4. **Dynamic dispatch** -- Host doesn't hardcode plugin names. Drop a `.wasm` in the plugins dir and it's available.
5. **Graceful DB errors** -- SQL failures return `DbExecResponse.error` instead of wasm traps, so plugins can handle errors.
6. **Event logging** -- Every state-changing action is automatically recorded for debugging and auditing.
7. **Persistent plugin logs** -- `host_log` calls are stored alongside stderr output for retrospective analysis.
8. **Fail-fast** -- Missing config, corrupt wasm, bad metadata -> immediate clear error. No silent fallbacks.
9. **Connection pooling** -- `PluginDb` caches SQLite connections per plugin for the lifetime of the registry.
10. **No MCP server** -- CLI-only. All plugins are invoked via `bea` CLI commands.
