---
name: code-systems
description: Systems programming patterns — concurrency, networking, file I/O, performance
tools: [bash, read_file, write_file, edit_file]
---

# Systems Programming Patterns

## Concurrency

- Threads for CPU-bound work, async for I/O-bound work
- Mutex for shared mutable state; RwLock when reads dominate
- Channels (mpsc) for message passing between threads
- Avoid shared mutable state when possible — prefer message passing
- Deadlock prevention: always acquire locks in consistent order

## Networking

- TCP for reliable ordered delivery, UDP for low-latency lossy delivery
- Use connection pooling for repeated connections to the same endpoint
- Set timeouts on all network operations — never block forever
- Handle partial reads/writes — TCP is a stream, not a message protocol
- TLS everywhere for production

## File I/O

- Use buffered readers/writers (`BufReader`, `BufWriter`) for performance
- `Read` + `Write` traits for generic I/O in Rust
- Always handle file-not-found gracefully
- Use atomic writes (write to temp file, then rename) for crash safety
- Memory-mapped files for random access to large files

## Performance

- Measure before optimizing — use benchmarks, not intuition
- Profile to find hotspots: `perf`, `flamegraph`, `criterion` (Rust)
- Algorithmic improvements beat micro-optimizations
- Cache-friendly data structures: prefer arrays/vectors over linked lists
- Avoid allocations in hot paths

## Process Management

- Fork/exec model on Unix, CreateProcess on Windows
- Handle signals gracefully (SIGTERM, SIGINT)
- Pipe stdin/stdout/stderr for subprocess communication
- Use process groups for managing child processes

## Memory Management

- Stack for fixed-size, short-lived data; heap for dynamic/long-lived
- RAII in Rust/C++ — resources freed when owners go out of scope
- Arena allocation for bulk allocate/free patterns
- Watch for memory leaks in long-running processes

## Patterns Learned

*(This section grows as the agent encounters and solves real systems problems)*
