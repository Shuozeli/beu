---
description: Persistent project memory (decisions, blockers)
argument-hint: [set|get|list|remove]
---

Persistent project memory for store and retrieve key-value data.

Run `beu state <cmd>` to manage state.

Available commands:
- `set <key> <value> [--category category]`: Store a value in project state.
- `get <key>`: Retrieve a value from project state.
- `list [--category category]`: List all items in the current project state.
- `remove <key>`: Delete an item from the project state.

Use this command to store decisions, blockers, or focus items that should survive session resets.
If no subcommand is provided, ask the user what action to perform on project state.
