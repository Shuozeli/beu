---
description: Save a checkpoint before pausing work
argument-hint: [checkpoint message]
---

Save a checkpoint of the current `beu` session before pausing work.

Run `beu pause <msg>` to record a message describing the current state.

After pausing:
1. Show the checkpoint message.
2. Ensure any active journal sessions or debug investigations are closed or summarized.
3. Suggest running `beu progress` one last time for a final session summary.

If no message is provided as $1, ask the user for a description of the current state.
