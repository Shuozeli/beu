---
description: Persistent investigation tracking
argument-hint: [open|log|symptom|cause|resolve|list|show]
---

Persistent investigation tracking for debugging and research.

Run `beu debug <cmd>` to manage investigations.

Available commands:
- `open <title> <symptom>`: Open a new debug investigation session.
- `log <msg>`: Log evidence in a debug session.
- `symptom <msg>`: Record a symptom in a debug session.
- `cause <msg>`: Record root cause.
- `resolve <resolution>`: Mark as resolved.
- `list`: List all debug sessions.
- `show <id>`: Show debug session timeline.

Suggest this command for investigations that span multiple interactions.
If no subcommand is provided, ask the user what action to perform on the debug investigations.
