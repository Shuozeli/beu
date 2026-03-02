# beu Workflow Patterns

Recommended patterns for using `beu` to maintain persistent context across sessions.

## New Session Start

1.  **Resume:** Run `beu resume` to see the last checkpoint, active blockers, and current focus.
2.  **Progress:** Run `beu progress` for a high-level view of all modules.
3.  **Journal:** Open a new journal session for the current interaction.
    ```bash
    beu journal open
    ```

## Task Management

1.  **Plan:** Break down the current request into `beu task` items.
    ```bash
    beu task add "implement feature X" --tag feature
    beu task add "fix bug Y" --tag bug
    ```
2.  **Sprint:** View current work items.
    ```bash
    beu task sprint
    ```
3.  **Execute:** As you complete work, mark tasks as `done`.
    ```bash
    beu task done 1
    ```

## Test Management

Every task has a `test_status` field: `planned -> designed -> implemented -> tested -> darklaunched -> launched`.

1.  **Consult patterns:** Before coding tests, see the reference.
    ```bash
    beu test patterns
    ```
2.  **Design:** Record the test decision.
    ```bash
    beu task test-status 1 designed
    ```
3.  **Implement:** After writing the tests.
    ```bash
    beu task test-status 1 implemented
    ```
4.  **Verify:** After all tests pass.
    ```bash
    beu task test-status 1 tested
    ```
5.  **Pre-pause checklist:** Find any tasks without tests.
    ```bash
    beu task list --test-status planned
    ```

## Persistent Memory (State)

Use `beu state` to store decisions or findings that should persist across sessions.
```bash
beu state set "database-choice" "sqlite" --category decision
beu state set "refactoring-needed" "true" --category blocker
```

## Debugging Investigations

When investigating a bug, use the `debug` module to track evidence and symptoms.
1.  **Open:** Start a new investigation.
    ```bash
    beu debug open "investigate issue X" "error log Y"
    ```
2.  **Evidence:** Log findings as you go.
    ```bash
    beu debug symptom "reproducible in test Z"
    beu debug log "database connection timeout observed"
    ```
3.  **Resolve:** Record the root cause and fix.
    ```bash
    beu debug cause "missing index on table A"
    beu debug resolve "added index and verified"
    ```

## Pre-Compaction Protocol

Before the conversation is compacted (Claude Code automatically triggers this):
1.  **Summary:** Record a brief session summary in the journal.
    ```bash
    beu journal summary "completed feature X, identified bug Y"
    ```
2.  **Checkpoint:** Save a checkpoint with `beu pause`.
    ```bash
    beu pause "feature X implemented, waiting for review"
    ```
3.  **Close:** Close the journal session.
    ```bash
    beu journal close
    ```
4.  **Progress:** Run `beu progress` one last time to ensure all state is captured.

## Deliverable Tracking (Artifacts)

Use `beu artifact` to track the status of specific deliverables or files.
```bash
beu artifact add "src/main.rs" --status "in-progress"
beu artifact status "src/main.rs" "ready-for-review"
```

## Compliance Checking

Run `beu check` after completing significant work to ensure documentation stays current.

### When to Run `beu check`
- After completing a batch of tasks (`beu task done`)
- Before `beu pause` (pre-compaction)
- After major architecture decisions (`beu state set --category decision`)
- Before submitting PRs or releasing

### Handling Staleness
If `staleness_threshold` is configured and `beu check` reports a stale doc:
1. Review what changed since the doc was last updated (use `beu events` to see recent activity)
2. Update the relevant documentation
3. Record the update: `beu artifact changelog <name> "updated for <summary>"`
4. Re-run `beu check` to confirm compliance

### Example Compliance Flow
```bash
# After completing several tasks...
beu check
# ERROR: required doc 'design' is stale (12 changes since last update)
#   -> update doc, then: beu artifact changelog design "<summary>"

# Fix: update the design doc content, then record it
beu artifact changelog design "updated for auth refactoring and project scoping"
beu check  # All 2 required docs satisfied.
```

## Multi-Project Workflows

Use `--project` to scope work when managing multiple logical projects in one repository.

```bash
# Work on the API project
beu --project api task add "implement auth endpoint" --priority high
beu --project api task sprint

# Work on the frontend project
beu --project frontend task add "login page" --priority medium
beu --project frontend task sprint

# Cross-project overview
beu project progress
```

## Keeping Agent Rules Up to Date

When `beu` is upgraded, existing projects retain old rule files because `beu init` skips existing ones. To push new agent instructions to an already-initialized project:

```bash
beu update-rules
```

Or reinstall directly via the skills CLI:

```bash
npx skills add Shuozeli/beu --all
```
