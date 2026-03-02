# beu CLI Reference

Complete command reference for the `beu` CLI.

## Global Options

- `--beu-dir <PATH>`: Path to the `.beu` directory (default: `.beu` in current or ancestor directory)
- `-p, --project <ID>`: Project ID to scope all commands to (default: from config `default_project`)
- `-v, --verbose`: Show detailed output
- `-q, --quiet`: Suppress all non-essential output
- `-h, --help`: Print help

## System Commands

### init
Initialize a new `.beu` project directory and scaffold agent rules.
```bash
beu init
```
Creates:
- `.beu/data/beu.db` -- SQLite database with all module tables
- `.beu/config.yml` -- module and compliance configuration
- `.claude/rules/beu.md` -- agent rules for Claude Code
- `.gemini/rules/beu.md` -- agent rules for Gemini
- `.agent/rules/beu.md` -- agent rules for Antigravity

Agent rule files are skipped if they already exist (user customizations are preserved).

### update-rules
Overwrite agent rule files with the latest embedded content. Use this after upgrading `beu` to propagate new agent instructions to already-initialized projects.
```bash
beu update-rules
```
Always overwrites all three rule files (`.claude/rules/beu.md`, `.gemini/rules/beu.md`, `.agent/rules/beu.md`).

### resume
Resume work: show checkpoint, blockers, and focus items.
```bash
beu resume
```

### pause
Save a checkpoint before pausing work.
```bash
beu pause "checkpoint message"
```

### progress
Cross-module progress summary. Shows checkpoints, blockers, task counts, artifact status, ideas, and active debug sessions.
```bash
beu progress
```

### status
Show project status overview, including enabled modules and database size.
```bash
beu status
```

### check
Compliance gate: verify all required docs are registered, not pending, and not stale.
```bash
beu check
```
Checks each artifact listed in `required_docs` config. Fails if any doc is missing, pending, or has too many mutation events since last update (controlled by `staleness_threshold`).

### health
Validate `.beu` directory integrity.
```bash
beu health [--repair]
```

### events
Show recent event log entries.
```bash
beu events [-n limit] [-m module]
```

## Module Commands

### journal
Agent interaction ledger.
- `open`: Start a new journal session.
- `log <msg>`: Record a message in the current session.
- `note <category> <msg>`: Record a categorized note.
- `summary`: Show a digest of the current session.
- `close`: Close the current session.

### artifact
Deliverable tracking.
- `add <name> [--status status]`: Add a new artifact.
- `status <name> <status>`: Update artifact status.
- `list`: List all artifacts.
- `show <name>`: Show artifact details.
- `changelog <name>`: Show artifact status history.

### task
Work items with sprint view and per-task test status tracking.
- `add <title> [--priority priority] [--tag tag]`: Add a new task.
- `list [--status status] [--tag tag] [--test-status status]`: List tasks.
- `update <id> [--status status] [--priority priority] [--tag tag]`: Update a task.
- `done <id>`: Mark a task as completed.
- `test-status <id> <status>`: Update test status. Values: `planned`, `designed`, `implemented`, `tested`, `darklaunched`, `launched`.
- `sprint`: Show tasks grouped by status. Includes test status.

### test
Agent reference for test patterns and the test status lifecycle.
- `patterns`: Show built-in test patterns and lifecycle.

### state
Persistent project memory (decisions, blockers, focus, notes).
- `set <key> <value> [--category category]`: Set a state value.
- `get <key>`: Get a state value.
- `list [--category category]`: List state items.
- `remove <key>`: Remove a state item.

### idea
Lightweight idea capture.
- `add <title> [--status status]`: Add a new idea.
- `list [--status status]`: List ideas.
- `done <id>`: Mark an idea as completed.
- `archive <id>`: Archive an idea.

### debug
Persistent investigation tracking.
- `open <title> <symptom>`: Open a new debug investigation session.
- `log <msg>`: Log evidence in a debug session.
- `symptom <msg>`: Record a symptom.
- `cause <msg>`: Record root cause.
- `resolve <resolution>`: Mark as resolved.
- `list`: List debug sessions.
- `show <id>`: Show debug session timeline.

## Data Management

### export
Export module data as JSON.
```bash
beu export <module> [--all]
```

### import
Import data from a JSON file into a module's database.
```bash
beu import <module> <file>
```

### reset
Reset a module's database (deletes data for the current project).
```bash
beu reset <module> [--force]
```

## Cross-Project Commands

### project list
Discover all `.beu` projects in the repository.
```bash
beu project list [--name <NAME>]
```

### project status
Show status across discovered projects (modules, database size).
```bash
beu project status [--name <NAME>]
```

### project progress
Show progress summary across all discovered projects.
```bash
beu project progress [--name <NAME>]
```

## Configuration Reference

`.beu/config.yml` controls behavior:

```yaml
modules:
  journal: true
  artifact: true
  task: true
  state: true
  idea: true
  debug: true

# Required documentation artifacts for `beu check`
required_docs:
  - name: design
    type: doc
  - name: changelog
    type: changelog

# Project scoping
require_project: false      # When true, --project flag is mandatory
default_project: default    # Project ID used when --project is omitted

# Staleness detection (for `beu check`)
staleness_threshold: 10     # Fail check if N+ mutation events since last doc update

# Optional: override built-in test patterns (unit, integration, systest, golden)
test_patterns:
  - key: property
    description: QuickCheck/proptest-based property testing.
```
