# bea - Project Rules

## 1. Interaction Journal

Every session between the user and the agent must be journaled into
`docs/journal/YYYY-MM-DD.md`. Each entry should include:
- Date and time.
- Summary of what was discussed and decided.
- What was implemented or changed.
- Any open questions or follow-ups.

Append to the day's file if multiple sessions occur on the same day.

## 2. Composable and Testable Components

- Every component (service, repository, combat engine, loot generator, etc.) must
  be designed with a **trait/interface** so it can be composed and swapped.
- For testing, create **Fake implementations** of traits. Never use mocks.
  - Example: `FakePluginDb` that stores data in a `HashMap` in memory.
  - Example: `FakeHostFunction` that returns deterministic values.
- All fakes live under a `testutil` or `fakes` module within the relevant crate/module.
- Tests must exercise public behavior only. Do not expose private internals for testing.

## 3. Task Tracking for Complex Work

- When implementing a complex feature (anything spanning multiple files or requiring
  multiple phases), break the work into granular tasks and write them into
  `docs/tasks.md`.
- Each task entry should have: a status (`[ ]` pending, `[x]` done, `[-]` blocked),
  a description, and an optional note on blockers or decisions.
- Update `docs/tasks.md` as each phase is completed.
- Group tasks by phase (matching the phased implementation plan in the design doc).

## 4. Tech Stack Reminders

- Rust host with Extism (Wasm plugin runtime) and rusqlite (SQLite).
- Plugin SDK (`bea-sdk`) for shared ABI types between host and plugins.
- Plugins compile to `wasm32-unknown-unknown` using `extism-pdk`.
- All database access wrapped in transactions (reads included).
- No WASI -- all plugin capabilities granted via explicit host functions.
- Per-plugin SQLite isolation (`.bea/data/<plugin_name>.db`).
- JSON ABI across host-plugin boundary (serde types in `bea-sdk`).
- Use `pnpm` if any Node tooling is needed. Use `uv` if any Python tooling is needed.
- No `any` types (this is Rust, so use concrete types and generics).
- Prefer composition over inheritance.
- Fail-fast on missing config (no default values).
- Event log all state-changing actions.

## 5. Plugin ABI Contract

Every `.wasm` plugin must export:
- `metadata() -> String` -- returns `PluginMetadata` JSON.
- `run_command(input: String) -> String` -- accepts `CommandInput` JSON, returns `CommandOutput` JSON.

Plugin filename must match the `name` field in its metadata (e.g., `journal.wasm` -> `"name": "journal"`).

## 6. Host Functions Available to Plugins

| Function | Signature | Purpose |
|---|---|---|
| `host_log` | `(level, message) -> String` | Structured logging to stderr |
| `host_fs_read` | `(path) -> String` | Read file (sandboxed to project root) |
| `host_fs_write` | `(path, content) -> String` | Write file (sandboxed to project root) |
| `host_db_exec` | `(request_json) -> String` | Execute SQL on per-plugin SQLite |
| `host_time` | `() -> String` | Get current UTC timestamp (ISO 8601) |
| `host_env` | `(key) -> String` | Read environment context (project_root, plugin_name, bea_version) |

`host_log` calls are automatically persisted to `.bea/data/_logs.db` for later querying via `bea logs`.

## 7. Per-Plugin Config

Place `.bea/config/<plugin_name>.toml` to pass config to plugins.
The host reads the TOML, flattens it into dot-separated key-value pairs,
and passes them via Extism's config mechanism. Plugins read with
`extism_pdk::config::get("key")`.

## 8. Event Log

Every plugin command dispatch is automatically recorded in `.bea/data/_events.db`.
Each event captures: timestamp, plugin name, command, args, status (ok/error), duration (ms).

Query with `bea events [-n <limit>] [--plugin <name>]`.

## 9. Available Plugins

| Plugin | Commands | Description |
|---|---|---|
| `journal` | open, log, note, summary, close | Agent interaction ledger |
| `track` | add, status, list, show, remove | Artifact progress tracking |
| `agile` | add, list, update, done, show, sprint | Task and issue engine |

## 10. CLI Commands

| Command | Description |
|---|---|
| `bea init` | Initialize a new `.bea` project directory (plugins/, data/, config/, skill.md) |
| `bea install <path.wasm>` | Install a plugin from a local `.wasm` file |
| `bea uninstall <name> [--purge]` | Remove a plugin (--purge deletes its database) |
| `bea list` | List installed plugins and their commands |
| `bea status` | Show project overview (plugins, data sizes, recent activity) |
| `bea events [-n N] [--plugin P]` | Show recent event log entries |
| `bea logs [-n N] [--plugin P] [--level L]` | Show recent plugin log entries (from host_log) |
| `bea run <script> [--fail-fast]` | Run a batch script of plugin commands |
| `bea export <plugin>` or `bea export --all` | Export plugin data as JSON |
| `bea import <plugin> <file.json>` | Import data from JSON into plugin database |
| `bea reset <plugin> --force` | Drop all tables in a plugin's database |
| `bea config <plugin> [key] [value] [--delete]` | View or set plugin configuration |
| `bea version` | Show version and build information |
| `bea completions <shell>` | Generate shell completions (bash, zsh, fish) |
| `bea skill export` | Generate `.bea/skill.md` from loaded plugins |
| `bea skill info` | Print all available commands as JSON |
| `bea <plugin>` | Show plugin help (available commands) |
| `bea <plugin> <cmd> [args]` | Run a plugin command |

## 11. Build Commands

```bash
# Build all plugins
cargo build -p journal -p track -p agile --target wasm32-unknown-unknown --release

# Run host tests
cargo test -p bea-sdk -p bea-host

# Build the host binary
cargo build -p bea-host --release
```
