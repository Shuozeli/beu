# Bulds Design Document

**Status:** Draft
**Author:** cyuan
**Date:** 2026-02-24

---

## Table of Contents

1. [Problem Statement](#1-problem-statement)
2. [Design Principles](#2-design-principles)
3. [System Architecture](#3-system-architecture)
4. [The Composable Core](#4-the-composable-core)
5. [Information Architecture: The Three Pillars](#5-information-architecture-the-three-pillars)
6. [The CLI Mesh](#6-the-cli-mesh)
7. [Data Model](#7-data-model)
8. [Plugin System](#8-plugin-system)
9. [Pipeline Engine](#9-pipeline-engine)
10. [AI Orchestrator](#10-ai-orchestrator)
11. [Tech Stack](#11-tech-stack)
12. [Security Model](#12-security-model)
13. [Phased Rollout](#13-phased-rollout)
14. [Open Questions](#14-open-questions)

---

## 1. Problem Statement

Tools like `beads` provide structured issue tracking and agent coordination,
but they are fundamentally **monolithic**: every capability is compiled into a
single binary, every workflow is a hardcoded command, and every integration
requires modifying the core. When your workflow evolves -- a new tool enters
the stack, a new AI model arrives, a new documentation format emerges -- the
rigid pipeline breaks.

**Bulds** addresses this by shifting from a monolithic CLI to a **pluggable
orchestrator with a CLI Mesh**, transforming the terminal from a collection
of isolated tools into a **contextual AI operating system**.

### What Beads Gets Right

- Hash-based IDs for conflict-free distributed work
- Transaction-wrapped storage for data integrity
- Agent-optimized output (JSON, dependency graphs)
- Git-backed versioning via Dolt

### Where Beads Breaks Down

| Limitation | Impact |
|---|---|
| Hardcoded commands | Adding a capability requires modifying and recompiling the core binary |
| Rigid pipelines | Cannot compose `journal -> architecture-doc` without custom code |
| No cross-tool awareness | Each CLI invocation is stateless; no shared context between tools |
| Monolithic AI integration | AI is bolted on, not a first-class orchestration layer |
| Single storage backend | Dolt is powerful but not every plugin needs SQL semantics |

### Target User

A developer or engineering team that:
- Uses 10+ CLI tools daily (git, docker, kubectl, terraform, npm, etc.)
- Maintains architecture docs, journals, and issue trackers across projects
- Wants AI assistance that understands their full terminal context
- Needs composable workflows that evolve without recompilation

---

## 2. Design Principles

### P1: Microkernel, Not Monolith

The core binary knows how to do three things: **parse intent**, **load
plugins**, and **manage execution state**. Everything else is a plugin.

### P2: Pipelines Are Data, Not Code

Workflows are declared in composable configurations (YAML/TOML), not compiled
into the binary. A user can pipe the output of the `journal` plugin into the
`architecture-doc` plugin by editing a config file.

### P3: Context Is a First-Class Citizen

The CLI Mesh maintains a shared context store accessible to all plugins and
external tools. A plugin does not need to re-derive state that another plugin
has already computed.

### P4: AI as Router, Not Executor

The AI layer does not execute domain logic. It builds a DAG of plugin
invocations based on the user's intent, then hands execution to the pipeline
engine. The AI is the planner; the plugins are the workers.

### P5: Fail Fast, Recover Gracefully

Every plugin declares its input/output contracts. The pipeline engine validates
the DAG before execution. If a plugin fails, the engine surfaces the error
immediately with the full execution context, not a generic stack trace.

### P6: Human-Readable by Default

All persistent state is stored in formats a human can read and edit directly:
Markdown files for documents, YAML for configuration, SQLite for structured
data. No opaque binary blobs.

---

## 3. System Architecture

```
+------------------------------------------------------------------+
|                         User / Agent                             |
|  "bulds journal add ..." / "bulds run my-pipeline" / NL input   |
+----------------------------------+-------------------------------+
                                   |
                                   v
+------------------------------------------------------------------+
|                      Intent Parser                               |
|  Structured command dispatch  |  NL -> DAG via AI Orchestrator   |
+----------------------------------+-------------------------------+
                                   |
                                   v
+------------------------------------------------------------------+
|                      Pipeline Engine                             |
|  DAG construction  |  Dependency resolution  |  Execution loop   |
+--------+-------------------+-------------------+-----------------+
         |                   |                   |
         v                   v                   v
+----------------+  +----------------+  +------------------+
|  Plugin: Docs  |  | Plugin: Issues |  | Plugin: Journal  |
+----------------+  +----------------+  +------------------+
|  Plugin: Git   |  | Plugin: Search |  | Plugin: Export   |
+----------------+  +----------------+  +------------------+
|  Plugin: K8s   |  | Plugin: Jira   |  | Plugin: Custom   |
+----------------+  +----------------+  +------------------+
         |                   |                   |
         v                   v                   v
+------------------------------------------------------------------+
|                      CLI Mesh Layer                               |
|  Context Store  |  Command Interception  |  Observability        |
+------------------------------------------------------------------+
         |                   |                   |
         v                   v                   v
+------------------------------------------------------------------+
|                      Storage Layer                                |
|  SQLite (structured)  |  Vector DB (semantic)  |  Markdown (docs)|
+------------------------------------------------------------------+
```

### Component Responsibilities

| Component | Responsibility |
|---|---|
| **Intent Parser** | Dispatches structured commands directly; routes NL input through the AI Orchestrator |
| **AI Orchestrator** | Translates natural language into a DAG of plugin calls using function-calling LLMs |
| **Pipeline Engine** | Validates, schedules, and executes plugin DAGs with dependency awareness |
| **Plugin Registry** | Discovers, loads, validates, and manages the lifecycle of plugins |
| **CLI Mesh** | Maintains cross-tool context, intercepts commands, provides observability |
| **Storage Layer** | Unified data access across SQLite, vector DB, and filesystem |

---

## 4. The Composable Core

### 4.1 The Microkernel

The `bulds` binary is a thin shell:

```
bulds (binary)
  |
  +-- intent_parser     # CLI arg parsing + NL routing
  +-- plugin_registry   # Discovery, loading, lifecycle
  +-- pipeline_engine   # DAG execution
  +-- mesh_runtime      # CLI Mesh context + interception
  +-- storage_broker    # Unified storage access
```

The binary does **not** contain any domain logic. No issue tracking, no
journal management, no documentation generation. All of that lives in plugins.

### 4.2 Command Routing

When the user types a command, the intent parser follows this decision tree:

```
Input: "bulds journal add 'Fixed auth bug in login service'"
                |
                v
        Is this a registered command?
               / \
             yes   no
              |     |
              v     v
        Dispatch   Route to AI Orchestrator
        directly   for NL intent parsing
              |     |
              v     v
        Plugin      Build DAG of plugin calls
        executes    from AI function-calling
              |     |
              v     v
        Return      Execute DAG via Pipeline Engine
        result      Return aggregated result
```

Structured commands (e.g., `bulds journal add`) dispatch directly to the
registered plugin handler. Natural language commands (e.g., `bulds "summarize
what I did today"`) go through the AI Orchestrator, which decomposes the
intent into a sequence of plugin calls.

### 4.3 Plugin Lifecycle

```
Discovery -> Validation -> Loading -> Initialization -> Ready -> Shutdown
```

1. **Discovery:** Scan plugin directories (`~/.bulds/plugins/`, `./bulds-plugins/`)
2. **Validation:** Check manifest schema, verify required fields, validate WASM signature
3. **Loading:** Load the plugin binary (WASM module or subprocess)
4. **Initialization:** Call the plugin's `init()` with the host environment
5. **Ready:** Plugin registers its commands, input/output schemas, and capabilities
6. **Shutdown:** Graceful teardown on `bulds` exit or plugin hot-reload

---

## 5. Information Architecture: The Three Pillars

Bulds organizes all project knowledge into three pillars. Each pillar has a
dedicated storage strategy, a set of plugins that operate on it, and AI
capabilities that connect them.

### 5.1 Pillar 1: Documentation

**What it stores:** Architecture docs, design docs, codelabs, ADRs, READMEs.

**Storage:** Markdown files on disk, indexed in SQLite for metadata and
cross-references, embedded in a vector DB for semantic search.

**Plugin capabilities:**
- `docs.watch` -- monitor directories for doc changes, auto-index
- `docs.toc` -- generate/update table of contents
- `docs.xref` -- extract and maintain cross-references between docs
- `docs.adr` -- architecture decision record management
- `docs.search` -- full-text and semantic search across all docs

**AI capabilities:**
- Auto-generate a TOC when a doc exceeds a configurable length
- Extract architectural decisions from prose and register them in an ADR log
- Cross-reference codelabs with the design docs they implement
- Detect stale docs by comparing against recent code changes

### 5.2 Pillar 2: Action Items

**What it stores:** Issues, TODOs, future improvements, blockers.

**Sources:**
- Explicit CLI commands (`bulds issue create "..."`)
- Code comments (`// TODO:`, `// FIXME:`, `// HACK:`)
- Journal entries containing action language ("need to", "should", "must")
- External imports (Jira, Linear, GitHub Issues)

**Storage:** SQLite with full relational model (issues, dependencies, labels,
comments). Hash-based IDs for conflict-free distributed work (carried over
from beads).

**Plugin capabilities:**
- `issues.create`, `issues.list`, `issues.update`, `issues.close`
- `issues.scan` -- scan codebase for TODO/FIXME comments, reconcile with DB
- `issues.dep` -- dependency graph management
- `issues.import` -- import from external trackers
- `issues.ready` -- list unblocked work

**AI capabilities:**
- Categorize items by urgency and domain automatically
- Deduplicate similar issues across sources (code TODOs vs explicit issues)
- Link issues to relevant design docs bidirectionally
- Suggest priority based on dependency graph position and staleness

### 5.3 Pillar 3: The Journal

**What it stores:** Daily log of actions, thoughts, decisions, and command
history.

**Storage:** Append-only Markdown files (one per day, `YYYY-MM-DD.md`),
indexed in SQLite for querying, embedded in vector DB for semantic retrieval.

**Plugin capabilities:**
- `journal.add` -- append an entry with timestamp
- `journal.today` -- show today's entries
- `journal.search` -- search across all journal entries
- `journal.summary` -- AI-generated daily/weekly summary
- `journal.auto` -- auto-log intercepted CLI commands (via Mesh)

**AI capabilities:**
- Summarize daily progress into a stand-up format
- Flag blocked tasks mentioned in journal entries
- Connect journal entries to architecture goals and open issues
- Detect patterns (e.g., "you've mentioned auth concerns 5 times this week")

### 5.4 Cross-Pillar Connections

The three pillars are not silos. The AI layer maintains a connection graph:

```
 Documentation <-------> Action Items
      |    \                /    |
      |     \              /     |
      |      \            /      |
      |       v          v       |
      +------> Journal <---------+
```

- A journal entry mentioning "refactored the auth module" auto-links to the
  auth architecture doc and closes related TODO items.
- An issue tagged `design-needed` triggers a prompt to create a design doc.
- A design doc change triggers a scan for affected issues and journal entries.

---

## 6. The CLI Mesh

The CLI Mesh is a layer that sits between the user's terminal and the
underlying tools, providing context sharing, command interception, agent
swarming, and observability.

### 6.1 Context Store

A shared, typed key-value store accessible by all plugins and external tools.

**Scope hierarchy:**
```
Global Context (user-level, ~/.bulds/context/)
  |
  +-- Project Context (project-level, .bulds/context/)
        |
        +-- Session Context (terminal session, ephemeral)
              |
              +-- Pipeline Context (single pipeline run, ephemeral)
```

**Examples of context entries:**
- `git.branch` = `feature/auth-refactor` (auto-populated)
- `git.dirty_files` = `["src/auth.ts", "src/login.ts"]` (auto-populated)
- `k8s.namespace` = `staging` (from last `kubectl` command)
- `docker.running` = `["postgres:15", "redis:7"]` (auto-populated)
- `project.language` = `typescript` (from project detection)
- `session.last_error` = `{ cmd: "npm test", exit: 1, stderr: "..." }`

**Access patterns:**
```
# Plugin reads context
ctx, err := mesh.Context.Get("git.branch")

# Plugin writes context
mesh.Context.Set("my-plugin.last-result", result, Scope.Session)

# External tool reads context (via Unix socket or env var)
BULDS_CTX=$(bulds mesh get git.branch)
```

### 6.2 Command Interception

The Mesh can wrap existing CLI tools to automatically capture their
input/output into the journal and context store.

**Mechanism:** Shell function wrappers injected via `eval "$(bulds mesh shell-init)"`.

**What gets intercepted:**
- `git commit` -> journal entry: "Committed: <message>" + context update
- `git checkout` -> context update: `git.branch`
- `docker compose up` -> context update: `docker.running`
- `kubectl apply` -> context update: `k8s.last_apply`
- `npm test` / `pytest` -> context update: `session.last_test_result`

**Configuration (`.bulds/mesh.yaml`):**
```yaml
intercept:
  git:
    commands: [commit, checkout, push, pull, merge]
    journal: true      # log to journal
    context: true      # update context store
  docker:
    commands: [compose, run, build]
    journal: false
    context: true
  kubectl:
    commands: [apply, delete, rollout]
    journal: true
    context: true
  custom:
    - pattern: "terraform *"
      journal: true
      context: true
      context_key: "terraform.last_action"
```

**Privacy controls:**
- Interception is opt-in per tool and per command
- Sensitive commands (e.g., `kubectl exec`) can be excluded
- Environment variables containing secrets are never captured
- Users can disable interception entirely with `BULDS_MESH=off`

### 6.3 Agent Swarming

Multiple specialized AI agents can communicate through the Mesh to solve
complex, multi-domain tasks.

**Agent types:**
- **Frontend Agent:** Understands React/Vue/Angular, CSS, accessibility
- **Backend Agent:** Understands APIs, databases, authentication
- **Infra Agent:** Understands Docker, K8s, Terraform, CI/CD
- **Docs Agent:** Understands documentation standards, generates docs

**Swarming protocol:**

```
User: "deploy the auth feature"
          |
          v
    AI Orchestrator builds DAG:
      1. Backend Agent: verify API tests pass
      2. Frontend Agent: verify UI tests pass
      3. Infra Agent: deploy to staging
      4. Docs Agent: update deployment log
          |
          v
    Agents communicate via Mesh:
      - Backend Agent sets: mesh.agent.backend.status = "tests_passed"
      - Frontend Agent reads backend status, proceeds
      - Infra Agent reads both statuses, deploys
      - Docs Agent reads deployment result, updates journal
```

**Coordination primitives (via Mesh):**
- `mesh.agent.signal(agent_id, event)` -- signal between agents
- `mesh.agent.wait(agent_id, event, timeout)` -- wait for agent signal
- `mesh.agent.broadcast(event)` -- broadcast to all active agents
- `mesh.agent.claim(resource)` -- exclusive access to a resource

### 6.4 Observability

The Mesh tracks execution metrics for all plugin and pipeline runs.

**Metrics collected:**
- Plugin execution time (p50, p95, p99)
- Plugin success/failure rates
- Pipeline stage durations
- Context store read/write frequency
- AI token usage per orchestration

**Storage:** Append-only SQLite table (`mesh_events`).

**Queries:**
```
bulds mesh stats                    # summary dashboard
bulds mesh stats --plugin journal   # per-plugin breakdown
bulds mesh slow                     # slowest pipelines this week
bulds mesh errors                   # recent failures with context
```

**AI-driven optimization:**
- "Your `docs.xref` plugin takes 4x longer than average. The bottleneck is
  the full-text search step. Consider enabling the vector index."
- "You run `git status` -> `git diff` -> `git add` 12 times per day.
  Consider creating a pipeline for this."

---

## 7. Data Model

### 7.1 Core Schema (SQLite)

```sql
-- Plugin registry
CREATE TABLE plugins (
    id          TEXT PRIMARY KEY,     -- e.g., "bulds-journal"
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    manifest    TEXT NOT NULL,        -- JSON manifest
    path        TEXT NOT NULL,        -- filesystem path to plugin
    status      TEXT NOT NULL,        -- 'active', 'disabled', 'error'
    loaded_at   TEXT NOT NULL,
    CHECK (status IN ('active', 'disabled', 'error'))
);

-- Issues (Pillar 2)
CREATE TABLE issues (
    id              TEXT PRIMARY KEY,    -- hash-based, e.g., "bl-a1b2c3"
    title           TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 3,
    assignee        TEXT,
    source          TEXT NOT NULL,       -- 'cli', 'code_scan', 'journal', 'import'
    source_ref      TEXT,               -- file:line for code_scan, entry_id for journal
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    closed_at       TEXT,
    CHECK (status IN ('open', 'in_progress', 'blocked', 'deferred', 'closed')),
    CHECK (priority BETWEEN 0 AND 5)
);

-- Issue dependencies
CREATE TABLE issue_deps (
    child_id    TEXT NOT NULL REFERENCES issues(id),
    parent_id   TEXT NOT NULL REFERENCES issues(id),
    dep_type    TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    PRIMARY KEY (child_id, parent_id),
    CHECK (dep_type IN ('blocks', 'related', 'parent_child', 'duplicates'))
);

-- Journal entries (Pillar 3)
CREATE TABLE journal_entries (
    id          TEXT PRIMARY KEY,
    content     TEXT NOT NULL,
    source      TEXT NOT NULL,       -- 'manual', 'mesh_intercept', 'ai_summary'
    tags        TEXT,                -- JSON array
    created_at  TEXT NOT NULL
);

-- Documents index (Pillar 1)
CREATE TABLE documents (
    id          TEXT PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE, -- relative filesystem path
    title       TEXT NOT NULL,
    doc_type    TEXT NOT NULL,        -- 'design', 'architecture', 'adr', 'codelab', 'readme'
    checksum    TEXT NOT NULL,        -- SHA256 of file content
    indexed_at  TEXT NOT NULL,
    CHECK (doc_type IN ('design', 'architecture', 'adr', 'codelab', 'readme', 'other'))
);

-- Cross-references between pillars
CREATE TABLE cross_refs (
    id          TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,        -- 'issue', 'journal', 'document'
    source_id   TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id   TEXT NOT NULL,
    ref_type    TEXT NOT NULL,        -- 'mentions', 'implements', 'obsoletes', 'related'
    created_at  TEXT NOT NULL,
    created_by  TEXT NOT NULL,        -- 'user', 'ai', 'plugin:<name>'
    CHECK (source_type IN ('issue', 'journal', 'document')),
    CHECK (target_type IN ('issue', 'journal', 'document')),
    CHECK (ref_type IN ('mentions', 'implements', 'obsoletes', 'related'))
);

-- CLI Mesh context store
CREATE TABLE mesh_context (
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,        -- JSON-encoded
    scope       TEXT NOT NULL,        -- 'global', 'project', 'session', 'pipeline'
    scope_id    TEXT,                 -- session ID or pipeline run ID
    updated_at  TEXT NOT NULL,
    PRIMARY KEY (key, scope, scope_id)
);

-- Mesh observability events
CREATE TABLE mesh_events (
    id          TEXT PRIMARY KEY,
    event_type  TEXT NOT NULL,        -- 'plugin_exec', 'pipeline_run', 'mesh_intercept', 'error'
    plugin_id   TEXT,
    pipeline_id TEXT,
    duration_ms INTEGER,
    status      TEXT NOT NULL,        -- 'success', 'failure', 'timeout'
    metadata    TEXT,                 -- JSON
    created_at  TEXT NOT NULL,
    CHECK (event_type IN ('plugin_exec', 'pipeline_run', 'mesh_intercept', 'error')),
    CHECK (status IN ('success', 'failure', 'timeout'))
);

-- Pipeline definitions
CREATE TABLE pipelines (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    definition  TEXT NOT NULL,        -- YAML pipeline definition
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Event log (system-wide debugging)
CREATE TABLE event_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   TEXT NOT NULL,
    level       TEXT NOT NULL,        -- 'debug', 'info', 'warn', 'error'
    component   TEXT NOT NULL,        -- 'core', 'plugin:<id>', 'mesh', 'pipeline', 'ai'
    event       TEXT NOT NULL,
    details     TEXT,                 -- JSON
    CHECK (level IN ('debug', 'info', 'warn', 'error'))
);
```

### 7.2 Vector Store Schema (LanceDB)

```
Collection: documents
  - id: string (matches documents.id)
  - embedding: vector[1536]
  - content_chunk: string
  - chunk_index: integer
  - source_path: string

Collection: journal_entries
  - id: string (matches journal_entries.id)
  - embedding: vector[1536]
  - content: string
  - created_at: string

Collection: issues
  - id: string (matches issues.id)
  - embedding: vector[1536]
  - content: string (title + description concatenated)
```

### 7.3 Filesystem Layout

```
~/.bulds/                           # Global config
  config.yaml                       # Global settings
  plugins/                          # User-installed plugins
  context/                          # Global context store

.bulds/                             # Project-level
  config.yaml                       # Project settings
  bulds.db                          # SQLite database
  vectors/                          # LanceDB directory
  context/                          # Project context
  plugins/                          # Project-local plugins
  pipelines/                        # Pipeline definitions
  mesh.yaml                         # Mesh configuration
  journal/                          # Journal markdown files
    2026-02-24.md
    2026-02-25.md
  hooks/                            # Lifecycle hooks
```

---

## 8. Plugin System

### 8.1 Plugin Manifest

Every plugin declares its capabilities via a `plugin.yaml` manifest:

```yaml
name: bulds-journal
version: "0.1.0"
description: "Append-only journal with AI summarization"
author: "cyuan"

# Runtime
runtime: wasm                        # 'wasm', 'subprocess', 'native'
entrypoint: journal.wasm             # or "./journal-plugin" for subprocess

# Commands this plugin registers
commands:
  - name: journal.add
    description: "Add a journal entry"
    args:
      - name: content
        type: string
        required: true
      - name: tags
        type: "list[string]"
        required: false
    output:
      type: journal_entry

  - name: journal.today
    description: "Show today's journal entries"
    output:
      type: "list[journal_entry]"

  - name: journal.summary
    description: "AI-generated summary of recent entries"
    args:
      - name: range
        type: string
        required: false
        default: "today"
    output:
      type: string

# Data types this plugin defines
types:
  journal_entry:
    fields:
      id: string
      content: string
      tags: "list[string]"
      source: string
      created_at: string

# Capabilities this plugin requires from the host
requires:
  - storage.sqlite                   # read/write SQLite
  - storage.filesystem               # read/write files
  - mesh.context.read                # read mesh context
  - mesh.context.write               # write mesh context
  - ai.embed                         # generate embeddings
  - ai.complete                      # LLM completion

# Events this plugin emits
emits:
  - journal.entry_added
  - journal.summary_generated

# Events this plugin listens to
listens:
  - mesh.command_intercepted         # auto-log intercepted commands
```

### 8.2 Host-Plugin Interface

The host exposes a set of functions to plugins via the WASM host interface:

```
Host Functions (available to plugins):
  storage_query(sql: string) -> Result<JSON>
  storage_exec(sql: string) -> Result<void>
  fs_read(path: string) -> Result<bytes>
  fs_write(path: string, data: bytes) -> Result<void>
  fs_list(pattern: string) -> Result<list[string]>
  context_get(key: string) -> Result<JSON>
  context_set(key: string, value: JSON, scope: Scope) -> Result<void>
  ai_embed(text: string) -> Result<vector>
  ai_complete(prompt: string, options: JSON) -> Result<string>
  event_emit(event: string, payload: JSON) -> Result<void>
  log(level: string, message: string) -> void

Plugin Functions (called by host):
  init(config: JSON) -> Result<void>
  handle_command(name: string, args: JSON) -> Result<JSON>
  handle_event(event: string, payload: JSON) -> Result<void>
  shutdown() -> void
```

### 8.3 Plugin Security

Plugins run in sandboxed WASM environments with capability-based security:

- A plugin can only access host functions listed in its `requires` section
- File system access is scoped to the project's `.bulds/` directory by default
- Network access requires an explicit `network` capability declaration
- The user approves capability grants on first plugin load

### 8.4 Plugin Development Workflow

```bash
# Scaffold a new plugin
bulds plugin new my-plugin --lang rust

# Build the plugin
cd bulds-plugins/my-plugin
bulds plugin build

# Test locally
bulds plugin test

# Install into current project
bulds plugin install ./target/my-plugin.wasm

# Publish to registry (future)
bulds plugin publish
```

---

## 9. Pipeline Engine

### 9.1 Pipeline Definition

Pipelines are YAML files that declare a DAG of plugin invocations:

```yaml
name: daily-standup
description: "Generate a daily standup from journal and issues"
trigger: manual

steps:
  - id: fetch-journal
    plugin: journal.today
    output: today_entries

  - id: fetch-ready
    plugin: issues.ready
    output: ready_issues

  - id: fetch-blocked
    plugin: issues.list
    args:
      status: blocked
    output: blocked_issues

  - id: generate-standup
    plugin: ai.complete
    depends_on: [fetch-journal, fetch-ready, fetch-blocked]
    args:
      prompt: |
        Generate a standup summary from:
        Journal: {{ today_entries }}
        Ready work: {{ ready_issues }}
        Blocked: {{ blocked_issues }}
      format: markdown
    output: standup

  - id: save-standup
    plugin: docs.write
    depends_on: [generate-standup]
    args:
      path: "standups/{{ date }}.md"
      content: "{{ standup }}"
```

### 9.2 DAG Execution

```
fetch-journal ----+
                  |
fetch-ready ------+--> generate-standup --> save-standup
                  |
fetch-blocked ----+
```

**Execution rules:**
- Steps with no `depends_on` run in parallel
- A step only runs after all its dependencies have completed successfully
- If a step fails, all downstream steps are skipped
- The engine reports which steps succeeded, failed, and were skipped
- Step outputs are type-checked against the plugin's declared output schema

### 9.3 Pipeline Composition

Pipelines can invoke other pipelines as steps:

```yaml
name: weekly-report
steps:
  - id: monday
    pipeline: daily-standup
    args:
      date: "{{ week_start }}"

  - id: summarize
    plugin: ai.complete
    depends_on: [monday, tuesday, wednesday, thursday, friday]
    args:
      prompt: "Summarize the week..."
```

### 9.4 Conditional Execution

```yaml
steps:
  - id: check-tests
    plugin: shell.exec
    args:
      command: "npm test"

  - id: deploy
    plugin: infra.deploy
    depends_on: [check-tests]
    when: "{{ check-tests.exit_code == 0 }}"

  - id: notify-failure
    plugin: notify.send
    depends_on: [check-tests]
    when: "{{ check-tests.exit_code != 0 }}"
    args:
      message: "Tests failed: {{ check-tests.stderr }}"
```

---

## 10. AI Orchestrator

### 10.1 Role

The AI Orchestrator is the bridge between natural language input and the
pipeline engine. It uses function-calling LLMs to decompose user intent
into structured plugin invocations.

### 10.2 Function-Calling Schema

The orchestrator registers all loaded plugin commands as callable functions
with the LLM:

```json
{
  "name": "journal.add",
  "description": "Add a journal entry",
  "parameters": {
    "type": "object",
    "properties": {
      "content": { "type": "string", "description": "The journal entry text" },
      "tags": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["content"]
  }
}
```

### 10.3 Orchestration Flow

```
User: "I just finished the auth refactor, update everything"
          |
          v
    AI Orchestrator receives:
      - User input
      - Current mesh context (git.branch, git.dirty_files, etc.)
      - Available plugin functions
          |
          v
    LLM generates function calls:
      1. journal.add(content="Completed auth refactor", tags=["auth", "refactor"])
      2. issues.list(search="auth refactor", status="open")
      3. issues.close(id=<from step 2>)
      4. docs.xref(trigger="auth")
          |
          v
    Pipeline engine executes the DAG
          |
          v
    Results aggregated and presented to user
```

### 10.4 AI Provider Configuration

```yaml
# .bulds/config.yaml
ai:
  provider: google                   # 'google', 'anthropic', 'openai', 'local'
  model: gemini-2.0-flash
  embed_model: text-embedding-004
  max_tokens: 4096
  temperature: 0.1
  api_key_env: GOOGLE_GENAI_API_KEY  # environment variable name, no default
```

The AI provider is itself a plugin, allowing users to swap providers without
modifying the core.

---

## 11. Tech Stack

| Component | Technology | Rationale |
|---|---|---|
| **Core binary** | Go | Fast startup, easy cross-compilation, strong concurrency, matches beads ecosystem |
| **Plugin runtime** | Extism (WASM) | Language-agnostic plugins, sandboxed execution, mature Go SDK |
| **Structured storage** | SQLite (ncruces/go-sqlite3) | Zero-config, embedded, transaction support, proven at scale |
| **Vector storage** | LanceDB | Embedded, no server needed, Go bindings, columnar format |
| **AI integration** | `@google/genai` via REST | Per project rules; provider abstracted behind plugin interface |
| **CLI framework** | Cobra + Viper | Industry standard for Go CLIs, matches beads patterns |
| **Config format** | YAML | Human-readable, widely understood, good Go support |
| **Docs format** | Markdown | Universal, version-control friendly, zero tooling required |
| **IPC (Mesh)** | Unix domain socket | Fast, no network overhead, standard on Linux/macOS |
| **Build** | Go toolchain + Makefile | Simple, reproducible, no external build system |
| **Testing** | Go testing + testutil | Standard library, no test framework dependency |

### Build Dependencies

- Go 1.25+
- CGO enabled (for SQLite)
- Extism SDK (`github.com/extism/go-sdk`)
- No Docker required for development

---

## 12. Security Model

### 12.1 Plugin Sandboxing

- WASM plugins run in a sandboxed VM with no direct OS access
- All host interactions go through the declared capability interface
- File system access is jailed to `.bulds/` unless explicitly granted
- Network access is disabled by default; requires `network` capability
- CPU/memory limits enforced per plugin invocation

### 12.2 Credential Handling

- API keys are read from environment variables, never stored in config files
- The mesh context store never captures environment variables matching
  `*KEY*`, `*SECRET*`, `*TOKEN*`, `*PASSWORD*` patterns
- Plugin manifests declare which env vars they need; the host validates
  before passing them through

### 12.3 Command Interception Privacy

- Interception is opt-in per tool and per command
- Intercepted command arguments are sanitized (secrets redacted)
- Users can audit all intercepted data via `bulds mesh audit`
- `BULDS_MESH=off` disables all interception

---

## 13. Phased Rollout

### Phase 0: Foundation (Dark Launch)

**Goal:** Core binary + SQLite storage + one built-in plugin (journal).

**Deliverables:**
- `bulds init` -- initialize project database
- `bulds journal add/today/search` -- basic journal operations
- SQLite schema creation and transaction management
- Config file loading (YAML + env vars, fail-fast on missing)
- Event log infrastructure

**Validation:** Run against 3-4 personal projects. Verify journal workflow
handles daily use for one week.

### Phase 1: Plugin System (1% Launch)

**Goal:** Extract journal into a WASM plugin. Build plugin lifecycle.

**Deliverables:**
- Plugin manifest schema and validation
- WASM host interface (storage, filesystem, context, log)
- Plugin discovery and loading
- `bulds plugin install/list/remove`
- Second plugin: `issues` (basic CRUD, no dependencies yet)

**Validation:** Both journal and issues plugins work identically to Phase 0
built-in behavior. Measure plugin call overhead (target: <5ms).

### Phase 2: Pipeline Engine

**Goal:** YAML-defined pipelines with DAG execution.

**Deliverables:**
- Pipeline YAML parser and validator
- DAG construction and dependency resolution
- Parallel execution of independent steps
- Template variable interpolation
- `bulds run <pipeline>` command
- First pipeline: `daily-standup` (journal + issues -> summary)

**Validation:** `daily-standup` pipeline produces useful output. Pipeline
failure handling works correctly (skip downstream on error).

### Phase 3: AI Orchestrator

**Goal:** Natural language -> DAG translation via function-calling LLM.

**Deliverables:**
- Function schema generation from plugin manifests
- AI provider plugin interface
- Google GenAI integration (default provider)
- NL intent parsing and DAG construction
- `bulds "natural language command"` support

**Validation:** 10 common NL commands correctly decompose into plugin DAGs.
Measure latency (target: <2s for DAG construction).

### Phase 4: CLI Mesh

**Goal:** Context sharing and command interception.

**Deliverables:**
- Context store (SQLite-backed, scoped)
- Shell init script for command interception
- `git` and `docker` interceptors
- `bulds mesh stats` observability
- Unix socket IPC for external tool integration

**Validation:** Context auto-populates from git/docker commands. Journal
auto-logs git commits. Mesh overhead is imperceptible (<10ms per command).

### Phase 5: Information Architecture

**Goal:** Three pillars fully connected.

**Deliverables:**
- Documentation plugin (watch, index, TOC, cross-ref)
- Vector DB integration (LanceDB) for semantic search
- Cross-reference engine (issue <-> doc <-> journal)
- Code scanning for TODOs
- AI-powered deduplication and linking

**Validation:** Cross-references are accurate. Semantic search returns
relevant results. TODO scan matches manual inspection.

### Phase 6: Agent Swarming

**Goal:** Multi-agent coordination through the Mesh.

**Deliverables:**
- Agent registration and discovery
- Signal/wait/broadcast coordination primitives
- Agent-specific context namespaces
- Swarming pipeline support
- Domain-specific agent templates (frontend, backend, infra)

**Validation:** A multi-agent pipeline (e.g., test -> deploy -> document)
coordinates correctly across 3 agents.

---

## 14. Open Questions

| # | Question | Options | Impact |
|---|---|---|---|
| 1 | Should the vector DB be optional or required? | Optional (degrade gracefully) vs Required (simpler code) | Phase 5 complexity |
| 2 | WASM vs subprocess plugins for v1? | WASM (secure, portable) vs Subprocess (simpler, faster iteration) | Phase 1 timeline |
| 3 | Should the Mesh daemon run persistently or on-demand? | Persistent (faster context) vs On-demand (simpler lifecycle) | Phase 4 architecture |
| 4 | How to handle plugin version conflicts? | Semver lockfile vs Allow multiple versions | Plugin ecosystem complexity |
| 5 | Should journal auto-logging require explicit opt-in per session? | Per-session opt-in vs Project-level config | Privacy vs convenience |
| 6 | SQLite WAL mode for concurrent plugin access? | WAL (concurrent reads) vs Default (simpler) | Multi-plugin performance |
| 7 | Should pipelines support fan-out (one step -> multiple parallel branches)? | Yes (more expressive) vs No (simpler DAG model) | Phase 2 scope |
| 8 | Local-only or support remote sync from day one? | Local-only v1, sync later vs Build sync into storage layer | Phase 0 scope |
