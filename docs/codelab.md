# beu Codelab

A hands-on walkthrough of `beu` for agent-driven development workflows. This
codelab walks through a realistic scenario: using beu to manage a feature
implementation from start to finish.

Each section explains not just *how* to use a command, but *why* it exists and
what problem it solves for agents working across sessions.

## Prerequisites

- Rust toolchain (1.70+)
- `cargo build` completes successfully

## Step 0: Build beu

```bash
cargo build --release

# Verify the binary works.
./target/release/beu version
# Output: beu 0.1.0
```

For the rest of this codelab, we assume `beu` is on your PATH or you use the
full path to the binary.

## Step 1: Initialize a Project

```bash
mkdir -p /tmp/beu-demo && cd /tmp/beu-demo
beu init
```

Expected output:

```
Initialized .beu at /tmp/beu-demo/.beu
  data/     - per-module SQLite databases
  project:  default

Modules: journal, artifact, task, state, idea, debug
```

This creates `.beu/data/beu.db` (a single SQLite database for all modules)
and registers a "default" project. All commands operate within this project
unless you use `--project`.

---

## The Journal: What Happened and Why

**Problem**: AI agents lose context between sessions. A conversation ends, and
the next agent has no idea what was explored, what was decided, or what blocked
progress. Without a persistent record, work gets repeated and decisions get
revisited.

**Solution**: The journal creates a time-ordered, session-scoped record of
events, decisions, and blockers. Each session is a bounded unit of work. When
the next agent resumes, it can read the journal to understand the full history.

### Step 2: Start a Journal Session

```bash
beu journal open
# Output: Session j-a1b2c3d4e5f6a7b8 opened.
```

The session ID is auto-generated. `beu` tracks the currently open session
automatically -- you never need to pass the session ID.

### Step 3: Log Work as You Go

Record observations and progress as you work:

```bash
beu journal log "exploring the codebase structure"
# Output: Logged: exploring the codebase structure

beu journal log "found the main entry point in src/main.rs"
# Output: Logged: found the main entry point in src/main.rs
```

For important items, use tagged notes. Tags make it easy to scan for specific
kinds of entries later:

```bash
beu journal note --tag decision "using SQLite for persistence -- simple, embedded, no server"
# Output: [decision] using SQLite for persistence -- simple, embedded, no server

beu journal note --tag blocker "need to understand the export format before proceeding"
# Output: [blocker] need to understand the export format before proceeding
```

**Why tagged notes?** Decisions are the most valuable thing to persist -- they
prevent the next agent from re-debating settled questions. Blockers signal
where work stopped and what needs to be unblocked first.

### Step 4: Review Your Session

```bash
beu journal summary
```

Output:

```
Session: j-a1b2c3d4e5f6a7b8
Started: 2026-02-24T15:30:00.123Z

  2026-02-24T15:30:05.456Z: exploring the codebase structure
  2026-02-24T15:30:10.789Z: found the main entry point in src/main.rs
  [decision] 2026-02-24T15:31:00.123Z: using SQLite for persistence
  [blocker] 2026-02-24T15:31:30.456Z: need to understand the export format
```

### Step 5: Close Your Session

```bash
beu journal close
# Output: Session j-a1b2c3d4e5f6a7b8 closed.
```

Closing a session marks it as complete. Start a new one next time:

```bash
beu journal open
# A fresh session. Previous entries are preserved in the database.
```

---

## Artifacts: Tracking Deliverables

**Problem**: Agents produce docs, specs, tests, and configs, but without
tracking it's unclear what's been started, what's in review, and what's done.
When handoffs happen, the next agent doesn't know the status of each
deliverable.

**Solution**: The artifact module tracks named deliverables with a simple
status pipeline: `pending` -> `in-progress` -> `review` -> `done`. Each
artifact has a type (doc, spec, test, config, codelab) and an optional short
description.

### Step 6: Register Artifacts

