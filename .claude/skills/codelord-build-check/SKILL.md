---
name: codelord-build-check
description: >
  Runs the codelord build pipeline: format check, clippy, tests, and build.
  Use when user says "check build", "run checks", "CI check", "does it
  compile", "run tests", or "pre-commit". Do NOT use for architecture review
  or code generation tasks.
---

# Codelord Build Check

Run the full build validation pipeline.

## Instructions

### Step 1: Format check

```bash
just fmt_check
```

If formatting fails, run `just fmt` to fix, then re-check.

### Step 2: Clippy

```bash
just clippy
```

All warnings are errors (`-D warnings`). Fix every warning before proceeding.
Do NOT suppress warnings with `#[allow(...)]` unless there is a clear justification.

### Step 3: Tests

```bash
just test
```

All tests must pass. If a test fails:
1. Read the test to understand intent
2. Fix the code (not the test) unless the test is wrong
3. Re-run to confirm

### Step 4: Build

```bash
just build
```

Verify clean compilation with no warnings.

## Output

Report results as:

```
## Build Check Report

- Format: PASS | FAIL
- Clippy: PASS | FAIL (N warnings fixed)
- Tests: PASS | FAIL (N/M passed)
- Build: PASS | FAIL
```
