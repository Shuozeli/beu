# Code Quality Audit: tools/beu

Audit performed on the Rust CLI tool at `tools/beu/`.

## Findings

### 1. Unnecessary `#[allow(dead_code)]` on structs with all fields used

**Files:**
- `tools/beu/src/store.rs:176` -- `Session` struct
- `tools/beu/src/store.rs:185` -- `JournalEntry` struct
- `tools/beu/src/store.rs:319` -- `Event` struct

All fields of these structs are accessed in production code (`cmd/journal.rs`, `cmd/system.rs`). The `#[allow(dead_code)]` annotations suppress real warnings unnecessarily. Removing them confirms there are no dead fields.

**Fix:** Remove the three `#[allow(dead_code)]` annotations.

### 2. `.unwrap()` in production code (migration function)

**File:** `tools/beu/src/sqlite/task.rs:27`

```rust
let mut stmt = tx.prepare("PRAGMA table_info([tasks])").unwrap();
```

This is the only `.unwrap()` in non-test code. While unlikely to fail in practice, it should use `?` for consistency with the rest of the codebase which properly propagates errors.

**Fix:** Replace `.unwrap()` with `?`.

### 3. Duplicated human-readable byte size formatting

**Files:**
- `tools/beu/src/cmd/system.rs:112-118`
- `tools/beu/src/cmd/project.rs:240-246`

The exact same if/else chain for formatting bytes as B/KB/MB is copy-pasted in two locations.

**Fix:** Extract a `format_byte_size(size: u64) -> String` helper function.

### 4. Redundant `let result = ...; result` pattern in scoping blocks

**Files (examples):**
- `tools/beu/src/sqlite/artifact.rs:57-61` (get_artifact)
- `tools/beu/src/sqlite/state.rs:93-98` (remove, exists check)
- `tools/beu/src/sqlite/debug.rs:52-57` (slug_exists)
- `tools/beu/src/sqlite/idea.rs:174-178` (describe_idea, exists check)

Many query blocks use the pattern:
```rust
let result = {
    let exists = tx.prepare(...)?.exists(...)?;
    exists
};
```

The inner binding is redundant -- the block can return directly without the extra variable.

**Fix:** Simplify to remove redundant inner bindings (e.g., `let exists = { tx.prepare(...)?.exists(...)? };`).

### 5. `serde_yaml` dependency for trivial config (minor)

**File:** `tools/beu/Cargo.toml`

The `serde_yaml` crate is used only for config.yml parsing/writing. The config format is simple enough that TOML or JSON would work and avoid an extra dependency. However, YAML is a reasonable choice for user-facing config, so this is informational only -- no fix needed.

## Summary

| # | Severity | Category | Files |
|---|----------|----------|-------|
| 1 | Low | Noise | `store.rs` |
| 2 | Low | Safety | `sqlite/task.rs` |
| 3 | Low | Duplication | `cmd/system.rs`, `cmd/project.rs` |
| 4 | Low | Style | Multiple sqlite/*.rs files |
| 5 | Info | Dependencies | `Cargo.toml` |

Overall, the codebase is clean and well-structured. The trait-based architecture with clean separation between store traits and SQLite implementations is solid. Error handling is consistent throughout (all functions return `Result` with `Box<dyn Error>`). Tests use Fakes rather than mocks, which is the preferred approach. No security issues, no unsafe code, no significant dead code.