```bash
beu artifact add api-spec --type spec --description "OpenAPI v3 spec for auth endpoints"
# Output: Tracking 'api-spec' (spec) - status: pending

beu artifact add design-doc
# Output: Tracking 'design-doc' (doc) - status: pending

beu artifact add integration-tests --type test
# Output: Tracking 'integration-tests' (test) - status: pending
```

The default type is `doc`. Use `--type` for other kinds. Use `--description`
to add a short note about what the artifact is for.

### Step 7: Update Artifact Status

As you make progress on each deliverable:

```bash
beu artifact status design-doc in-progress
# Output: 'design-doc': pending -> in-progress

beu artifact status api-spec review
# Output: 'api-spec': pending -> review
```

Valid statuses: `pending`, `in-progress`, `review`, `done`.

### Step 8: Add or Update Descriptions

Descriptions can be added later or updated at any time:

```bash
beu artifact describe design-doc "High-level architecture for the auth module"
# Output: 'design-doc': description updated
```

### Step 9: List and Inspect

```bash
beu artifact list
```

Output:

```
  [in-progress] design-doc (doc) - updated 2026-02-24T15:35:00.123Z
  [pending] integration-tests (test) - updated 2026-02-24T15:33:00.456Z
  [review] api-spec (spec) - updated 2026-02-24T15:34:00.789Z
```

Filter by status:

```bash
beu artifact list --filter pending
```

Show details for a specific artifact:

```bash
beu artifact show api-spec
```

Output:

```
Artifact: api-spec
Type: spec
Status: review
Description: OpenAPI v3 spec for auth endpoints
Created: 2026-02-24T15:32:00.123Z
Updated: 2026-02-24T15:34:00.789Z
```

---

## Compliance Checking: Keeping Docs Up to Date

**Problem**: Agents produce documentation early in a project but forget to
update it as the implementation evolves. After 20 task completions and several
architecture decisions, the design doc is outdated, but nobody notices until
a handoff happens and the next agent is working from stale information.

**Solution**: The `check` command enforces documentation hygiene. You declare
which artifacts are required in config, and `beu check` verifies they exist,
are active (not pending), and are fresh (not too many changes since last
update). Agents call `beu check` periodically -- after completing tasks,
before pausing -- and act on failures.

### Step 9a: Configure Required Docs

Edit `.beu/config.yml` to declare required artifacts and a staleness threshold:

```yaml
required_docs:
  - name: design-doc
    type: doc
  - name: api-spec
    type: spec

staleness_threshold: 10
```

The `staleness_threshold` counts "mutation events" -- successful write
operations like task add, task done, state set, debug log -- that occurred
since each doc was last updated. If the count reaches the threshold, the doc
is flagged as stale.

### Step 9b: Run the Compliance Check

```bash
beu check
```

If everything is up to date:

```
All 2 required docs satisfied.
```

If a doc is stale (many changes happened since it was last touched):

```
ERROR: required doc 'design-doc' is stale (12 changes since last update)
  -> update doc, then: beu artifact changelog design-doc "<summary>"

1/2 required docs satisfied.
```

### Step 9c: Fix Staleness

Update the actual documentation content, then record the update:

```bash
beu artifact changelog design-doc "updated architecture for JWT auth changes"
```

This refreshes the artifact's `updated_at` timestamp. Now `beu check` passes:

```bash
beu check
# All 2 required docs satisfied.
```

**Why staleness detection?** Time-based staleness (e.g., "updated 7 days ago")
doesn't correlate with actual changes. Event-count staleness does -- if 15
tasks were completed since the doc was last touched, it's likely outdated
regardless of wall-clock time.

**What counts as a mutation?** Successful write operations in task, state,
debug, idea, and journal modules. Read-only commands (list, show, get) and
artifact/system commands (which are doc updates themselves) do not count.

---

## Tasks: Breaking Work into Actionable Items

**Problem**: "Implement auth" is too vague to act on. Agents need concrete,
prioritized work items they can pick up one at a time. Without a task list,
work happens in whatever order the agent thinks of, and critical items get
buried under low-priority ones.

