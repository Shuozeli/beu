# Tool Comparison: bd vs get-shit-done vs beu

Three tools in this repo address different aspects of AI-agent-driven development.

## What Each Tool Solves

| Dimension | **bd** (beads) | **get-shit-done** (GSD) | **beu** |
|---|---|---|---|
| **One-liner** | Distributed, git-backed graph issue tracker for AI agents | Meta-prompting & context-engineering system for Claude Code | Lightweight session memory CLI for agent workflows |
| **Core Problem** | AI agents lack a persistent, structured, dependency-aware task store that survives sessions and supports multi-agent coordination | Context rot -- AI quality degrades as context fills up during long coding sessions | Agents need fast, ephemeral project memory (decisions, debug logs, tasks) without heavyweight infrastructure |
| **Metaphor** | "Jira for AI agents, stored in version-controlled SQL" | "A curriculum that keeps Claude from going off the rails" | "A personal notebook for an AI agent" |

## Tech Stack

| | **bd** | **GSD** | **beu** |
|---|---|---|---|
| **Language** | Go | Node.js (JavaScript) | Rust |
| **Storage** | Dolt (version-controlled SQL, MySQL-compatible) | Flat files in `.planning/` (Markdown, JSON, YAML) | SQLite (one DB per module, WAL mode) |
| **Binary Size** | ~30MB (includes embedded Dolt) | N/A (npm package, installed to `~/.claude/`) | ~5MB |
| **Distribution** | npm, Homebrew, `go install`, binary releases | npm (`get-shit-done-cc`) | `cargo build` (local) |

## Target Users

| | **bd** | **GSD** | **beu** |
|---|---|---|---|
| **Primary** | AI agents + human devs managing task graphs | Solo devs doing AI-assisted (vibe)coding | AI agents managing session state |
| **Scale** | Multi-agent, multi-repo, team workflows | Single developer, single project at a time | Single agent, single project |
| **Collaboration** | Distributed sync via Dolt push/pull | None (local `.planning/` dir) | None (local `.beu/` dir) |

## Core Capabilities

| Capability | **bd** | **GSD** | **beu** |
|---|---|---|---|
| **Issue/Task Tracking** | Full graph: create, list, ready, blocked, dependencies, labels, comments, events | Phase-based roadmap with plans, but no queryable task DB | Lightweight agile module (add, list, update, done, sprint view) |
| **Dependency Management** | First-class: blocks, parent-child, related, conditional-blocks, cycle detection | Implicit via phase ordering + plan wave dependencies | None |
| **Workflow Orchestration** | Formulas, molecules, gates, wisps (chemistry metaphor) | Full lifecycle: new-project -> discuss -> plan -> execute -> verify -> complete | Journal sessions (open/log/close) + pause/resume |
| **Agent Coordination** | Multi-agent safe (hash IDs), routing, gates for async handoffs | Spawns specialized agents (planner, executor, verifier, researcher) with fresh 200K contexts | Single-agent use; export/import for handoff |
| **Persistent Memory** | Structured SQL with version history (Dolt branches, diffs) | `.planning/` directory (PROJECT.md, STATE.md, ROADMAP.md, etc.) | SQLite key-value state module + journal entries |
| **Debug Tracking** | Via issues + comments | `/gsd:debug` command with persistent debug state | Dedicated debug module (open, symptom, cause, resolve) |
| **Context Management** | Compaction (summarize old issues to save tokens) | Core feature: fresh 200K context per executor, thin orchestrators, size-capped docs | Not addressed |
| **Sync/Distribution** | Dolt remotes (push/pull/branch/merge) | None | JSON export/import |
| **CI/External Integration** | Gates (wait for GitHub CI, human approval, timers), Linear/Jira/GitLab import | Git integration (atomic commits per task) | None |
| **Health/Diagnostics** | `bd doctor` (DB validation, orphan detection, migration) | `/gsd:health [--repair]` (.planning/ integrity) | `beu health [--repair]` (.beu/ integrity) |

## Architecture Philosophy

| | **bd** | **GSD** | **beu** |
|---|---|---|---|
| **Design** | Heavy, feature-rich, distributed-first | Opinionated workflow engine, context-engineering-first | Minimal, fast, module-per-concern |
| **Data Model** | Relational graph (issues, deps, events in SQL) | Document-oriented (Markdown files with YAML frontmatter) | Per-module SQLite databases (journal.db, agile.db, state.db, etc.) |
| **Extensibility** | Go API (`beads.go`), SQL queries, formula templates | 42+ slash commands, 12 agent personas, reference docs | 6 hardcoded modules, JSON ABI |
| **Startup Time** | Slower (Dolt engine init) | N/A (Claude command expansion) | Instant (native binary, no runtime) |
| **Complexity** | High (70+ commands, 31 internal packages) | Medium (42 commands, but they're markdown prompts, not code) | Low (6 modules, ~1300 LOC main.rs) |

## Key Differentiators

### bd (beads)

- Only one with **true distributed sync** (Dolt = git for databases)
- **Dependency graph** with `ready` queue -- agents know exactly what's unblocked
- **Multi-agent safe** by design (hash-based IDs, routing, gates)
- **Version-controlled data** -- branch, diff, and merge the issue database
- Best for: **long-horizon, multi-agent projects** with complex task dependencies

### GSD (get-shit-done)

- Only one that **actively manages Claude's context window** (fresh 200K per executor)
- **Spec-driven development** with full lifecycle (research -> plan -> execute -> verify)
- **Parallel execution with wave-based scheduling** of independent plans
- **Atomic commits per task** with deviation rules and verification
- Best for: **keeping Claude reliable** across multi-phase feature builds

### beu

- Only one optimized for **instant startup** and **zero overhead** (~5MB, no runtime)
- **Session-oriented** (journal open/close, pause/resume, debug investigations)
- **Structured personal memory** (key-value state, todos, artifact tracking)
- **Full audit trail** via event logging across all modules
- Best for: **fast, lightweight agent memory** during a single coding session

## Complementary Use

All three can work together:

1. **bd** for the project's authoritative issue graph (what needs doing)
2. **GSD** for orchestrating Claude through multi-phase execution (how to do it reliably)
3. **beu** for ephemeral session state during execution (what's happening right now)

### Overlap

All three track tasks/work items, but differently:

- `bd` tracks them as a **queryable dependency graph** with sync
- `GSD` tracks them as **phased plans** optimized for Claude's context window
- `beu` tracks them as **lightweight local records** for fast agent reference
