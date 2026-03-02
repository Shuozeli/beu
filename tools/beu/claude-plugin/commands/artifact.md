---
description: Deliverable progress tracking
argument-hint: [add|status|list|show|changelog]
---

Deliverable tracking for files, components, or documents.

Run `beu artifact <cmd>` to manage artifacts.

Available commands:
- `add <name> [--status status]`: Add a new artifact.
- `status <name> <status>`: Update artifact status (e.g., "in-progress", "done").
- `list`: List all artifacts tracked in the current project.
- `show <name>`: Show artifact details, including its status and metadata.
- `changelog <name>`: Show artifact status history over time.

Suggest this command to track the status of specific files or project deliverables.
If no subcommand is provided, ask the user what action to perform on the artifact.