**Solution**: The task module provides prioritized work items with a sprint
view. Tasks have statuses (`open`, `in-progress`, `done`, `blocked`),
priorities (`low`, `medium`, `high`, `critical`), and optional tags for
categorization. The sprint view filters out completed work and groups active
items by status.

### Step 10: Create Tasks

```bash
beu task add "implement user authentication" --priority high --tag backend
# Output: #1: implement user authentication [backend] (high)

beu task add "write API documentation" --priority medium --tag docs
# Output: #2: write API documentation [docs] (medium)

beu task add "fix login timeout bug" --priority critical --tag backend
# Output: #3: fix login timeout bug [backend] (critical)

beu task add "add dark mode support" --priority low --tag frontend
# Output: #4: add dark mode support [frontend] (low)
```

**Why priorities?** When an agent starts a session, it needs to know what to
work on first. Critical > high > medium > low. The sprint view orders items
by priority so the most important work surfaces at the top.

**Why tags?** Tags let you slice tasks by area (backend, frontend, docs) to
focus on a specific domain without losing sight of the whole.

### Step 11: Manage Task Workflow

Update task status as you work:

```bash
beu task update 3 --status in-progress
# Output: #3: status: open -> in-progress

beu task update 1 --status blocked
# Output: #1: status: open -> blocked
```

Mark tasks done:

```bash
beu task done 3
# Output: #3 done: fix login timeout bug
```

### Step 12: Sprint Overview

**Why sprint?** The sprint view is the single most useful command for an agent
starting a session. It answers "what needs my attention right now?" by showing
only active work, grouped by urgency.

```bash
beu task sprint
```

Output:

```
Blocked:
  #1 (high) [backend] implement user authentication

Open:
  #2 (medium) [docs] write API documentation
  #4 (low) [frontend] add dark mode support
```

Done tasks are excluded. In-progress items appear first, then blocked, then
open -- each group ordered by priority.

### Step 13: Filter Tasks

```bash
# By status.
beu task list --status open

# By tag.
beu task list --tag backend

# Show details for a specific task.
beu task show 1
```

Output of `task show`:

```
Task #1: implement user authentication
Status: blocked
Priority: high
Tag: backend
Created: 2026-02-24T15:40:00.123Z
Updated: 2026-02-24T15:42:00.456Z
```

---

## State: Persistent Project Memory

**Problem**: AI agents are stateless. Every new session starts from scratch.
Critical knowledge -- "we decided to use JWT", "CI is broken", "focus on the
auth module" -- is lost unless explicitly persisted. The journal records *what
happened*, but state records *what is true right now*.

**Solution**: The state module is a key-value store organized by category.
Categories have semantic meaning:

- `decision` -- settled questions that should not be re-debated
- `blocker` -- issues preventing progress that need resolution
- `focus` -- current priorities the agent should work on
- `note` -- general persistent notes

State entries are upserted (insert or update on conflict), so setting the
same key twice just updates the value.

### Step 14: Record Decisions and Context

```bash
beu state set --category decision auth-method "JWT with RS256 signing"
# Output: [decision] auth-method = JWT with RS256 signing

beu state set --category decision database "PostgreSQL for production, SQLite for dev"
# Output: [decision] database = PostgreSQL for production, SQLite for dev
```

**Why decisions?** This is the highest-value use of state. An agent that sees
`auth-method = JWT with RS256 signing` won't waste time evaluating OAuth vs
sessions vs API keys. The decision is settled.

### Step 15: Track Blockers

```bash
beu state set --category blocker ci-pipeline "flaky test in auth_test.rs line 42"
# Output: [blocker] ci-pipeline = flaky test in auth_test.rs line 42

beu state set --category focus current-task "finish JWT middleware implementation"
# Output: [focus] current-task = finish JWT middleware implementation
```

**Why blockers and focus?** The `resume` command (covered below) automatically
surfaces all blockers and focus items, giving the agent an instant briefing
when it starts work.

### Step 16: Query and Manage State

