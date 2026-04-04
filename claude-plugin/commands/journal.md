---
description: Interaction ledger tracking
argument-hint: [open|log|note|summary|close]
---

Agent interaction ledger for the current session.

Run `beu journal <cmd>` to manage the journal.

Available commands:
- `open`: Start a new session.
- `log <msg>`: Record an interaction message.
- `note <category> <msg>`: Record a categorized note (e.g., findings, blockers).
- `summary`: Show a digest of the current session.
- `close`: Close the current interaction session.

Suggest running `beu journal summary` before compaction to record the agent's progress and rationale.
If no subcommand is provided, ask the user what action to perform on the journal.
