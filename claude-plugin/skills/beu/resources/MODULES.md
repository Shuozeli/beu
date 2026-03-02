# beu Module Deep Dives

Detailed breakdown of the `beu` modules and their usage patterns.

## Journal

The `journal` module acts as an agent interaction ledger, tracking interactions within a session.

-   **Session Persistence:** Journal entries are grouped by session ID (automatically managed).
-   **Interaction History:** Logs represent the "what" (agent actions) and "why" (agent rationale).
-   **Categorized Notes:** Use `beu journal note <category> <msg>` to record findings, decisions, or blockers during a session.
-   **Session Summary:** `beu journal summary` provides a concise view of the current interaction.

## Artifact

The `artifact` module tracks the status and progress of deliverables (files, components, or documents).

-   **Deliverable Status:** Track whether an artifact is "in-progress", "ready-for-review", or "done".
-   **Changelog:** `beu artifact changelog <name>` shows the status history of an artifact over time.
-   **List and Show:** Use `beu artifact list` and `beu artifact show <name>` to manage your deliverables.

## Task

The `task` module provides work item tracking with a sprint view and per-task test status tracking.

-   **Work Item Management:** Add, update, and complete tasks with `beu task add`, `beu task update <id>`, and `beu task done <id>`.
-   **Sprint View:** `beu task sprint` groups tasks by status, providing a clear picture of current progress. Includes test status.
-   **Test Status Lifecycle:** Every task tracks `test_status` through the lifecycle: `planned -> designed -> implemented -> tested -> darklaunched -> launched`. Use `beu task test-status <id> <status>` to advance it.
-   **Test Status Filter:** `beu task list --test-status planned` shows all tasks that still need tests written.
-   **Test Patterns Reference:** Run `beu test patterns` to see the four built-in test patterns and decide which apply to the task at hand.
-   **Tagging:** Use `--tag` to group tasks (e.g., by feature area).

## Test

The `test` command provides read-only reference information for agents.

-   **Patterns:** `beu test patterns` displays the built-in test patterns (`unit`, `integration`, `systest`, `golden`) and the test status lifecycle. No arguments, no database required.
-   **Configurable:** Override defaults in `config.yml` with a `test_patterns:` list.
-   **Purpose:** Helps agents decide which test strategy to apply without enforcing a specific choice.

## State

The `state` module stores persistent project memory (decisions, blockers, focus items, notes).

-   **Persistent Key-Value Store:** Use `beu state set <key> <value>` to store arbitrary data that should survive session resets.
-   **Focus Items:** Mark current focus or active blockers using categories.
-   **Cross-Session Recovery:** `beu resume` relies on `state` items to rebuild the session context.

## Idea

The `idea` module is a lightweight capture tool for thoughts that don't yet fit into a task or artifact.

-   **Capture:** Quick additions with `beu idea add <title>`.
-   **Management:** Mark as `done` or `archive` ideas when they are resolved or no longer relevant.
-   **Unstructured Backlog:** Provides a low-friction place for "brainstorming" and thought capture.

## Debug

The `debug` module provides persistent investigation tracking for debugging and research.

-   **Investigation Lifecycle:** From `open` (symptom) to `log` (evidence) and `cause` to `resolve`.
-   **Timeline View:** `beu debug show <id>` provides a chronological view of an investigation's history.
-   **Evidence Capture:** Store log snippets, command outputs, or observations directly in the debug session.
-   **Research History:** Helps prevent redundant investigations by documenting what was already tried.

## Compliance (Check)

The `check` system verifies documentation hygiene using the `artifact` module. Configure required docs in `config.yml`:

```yaml
required_docs:
  - name: design
    type: doc
  - name: changelog
    type: changelog
staleness_threshold: 10
```

`beu check` verifies:
1. **Existence**: Each required doc is registered as an artifact
2. **Status**: Each artifact is not "pending" (must be in-progress, review, or done)
3. **Freshness**: If `staleness_threshold` is set, each artifact must not have N+ mutation events since its last update

**Mutation events** are successful write operations in non-artifact, non-system modules (task add/done/update, state set/remove, debug open/log/resolve, idea add/done, journal open/log/note/close). Read-only commands (list, show, get) do not count.

## Module Gating

Modules can be enabled or disabled in `.beu/config.yml`. When a module is disabled, its CLI commands will return an error, and it will be excluded from `beu progress` and `beu status`.

```yaml
modules:
  journal: true
  artifact: true
  task: true
  state: true
  idea: true
  debug: true
```