```bash
# Get a specific entry.
beu state get auth-method
# Output: [decision] auth-method = JWT with RS256 signing

# List all state entries.
beu state list

# List by category.
beu state list --category blocker

# Remove a resolved blocker.
beu state remove ci-pipeline
# Output: Removed 'ci-pipeline'.

# Clear an entire category (requires --force).
beu state clear --category note --force
# Output: Cleared 2 'note' entries.
```

---

## Ideas: Capturing What You Think Of

**Problem**: During focused work, ideas and observations surface that aren't
tasks yet. "We should add rate limiting." "The error messages could be
better." If you stop to create a formal task for each one, you lose flow. If
you ignore them, they're forgotten.

**Solution**: The idea module is a lightweight scratchpad. Ideas have an area
(api, ui, database, testing, docs, tooling, general), a priority, and an
optional description. They exist separately from tasks -- an idea is something
worth remembering, not necessarily something to do next.

### Step 17: Capture Ideas

```bash
beu idea add "add rate limiting to public endpoints" --area api --priority high
# Output: #1: add rate limiting to public endpoints [api] (high)

beu idea add "improve error messages" --area general
# Output: #2: improve error messages [general] (medium)

beu idea add "benchmark database queries" --area database --priority low
# Output: #3: benchmark database queries [database] (low)
```

### Step 18: Manage Ideas

```bash
# Add detail to an idea.
beu idea describe 1 "Use token bucket algorithm, 100 req/min per API key"

# List ideas (archived are excluded by default).
beu idea list

# Filter by area.
beu idea list --area api

# Mark an idea as done (promoted to a task and completed).
beu idea done 2
# Output: #2 done: improve error messages

# Archive an idea (decided not to pursue).
beu idea archive 3
# Output: #3 archived: benchmark database queries
```

**Why separate from tasks?** Tasks are commitments -- things you plan to do
this sprint. Ideas are possibilities. The separation prevents the task list
from being cluttered with "maybe someday" items while still capturing them
persistently.

---

## Debug: Structured Investigation Tracking

**Problem**: Debugging is iterative and messy. You observe symptoms, gather
evidence, form hypotheses, find root causes. Without structure, you go in
circles -- revisiting the same symptoms, forgetting evidence you already
collected, or losing track of which theory you were testing.

**Solution**: The debug module provides structured investigation sessions.
Each session has a slug (auto-generated from the title), a status timeline
(`investigating` -> `root-cause-found` -> `resolved`), and typed entries
(symptom, evidence, cause). The timeline prevents circular investigations
by creating a persistent record of what was found.

### Step 19: Open a Debug Session

```bash
beu debug open "auth middleware returns 500 on expired tokens"
# Output: Debug session 'auth-middleware-returns-500-on-expired-tokens' opened.
```

The slug is auto-generated from the title. If a slug already exists, a
numeric suffix is appended (`-2`, `-3`, etc.).

### Step 20: Record Investigation Progress

```bash
# Record what you're observing.
beu debug symptom auth-middleware-returns-500-on-expired-tokens \
  "500 error with stack trace pointing to jwt_verify.rs line 89"

# Record evidence you've gathered.
beu debug log auth-middleware-returns-500-on-expired-tokens \
  "token expiry check uses system time, not UTC"

# Record the root cause when found.
beu debug cause auth-middleware-returns-500-on-expired-tokens \
  "timezone mismatch: token uses UTC, verify uses local time"
```

Recording a cause automatically updates the session status to
`root-cause-found`.

### Step 21: Resolve and Review

```bash
# Mark the session resolved after the fix is applied.
beu debug resolve auth-middleware-returns-500-on-expired-tokens
# Output: Debug session 'auth-middleware-returns-500-on-expired-tokens' resolved.

# Review the full investigation timeline.
beu debug show auth-middleware-returns-500-on-expired-tokens
```

Output:

