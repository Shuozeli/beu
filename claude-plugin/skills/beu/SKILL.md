---
name: beu
description: >
  Lightweight session memory CLI for agent workflows. Provides persistent
  journal, artifact, task, state, idea, and debug tracking across sessions
  and conversation compaction.
allowed-tools: "Read,Bash(beu:*)"
version: "0.1.0"
author: "Shuoze Li"
---

# beu - Session Memory for AI Agents

SQLite-backed CLI that persists agent workflow state. Six modules cover the
full lifecycle: plan, track, record, investigate, and resume.

## beu vs TodoWrite

| beu (persistent) | TodoWrite (ephemeral) |
|-------------------|----------------------|
| Multi-session work | Single-session tasks |
| Structured modules | Linear checklist |
| Survives compaction | Conversation-scoped |
| SQLite-backed | In-memory only |

**Decision test**: "Will I need this context after this session?" -> YES = beu

## Prerequisites

```bash
beu --version  # Requires v0.1.0+
```

- **beu CLI** installed and in PATH
- **Initialization**: `beu init` run once in project root

## What `beu init` Creates

Running `beu init` in a project root scaffolds:

```
.beu/
  data/beu.db      SQLite database (all modules)
  config.yml       Module and compliance configuration
.claude/rules/beu.md   Agent rules for Claude Code
.gemini/rules/beu.md   Agent rules for Gemini
.agent/rules/beu.md    Agent rules for Antigravity
```

The agent rule files teach each AI agent the session protocol, available
commands, and workflow patterns. They are skipped if already present.

## Module Quick Reference

| Module | Purpose | Key Commands |
|--------|---------|--------------|
| **journal** | Agent interaction ledger | `open`, `log`, `note`, `summary`, `close` |
| **artifact** | Deliverable tracking | `add`, `status`, `list`, `show`, `changelog` |
| **task** | Work items with sprint view | `add`, `list`, `update`, `done`, `sprint` |
| **state** | Persistent memory (decisions, blockers) | `set`, `get`, `list`, `remove` |
| **idea** | Lightweight idea capture | `add`, `list`, `done`, `archive` |
| **debug** | Investigation tracking | `open`, `log`, `symptom`, `cause`, `resolve` |

## Configuration

`.beu/config.yml` controls modules, compliance, and project scoping:

```yaml
modules:
  journal: true
  artifact: true
  task: true
  state: true
  idea: true
  debug: true

# Required docs for compliance checking (beu check)
required_docs:
  - name: design
    type: doc
  - name: changelog
    type: changelog

# Staleness: fail check if N+ mutation events since last doc update
staleness_threshold: 10

# Project scoping
require_project: false
default_project: default
```

Set a module to `false` to disable its CLI commands.

## Session Protocol

1. `beu resume` -- Check for checkpoint, blockers, focus items
2. Work on tasks using module commands
3. `beu check` -- Verify docs are up to date after major changes
4. `beu pause "description of current state"` -- Save checkpoint before stopping
5. `beu progress` -- Cross-module summary (auto-loaded by hooks)

## Compliance Pipeline

Call `beu check` periodically (after completing tasks, before pausing) to enforce documentation hygiene:

- **Missing docs**: `beu check` fails if a required artifact isn't registered
- **Pending docs**: Fails if a required artifact is still "pending"
- **Stale docs**: Fails if `staleness_threshold` is set and enough mutation events (task adds, state changes, etc.) occurred since the doc was last updated

Fix staleness by updating the doc and recording it:
```bash
beu artifact changelog <name> "updated for <summary>"
```

## Project Scoping

Use `--project <id>` (or `-p <id>`) to scope commands to a specific project within a shared `.beu` database. Without the flag, the `default_project` is used. Set `require_project: true` to force explicit project selection.

## Resources

| Resource | Content |
|----------|---------|
| [CLI_REFERENCE.md](resources/CLI_REFERENCE.md) | Complete command syntax |
| [WORKFLOWS.md](resources/WORKFLOWS.md) | Step-by-step workflow patterns |
| [MODULES.md](resources/MODULES.md) | Module deep dives and usage patterns |
