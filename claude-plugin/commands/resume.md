---
description: Resume a beu session
---

Resume the current `beu` session by showing the last checkpoint, active blockers, and current focus items.

Run `beu resume` to recover context from the previous interaction.

After resuming:
1. Show the last checkpoint message.
2. List any active blockers or focus items recorded in the `state` module.
3. Show the current project status and counts for each module.

If there is no active checkpoint or focus items, inform the user and suggest starting a new task or journal session.