```
Debug: auth-middleware-returns-500-on-expired-tokens (resolved)
Title: auth middleware returns 500 on expired tokens
Created: 2026-02-24T16:00:00.123Z
Updated: 2026-02-24T16:15:00.456Z

Timeline:
  [symptom] 2026-02-24T16:01:00: 500 error with stack trace...
  [evidence] 2026-02-24T16:05:00: token expiry check uses system time...
  [cause] 2026-02-24T16:10:00: timezone mismatch...
```

```bash
# List active debug sessions.
beu debug list

# Filter by status.
beu debug list --status investigating
```

**Why structured debugging?** When an agent encounters a bug it has seen
before (or a similar one), the debug timeline provides a reusable playbook.
The symptom/evidence/cause structure forces organized thinking instead of
ad-hoc exploration.

---

## Pause and Resume: Session Continuity

**Problem**: Agents context-switch. A session ends mid-task, and the next
agent needs to know: what was I working on? What's blocking me? What should
I focus on? Reconstructing this from the journal or task list is possible
but slow.

**Solution**: `pause` saves a checkpoint message describing the current state.
`resume` restores it along with all blockers and focus items from the state
module. It's a one-command briefing for the incoming agent.

### Step 22: Pause Work

```bash
beu pause "halfway through JWT middleware, need to handle token refresh next"
# Output: Checkpoint saved. Run 'beu resume' to pick up where you left off.
```

### Step 23: Resume Work

```bash
beu resume
```

Output:

```
Checkpoint: halfway through JWT middleware, need to handle token refresh next

Blockers:
  [blocker] ci-pipeline: flaky test in auth_test.rs line 42

Focus:
  [focus] current-task: finish JWT middleware implementation
```

**Why pause/resume?** This is the glue between sessions. `pause` takes 2
seconds at the end of a session. `resume` saves minutes of context
reconstruction at the start of the next one.

---

## Progress: Cross-Module Summary

**Problem**: With work spread across tasks, artifacts, ideas, and debug
sessions, getting a holistic view requires querying each module separately.

**Solution**: `progress` aggregates counts from all modules into a single
summary.

### Step 24: Check Progress

```bash
beu progress
```

Output:

```
=== Progress Summary ===

Checkpoint: halfway through JWT middleware

Tasks:
  blocked: 1
  done: 1
  open: 2

Artifacts:
  in-progress: 1
  pending: 1
  review: 1

Ideas:
  pending: 1

Active debug sessions: 0
```

---

## System Commands

### Project Overview

```bash
beu status
```

Output:

```
beu project: beu-demo
  modules: journal, artifact, task, state, idea, debug
  data:     52.0KB
  last activity: 2026-02-24T16:20:00.123Z system pause (ok)
  total events: 22
```

### Audit Trail

**Why event logging?** Every command is automatically recorded. This provides
an audit trail for debugging agent behavior -- you can see exactly what the
agent did, in what order, and whether each command succeeded.

```bash
beu events
```

Output:

```
ID    TIMESTAMP              MODULE       COMMAND      ARGS                 STATUS
--------------------------------------------------------------------------------
22    2026-02-24T16:20:00    system       pause                            ok
21    2026-02-24T16:19:00    debug        resolve                          ok
20    2026-02-24T16:18:00    debug        cause                            ok
...

22 event(s) shown
```

Filter by module:

```bash
beu events --module task
```

### Export and Import

Export data as JSON for backups or migration:

```bash
beu export journal > journal-backup.json
beu export --all > full-backup.json
```

Import data back:

```bash
beu import journal journal-backup.json
# Output: Imported 5 rows across 2 tables into 'journal'.
```

### Reset a Module

```bash
# Safety check -- requires --force.
beu reset journal
# Output: error: this will delete all data for 'journal'. Use --force to confirm.

beu reset journal --force
# Output: Reset 'journal': dropped 2 table(s).
```

After reset, schemas are recreated automatically on next use.

### Health Check

```bash
beu health
```

Output:

```
[ok] data/ directory exists
[ok] beu.db valid
[ok] event log accessible

All checks passed.
```

Use `--repair` to attempt automatic fixes for any issues found.

---

