# beu-skills

Skill rule files for [beu](../), the persistent session-memory CLI for AI agents.

## Installation

Install beu skills using the [skills](https://github.com/nicepkg/skills) CLI:

```bash
npx skills add Shuozeli/beu --all
```

Or let beu handle it automatically:

```bash
beu init          # installs skill rules on first setup
beu update-rules  # re-downloads the latest skill rules
```

## Files

- `SKILL.md` (at repo root) -- the skill definition with frontmatter + agent instructions
- `beu-skills/skills/beu.md` -- standalone agent rule content
- `beu-skills/bin/install.js` -- legacy CLI installer (for direct invocation)
