# beu -- Persistent Session Memory for AI Agents

`beu` is a single-binary CLI that gives AI coding agents (Claude Code, Gemini CLI,
etc.) persistent, structured memory. It survives conversation compaction, tracks work
across sessions, and keeps your project context in a local SQLite database.

Zero config. Offline-first. Instant startup. ~5MB binary.

> **Alpha software.** beu is under active development and not intended for
> general use. The CLI interface and SQLite data format may change between
> versions without migration support. Expect breaking changes.

## Installation

### From source (requires Rust toolchain)

```bash
git clone git@github.com:Shuozeli/beu.git
cd beu
cargo install --path .
```

### Verify

```bash
beu version
```

## Quick Start

```bash
# 1. Initialize beu in your project
cd /your/project
beu init

# 2. Install agent skill rules (teaches your AI agent how to use beu)
npx skills add Shuozeli/beu --all

# 3. Start using it
beu journal open
beu journal log "started working on auth module"
beu task add "implement login endpoint" --priority high
beu state set --category decision auth-method "JWT with RS256"
beu pause "auth module in progress"

# Next session
beu resume
```

## Installing Agent Skills

beu ships a `SKILL.md` that teaches Claude Code, Gemini CLI, and other AI agents
how to use beu automatically. Install skills using the
[skills](https://github.com/nicepkg/skills) CLI:

```bash
npx skills add Shuozeli/beu --all
```

This installs the beu skill into agent rule directories for all supported agents.

Or let beu handle it:

```bash
beu init            # installs skills on first init
beu update-rules    # force-refresh skill rules
```

## Session Protocol

The recommended workflow for AI agent sessions:

```bash
beu resume              # Check checkpoint, blockers, focus items
beu journal open        # Start interaction ledger
# ... do work ...
beu check               # Verify docs are up to date (if configured)
beu pause "state desc"  # Save checkpoint before stopping
```

## Modules

### journal -- Interaction Ledger

Session-based logging for tracking what happened during agent interactions.

| Command | Description |
|---|---|
| `beu journal open` | Start a new session |
| `beu journal log <message>` | Record a message |
| `beu journal note --tag <tag> <message>` | Categorized entry (decision, blocker, observation) |
| `beu journal summary` | Show entries for the current session |
| `beu journal close` | Close the current session |

### artifact -- Deliverable Tracking

Monitor the lifecycle of deliverables (docs, specs, configs, tests).

| Command | Description |
|---|---|
| `beu artifact add <name> [--type TYPE]` | Add an artifact (default type: doc) |
| `beu artifact status <name> <status>` | Update status: pending, in-progress, review, done |
| `beu artifact list [--filter STATUS]` | List artifacts, optionally filtered |
| `beu artifact show <name>` | Show artifact details |
| `beu artifact remove <name>` | Remove an artifact |

### task -- Sprint-Style Work Items

Task management with priority ordering, tags, and test status lifecycle.

| Command | Description |
|---|---|
| `beu task add <title> [--priority P] [--tag T]` | Create a task (low, medium, high, critical) |
| `beu task list [--status S] [--tag T]` | List with optional filters |
| `beu task update <id> [--status S] [--priority P] [--tag T]` | Update fields |
| `beu task done <id>` | Mark done |
| `beu task show <id>` | Show details |
| `beu task sprint` | Sprint view grouped by status |
| `beu task test-status <id> <status>` | Set test lifecycle status |

Test status lifecycle: `planned` -> `designed` -> `implemented` -> `tested` -> `darklaunched` -> `launched`

### state -- Persistent Key-Value Memory

Store decisions, blockers, focus items, and notes that persist across sessions.

| Command | Description |
|---|---|
| `beu state set --category <C> <key> <value>` | Set entry (upserts) |
| `beu state get [key]` | Get one or list all |
| `beu state list [--category C]` | Filter by category |
| `beu state remove <key>` | Delete entry |
| `beu state clear --category <C> --force` | Clear all in category |

Categories: `decision`, `blocker`, `focus`, `note`

### idea -- Lightweight Idea Capture

Quick backlog capture, separate from formal task tracking.

| Command | Description |
|---|---|
| `beu idea add <title> [--area A] [--priority P]` | Capture an idea |
| `beu idea list [--area A] [--status S]` | List with filters |
| `beu idea done <id>` | Mark done |
| `beu idea archive <id>` | Soft delete |
| `beu idea describe <id> <description>` | Add description |

Areas: `api`, `ui`, `database`, `testing`, `docs`, `tooling`, `general`

### debug -- Investigation Tracking

Structured debug sessions with timeline of symptoms, evidence, and root causes.

| Command | Description |
|---|---|
| `beu debug open <title>` | Open investigation (auto-generates slug) |
| `beu debug log <slug> <message>` | Append evidence |
| `beu debug symptom <slug> <description>` | Record symptom |
| `beu debug cause <slug> <description>` | Record root cause |
| `beu debug resolve <slug>` | Mark resolved |
| `beu debug list [--status S]` | List sessions |
| `beu debug show <slug>` | Show full timeline |

## System Commands

| Command | Description |
|---|---|
| `beu init` | Initialize `.beu/` directory and install agent skills |
| `beu status` | Project overview (modules, data size, last activity) |
| `beu events [-n N] [--module M]` | Query event audit log |
| `beu export <module>` / `--all` | Export module data as JSON |
| `beu import <module> <file.json>` | Import data from JSON |
| `beu reset <module> --force` | Drop all data in a module |
| `beu health [--repair]` | Validate `.beu/` directory integrity |
| `beu pause [message]` | Save checkpoint before pausing work |
| `beu resume` | Show checkpoint, blockers, and focus items |
| `beu progress` | Cross-module activity summary |
| `beu check` | Verify artifact compliance |
| `beu update-rules` | Force-refresh agent skill rules |
| `beu version` | Show version |

## Global Flags

| Flag | Description |
|---|---|
| `--beu-dir <path>` | Override `.beu` directory location |
| `-p, --project <id>` | Scope to a specific project (default: `default`) |
| `-v, --verbose` | Detailed output |
| `-q, --quiet` | Suppress non-essential output |

## Project Scoping

beu supports multiple isolated projects in a single `.beu/` directory:

```bash
beu -p frontend task add "fix nav bar"
beu -p backend task add "add rate limiting"
beu -p frontend task list    # only shows frontend tasks
```

## Data Directory

```
.beu/
  data/
    beu.db      # All module data (SQLite, WAL mode)
  config.yml    # Optional configuration
  .gitignore    # Excludes data/*.db
```

Add `.beu/` to your project's `.gitignore`. The data is local to each developer.

## Build & Test

```bash
cargo build                 # debug build
cargo build --release       # release build (~5MB)
cargo test                  # all tests
```

### Pre-commit Hooks

Install pre-commit hooks that run `cargo fmt --check` and `cargo clippy` before each commit:

```bash
./scripts/install-hooks
```

## Documentation

- [Architecture](docs/architecture.md) -- Technical design and internals
- [Codelab](docs/codelab.md) -- Step-by-step tutorial
- [Comparison](docs/comparison.md) -- beu vs get-shit-done vs beads
