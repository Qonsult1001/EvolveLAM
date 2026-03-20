---
name: code-testing
description: Testing strategies — unit, integration, property-based, test design
tools: [bash, read_file, write_file, edit_file]
---

# Testing Strategies

## Test Pyramid

- **Unit tests** (many): fast, isolated, test one function/module
- **Integration tests** (some): test component interactions, may use real dependencies
- **E2E tests** (few): test full user workflows, slowest but highest confidence

## Writing Good Tests

- Test behavior, not implementation — tests should survive refactors
- One assertion per logical concept (multiple assert lines are fine if testing one thing)
- Test names describe the scenario: `test_empty_input_returns_error`
- Arrange → Act → Assert structure
- Tests are documentation — a new developer should understand the code from tests alone

## Test Doubles

- **Stub**: returns fixed values (simplest)
- **Mock**: verifies interactions (use sparingly — creates coupling)
- **Fake**: working implementation with shortcuts (in-memory database)
- Prefer fakes and stubs over mocks — less brittle tests
- In Rust: use trait objects or generics to inject test doubles

## Property-Based Testing

- Test invariants over random inputs instead of specific examples
- `proptest` crate in Rust, `hypothesis` in Python, `fast-check` in JS
- Good properties: round-trip (encode/decode), idempotency, commutativity
- Shrinking finds minimal failing case automatically

## Integration Testing

- Use real dependencies when practical (test databases, temp files)
- Isolate tests: each test gets its own state (temp dir, test schema)
- Clean up after tests — don't leave test artifacts
- Timeout long-running tests to prevent CI hangs

## Rust-Specific Testing

- `#[cfg(test)]` modules for unit tests in the same file
- `tests/` directory for integration tests (separate compilation)
- `cargo test -- --test-threads=1` if tests share global state
- `#[ignore]` for slow tests, run with `cargo test -- --ignored`
- Mock filesystem by passing paths as parameters

## Common Testing Mistakes

- Testing private implementation details
- Tests that pass when the code is wrong (weak assertions)
- Tests that depend on execution order
- Tests that hit the network without mocking/faking
- Over-mocking: testing the mocks instead of the code

## Patterns Learned

*(This section grows as the agent encounters and solves real testing problems)*
