---
name: codelord-architecture-guard
description: >
  Validates code changes against codelord's ECS architecture, voice-first
  philosophy, held-key modality, and data-oriented design. Use when reviewing
  PRs, planning features, or when user says "check architecture", "validate
  design", "architecture review", or "guard check". Do NOT use for simple
  formatting or typo fixes.
---

# Codelord Architecture Guard

Validate changes against codelord's core architectural rules.

## Process

### Step 1: Identify what changed

Read the modified files. Classify each change by crate and subsystem.

### Step 2: Check ECS discipline

For each change in `codelord-core/`:

- **Components** must be plain data structs. No methods with side effects.
- **Resources** hold shared mutable state. Must be `Send + Sync` if cross-thread.
- **Systems** contain ALL logic. Systems read Components/Resources, produce side effects.
- Every feature module follows: `mod.rs` + `components.rs` + `resources.rs` + `systems.rs` (+ optional `events.rs`, `bundles.rs`)
- VIOLATION if: logic lives in component structs, UI code directly mutates resources without systems, or feature module skips the pattern.

### Step 3: Check UI layer separation

For each change in `codelord-components/`:

- UI components receive data, render UI, return responses. No state management.
- All state flows through ECS resources from `codelord-core`.
- Atomic design hierarchy: atoms -> molecules -> organisms -> pages/panels/views.
- VIOLATION if: UI components own state, bypass ECS, or skip hierarchy levels.

### Step 4: Check data-oriented design

- No heap allocation in render loops or per-frame paths
- Linear access patterns preferred over random access
- `SmallVec`/`ThinVec` for small collections, arenas for tree structures
- No trait objects in hot paths - use enums for closed sets
- Prefer `&str` over `String` in UI rendering
- VIOLATION if: `Box<dyn Trait>` in render path, unnecessary `Vec` allocations per frame, or pointer chasing in hot loops.

### Step 5: Check crate boundaries

- `codelord-core` does NOT depend on `codelord-components` (one-way)
- `codelord-protocol` contains ONLY serializable DTOs
- `codelord-server` does NOT depend on UI crates
- VIOLATION if: circular dependencies or leaking UI types into non-UI crates.

### Step 6: Check philosophy alignment

- **Voice-first**: New features must be voice-controllable. Voice is primary, not accessory.
- **Held-key modality**: No toggle modes. Hold = active. Release = back to typing.
- **Zero-config**: New settings must have visual UI equivalents. No config-file-only options.
- **Data flow**: One-way. Systems -> Resources -> UI. Never reverse.
- VIOLATION if: feature requires config file editing, introduces toggle modes, or breaks one-way data flow.

## Output Format

```
## Architecture Guard Report

### Verdict: PASS | WARN | FAIL

### Findings
[For each finding:]
- **Rule**: [which rule violated]
- **Location**: file:line
- **Issue**: [what's wrong]
- **Severity**: BLOCKING | WARNING | NOTE
- **Fix**: [how to fix]
```

If no issues found, state PASS with a one-line confirmation.

## Reference

See `references/architecture-quick-ref.md` for the full reference.
