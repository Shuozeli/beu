---
description: Work item tracking with sprint view and test status lifecycle
argument-hint: [add|list|update|done|sprint|test-status]
---

Work item tracking for linear and structured task lists, with per-task test status tracking.

Run `beu task <cmd>` to manage tasks.

Available commands:
- `add <title> [--priority priority] [--tag tag]`: Add a new task (test_status starts as `planned`).
- `list [--status status] [--tag tag] [--test-status test_status]`: List tasks.
- `update <id> [--status status] [--priority priority] [--tag tag]`: Update task details.
- `done <id>`: Mark a task as completed.
- `test-status <id> <status>`: Update the test status. Values: `planned`, `designed`, `implemented`, `tested`, `darklaunched`, `launched`.
- `sprint`: Show tasks grouped by status (Sprint View). Includes test status.

Test workflow:
1. `beu test patterns` -- see available test patterns before deciding what to write.
2. `beu task test-status <id> designed` -- decided what tests to write.
3. `beu task test-status <id> implemented` -- tests written.
4. `beu task test-status <id> tested` -- tests passing.
5. `beu task list --test-status planned` -- find tasks still needing tests.

Suggest running `beu task sprint` to see the current task breakdown and status.
If no subcommand is provided, ask the user what action to perform on the task list.
