---
name: code-rust
description: Rust coding patterns, idioms, and best practices — grows through experience
tools: [bash, read_file, write_file, edit_file]
---

# Rust Coding Patterns

## Error Handling

- Prefer `Result<T, E>` over `panic!` in library/production code
- Use `thiserror` for library errors, `anyhow` for application errors
- The `?` operator is your friend — chain it through functions that return Result
- `unwrap()` is acceptable only in tests and when the invariant is provably true
- Replace `expect("message")` with proper match arms in code paths that can fail at runtime

## Ownership & Lifetimes

- Take `&str` not `String` in function parameters when you don't need ownership
- Return `String` when the caller needs to own the data
- Use `Cow<'_, str>` when you sometimes need to allocate, sometimes not
- Clone is fine for prototyping; profile before optimizing away clones

## Structs & Traits

- Derive `Debug` on everything; derive `Clone, PartialEq` when useful
- Implement `Display` for user-facing output, `Debug` for developer output
- Use builder pattern for structs with many optional fields
- Prefer composition over inheritance (there is no inheritance — use trait objects or enums)

## Async

- `tokio` is the standard runtime for async Rust
- Use `async fn` + `.await` — avoid manual `Future` implementations
- `tokio::spawn` for concurrent tasks; `join!` to await multiple futures
- Be careful with `Mutex` across `.await` points — use `tokio::sync::Mutex`

## Testing

- `#[cfg(test)] mod tests { ... }` in the same file for unit tests
- `tests/` directory for integration tests
- Use `assert_eq!`, `assert!(condition)`, `assert!(result.is_err())`
- `#[should_panic]` for testing expected panics
- Mock filesystem operations by parameterizing paths, not by mocking the fs

## Common Crates

- `serde` / `serde_json` — serialization (derive `Serialize, Deserialize`)
- `clap` — CLI argument parsing (derive API)
- `reqwest` — HTTP client
- `tokio` — async runtime
- `regex` — regular expressions
- `chrono` — date/time handling

## Patterns Learned

*(This section grows as the agent encounters and solves real Rust problems)*

- Use `pub(crate)` instead of `pub` for functions that only need to be visible within the crate (e.g., utility functions tested from sibling modules)
- `LazyLock<Mutex<T>>` for session-scoped mutable state in Rust — avoids unsafe statics while keeping initialization lazy
- Use `type` aliases to satisfy clippy::type_complexity when generic types get deeply nested (e.g., `type ThrottleState = HashMap<String, (u32, u32, u64)>`)
- `splitn(2, ' ')` is the idiomatic way to split "first_word rest_of_string" in Rust — returns at most 2 parts
- Mark functions only used in tests with `#[cfg(test)]` to avoid dead_code warnings in release builds
