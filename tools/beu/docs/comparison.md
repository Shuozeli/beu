# beu vs get-shit-done vs beads

`beu` is a personal tool for vibe coding. This document explains where it
fits relative to two other tools in the same space.

> This is a personal library. It solves problems specific to how I work and may
> not fit general use cases.

---

## Overview

| | beu | get-shit-done | beads |
|---|---|---|---|
| Primary purpose | Agent session memory | Personal task manager | Project knowledge base / Dolt-based data layer |
| Target user | AI agents (Claude, Gemini) during vibe coding | Human productivity | Humans + agents for structured knowledge |
| State model | SQLite per-project, session-scoped | Flat files or simple DB | Dolt (versioned SQL database) |
| Interface | CLI (`beu <module> <command>`) | CLI / TUI | CLI + web |
| Persistence scope | Single project (`.beu/` dir) | Global or per-workspace | Repo-wide, version controlled |
| Offline-first | Yes | Yes | Requires Dolt |

---

## beu

**What it is**: A CLI that gives AI agents persistent memory within a coding
session. When I'm vibe coding -- letting Claude or Gemini drive implementation
-- the agent needs somewhere to track what it decided, what it's working on,
and where it got stuck, so it doesn't repeat itself or lose context mid-session.

**What it solves**:

- Agents lose context between conversation turns and sessions
- No built-in way for agents to track tasks, decisions, or blockers across
  a long coding session
- Debug investigations go in circles without structured tracking

**What it does not solve**:

- Human project management (use a proper issue tracker for that)
- Collaboration across team members (no sync, no sharing)
- Long-term knowledge retention beyond a project (that's beads' job)

**Honest assessment**: beu is opinionated toward my workflow. The session
protocol (`journal open` -> work -> `check` -> `pause`) is something I've
found useful for sustained vibe coding sessions. The assumption that "the user
is an AI agent" shapes every design decision. It probably won't fit teams or
users with different workflows.

---

## get-shit-done

**What it is**: A minimal personal task manager focused on human productivity.
The core use case is capturing and triaging tasks quickly, without ceremony.

**Key difference from beu**: get-shit-done is built for humans. beu is built
for AI agents. The UX, command names, and data model all reflect that. You
wouldn't run `beu journal open` as a human -- the session model only makes
sense when the "user" is an agent that might be restarted or replaced mid-task.

**Overlap**: Both have task/todo tracking. beu's task module covers the same
ground as get-shit-done's core, but beu buries it as one of six modules. If
you just want human task management, get-shit-done is the better fit.

**Complement**: I use both. get-shit-done for my own TODO list. beu for what
agents are working on.

---

## beads

**What it is**: A structured knowledge base built on Dolt (a version-controlled
SQL database). It's for capturing and organizing knowledge in a way that
survives across projects and is queryable over time.

**Key difference from beu**: beads is about long-term knowledge. beu is about
short-term session memory. The mental model:

- beu: "what did the agent do in this session?" -> ephemeral, project-local
- beads: "what do I know about this domain?" -> persistent, cross-project

**Overlap**: Both store structured data persistently. The storage layer is
completely different (SQLite local vs Dolt with version history and branching).

**Complement**: beads is the external knowledge base that an agent can consult
for context. beu is the working memory the agent maintains during a session.
They serve different layers of the same cognitive stack.

---

## When to use what

| Situation | Tool |
|---|---|
| AI agent needs to track its work mid-session | beu |
| I need to track my personal TODOs | get-shit-done |
| I want to build a queryable knowledge base | beads |
| I want to persist decisions across projects | beads |
| I want to see what the agent did in the last hour | beu (`beu events`) |
| I want to resume where the agent left off | beu (`beu resume`) |

---

## Design philosophy

beu is not trying to be a general-purpose tool. It is intentionally narrow:

- Built for AI agents, not humans
- Built for vibe coding sessions, not team projects
- Built to be instant and offline (no network, no servers)
- Built to be throwaway (`.beu/data/` is gitignored, sessions close and open)

If you're not doing AI-assisted vibe coding for extended sessions, beu is
probably not for you. That's fine.
