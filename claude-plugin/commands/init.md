---
description: Initialize beu in the current project
---

Initialize `beu` session memory in the current directory.

Run `beu init` to create the `.beu` directory, default configuration, and agent rule files.

`beu init` creates:
- `.beu/data/beu.db` -- SQLite database with all module tables
- `.beu/config.yml` -- module and compliance configuration
- `.claude/rules/beu.md` -- agent rules for Claude Code
- `.gemini/rules/beu.md` -- agent rules for Gemini
- `.agent/rules/beu.md` -- agent rules for Antigravity

Agent rule files are skipped if they already exist, so user customizations are preserved.

After initialization:
1. Show the project status using `beu status`.
2. Suggest opening a new journal session with `beu journal open`.
3. Explain the basic workflow of tracking tasks and artifacts.

If `beu` is already initialized, inform the user and show project overview using `beu status`.
