# bea - Task Tracking

## Phase 1: Core Host + SDK + Journal Plugin

- [x] Create Cargo workspace layout (`tools/bea/`)
- [x] Implement `bea-sdk` shared ABI types
- [x] Implement `bea-host` CLI scaffold (clap)
- [x] Implement host functions (`host_log`, `host_fs_read`, `host_fs_write`, `host_db_exec`, `host_time`)
- [x] Implement per-plugin SQLite (`db.rs`)
- [x] Implement plugin manager (discovery, loading, dispatch)
- [x] Implement skill export (Markdown + JSON)
- [x] Implement `bea init`
- [x] Build `journal` plugin (open/log/note/summary/close)
- [x] End-to-end verification

## Phase 2: Hardening

- [x] Add integration tests (`bea-host/tests/integration.rs`) -- 33 tests across all plugins
- [-] Add `host_random` host function -- dropped; keeping sandbox minimal, IDs use host_time + counter hash
- [x] Add `--verbose` / `--quiet` flags to CLI
- [x] Handle graceful errors when no plugins are installed (helpful dispatch errors)
- [x] Add per-plugin config support (`.bea/config/<plugin>.toml` via Extism config)

## Phase 3: Additional Plugins

- [x] `track.wasm` -- Artifact progress tracking (add/status/list/show/remove)
- [x] `agile.wasm` -- Task and issue engine (add/list/update/done/show/sprint)

## Phase 4: Operations & Observability

- [x] DB connection pooling per plugin (reuse connections across calls)
- [x] `bea install <path.wasm>` -- Install plugin with validation and auto skill.md update
- [x] `bea uninstall <name> [--purge]` -- Remove plugin, optionally purge DB
- [x] `bea list` -- Show installed plugins with version, command count, DB size
- [x] `bea events` -- Event log for all dispatched commands (timestamp, plugin, command, args, status, duration)
- [x] Event log auto-records every `dispatch()` call in `.bea/data/_events.db`
- [x] Millisecond-precision timestamps in `host_time` and event log
- [x] Plugin help: `bea <plugin>` shows available commands
- [x] Enhanced `bea init`: creates config/ dir, generates initial skill.md

## Phase 5: Polish

- [x] `bea status` -- Project overview (plugin count, commands, DB sizes, recent activity, event count)
- [x] `bea completions <shell>` -- Shell completions for bash/zsh/fish via clap_complete
- [x] CLI-level binary tests (`bea-host/tests/cli.rs`) -- 16 tests via subprocess
- [x] Fix `host_time` millisecond precision (ID collision bug)

## Phase 6: Robustness & UX

- [x] Graceful `host_db_exec` error handling -- SQL errors return `DbExecResponse.error` instead of causing wasm trap
- [x] `bea config <plugin> [key] [value] [--delete]` -- View/set/delete plugin config from CLI

## Phase 7: Productivity Features

- [x] `bea run <script> [--fail-fast]` -- Batch execution of plugin commands from a script file
- [x] `bea export <plugin>` / `bea export --all` -- Export plugin data as JSON for backup/migration
- [x] `bea import <plugin> <file.json>` -- Import data from JSON into plugin database
- [x] `bea reset <plugin> --force` -- Drop all tables in plugin database
- [x] `bea version` -- Show version and build info
- [x] `host_env` host function -- Plugins can read project_root, plugin_name, bea_version

## Phase 8: Quality & Observability

- [x] Replace all `unwrap()` calls in production code with proper error handling (db.rs, plugin_manager.rs)
- [x] Add missing unit tests: EventLog::count, PluginDb::list_plugin_dbs, ensure_dir, export/import roundtrip, titlecase, detect_project_name, PluginLog
- [x] `bea logs [-n N] [--plugin P] [--level L]` -- Persistent plugin log viewer
- [x] `host_log` calls now persisted to `.bea/data/_logs.db` alongside stderr output

## Bug Fixes

- [x] `host_time` only had second precision, causing ID collisions when CLI commands run < 1s apart. Fixed with millisecond precision.
- [x] `host_db_exec` returned `anyhow::Error` on SQL failures, causing wasm trap. Changed to return `DbExecResponse::err()` so plugins handle errors gracefully.

## Test Summary

- **bea-sdk**: 4 unit tests (serialization roundtrips)
- **bea-host**: 22 unit tests (path sandboxing, skill rendering, DB ops, event log, plugin log, import/export roundtrip, titlecase)
- **integration**: 34 tests (metadata, skill export, journal workflow, track workflow, agile workflow, error handling, empty plugins, config, event log, graceful DB errors)
- **CLI**: 34 tests (init, list, install/uninstall, journal workflow, plugin help, status, events, logs, completions, config, export, import, reset, run, version, error cases)
- **Total**: 94 tests