## Project Scoping: Multiple Projects in One Database

**Problem**: A monorepo might have multiple logical projects (API, frontend,
worker) sharing a single `.beu` directory. Without scoping, all tasks, state,
and artifacts are mixed together.

**Solution**: The `--project` (or `-p`) flag scopes every command to a named
project within the same database. Each project has its own isolated tasks,
artifacts, state, and event log.

### Step 27: Work in Multiple Projects

```bash
# Add tasks to the API project
beu --project api task add "implement auth endpoint" --priority high
beu --project api task add "add rate limiting" --priority medium

# Add tasks to the frontend project
beu --project frontend task add "login page" --priority high
beu --project frontend task add "dashboard layout" --priority low

# Sprint view for API only
beu --project api task sprint
# Only shows: implement auth endpoint, add rate limiting

# Sprint view for frontend only
beu -p frontend task sprint
# Only shows: login page, dashboard layout
```

Data is fully isolated. A task added with `--project api` is invisible when
querying with `--project frontend` or the default project.

### Step 28: Configure Default Behavior

By default, omitting `--project` uses the "default" project. Change this in
`.beu/config.yml`:

```yaml
# Use "api" as the default when --project is omitted
default_project: api

# Or require explicit --project on every command
require_project: true
```

With `require_project: true`, running `beu task list` without `--project`
produces an error, forcing agents to be explicit about which project they're
operating on.

---

## Cross-Project Discovery

**Problem**: In a monorepo with multiple subprojects (each with their own
`.beu/` directory), you need a way to see the state of all projects at once.

**Solution**: The `project` command discovers all `.beu/` directories in the
repository and queries them read-only.

### Step 29: List All Projects

```bash
beu project list
```

Output:

```
  api (journal, artifact, task, state, idea, debug)
  frontend (journal, artifact, task, state, debug)

2 project(s) found.
```

### Step 30: Cross-Project Progress

```bash
beu project progress
```

Output:

```
=== Repo Projects ===

--- api ---
  Checkpoint: implementing rate limiting
  Tasks:      open: 3, done: 5
  Artifacts:  done: 2, in-progress: 1

--- frontend ---
  Tasks:      open: 2, blocked: 1
  Ideas:      pending: 3

2 project(s).
```

---

## Putting It All Together

Here's a complete workflow for an agent-driven development session:

```bash
# === Session start ===
beu resume                    # Check for checkpoint, blockers, focus
beu journal open              # Start a new session
beu task sprint               # What needs attention?

# === Plan the work ===
beu task add "design auth schema" --priority high --tag auth
beu task add "implement JWT middleware" --priority high --tag auth
beu task add "write auth tests" --priority medium --tag auth
beu artifact add auth-design --type spec --description "Auth module architecture"

# === Work ===
beu task update 1 --status in-progress
beu journal note --tag decision "using RS256 for JWT signing"
beu artifact status auth-design in-progress

# === Complete items ===
beu task done 1
beu artifact status auth-design done
beu task update 2 --status in-progress

# === Capture ideas along the way ===
beu idea add "add rate limiting" --area api --priority high

# === Hit a bug? Track the investigation ===
beu debug open "token verification fails on clock skew"
beu debug symptom token-verification-fails-on-clock-skew "intermittent 401s"
beu debug cause token-verification-fails-on-clock-skew "5-second clock skew tolerance needed"
beu debug resolve token-verification-fails-on-clock-skew

# === Compliance check -- are docs still current? ===
beu check                     # Verify required docs are up to date
# If stale: update docs, then record it
beu artifact changelog auth-design "updated after JWT middleware implementation"
beu check                     # Should pass now

# === Review progress ===
beu progress
beu task sprint
beu artifact list
beu journal summary

# === Session end ===
beu pause "JWT middleware done, starting on auth tests next"
beu journal close
```

## What's Next

- Read the [Architecture doc](architecture.md) for technical internals.
- Check `beu --help` and `beu <module> --help` for full CLI reference.
- Run `cargo test` to see the full test suite (~350 tests).
