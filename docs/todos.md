# beu TODOs

## Customizable Check Rules

Currently `beu check` has hardcoded rules (existence, status, staleness). Allow more ways to customize checks:

- **Per-doc staleness thresholds**: Different thresholds for different required docs (e.g., changelog needs frequent updates, architecture doc can be stale longer)
- **Custom rule types**: Beyond staleness, support rules like:
  - "doc must have changelog entries" (not just exist)
  - "doc must be updated after any task with tag X is completed"
  - "doc status must be 'review' or 'done' (not just non-pending)"
- **Module-specific mutation filters**: Configure which modules/commands count as mutations per doc (e.g., only task completions trigger staleness for design docs, but all mutations trigger for changelog)
- **Severity levels**: Warn vs fail -- some rules could be warnings that don't block, others are hard failures
- **Rule definitions in config.yml**: Move from hardcoded rules to a declarative rule system:
  ```yaml
  check_rules:
    - name: design-freshness
      doc: design
      type: staleness
      threshold: 15
      modules: [task, debug]
    - name: changelog-required
      doc: changelog
      type: staleness
      threshold: 5
    - name: api-spec-status
      doc: api-spec
      type: status
      required_status: [review, done]
  ```
- **Custom check hooks**: Allow running external scripts as part of `beu check` (e.g., lint documentation files, verify links)

## Project Scoping Enhancements

- Cross-project dependency tracking (project A blocks project B)
- Project templates (init with pre-configured modules/docs)
- Project-level config overrides (each project can have different staleness thresholds)
