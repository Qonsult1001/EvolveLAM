---
name: build
description: Write code, create artifacts, and implement changes following the project's build-test-verify cycle.
tools: [bash, read_file, write_file, edit_file]
---

# Build

You are implementing a change. This skill covers writing code, creating new
files, and producing working artifacts. It complements `evolve` (which is
specifically about self-modification) — use `build` for general implementation
work including features, refactors, and fixes.

## Before writing code

1. **Read the plan.** Know which task you're implementing and why.
2. **Read the relevant source.** Understand what exists before changing it.
3. **Check JOURNAL.md** for past attempts at similar work.
4. **Check memory/active_learnings.md** for relevant lessons.

## Implementation workflow

1. **Write the test first.** Define what success looks like before writing production code.
2. **Make the smallest change that works.** Don't over-engineer.
3. **Use edit_file for surgical edits.** Don't rewrite entire files when a few lines will do.
4. **Verify after each change:**
   ```bash
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   cargo build
   cargo test
   ```
5. **Commit when green.** One focused commit per change:
   ```bash
   git add -A && git commit -m "Day N (HH:MM): <short description>"
   ```

## Creating new files

- Only create files when genuinely needed. Prefer editing existing files.
- New modules must be declared in their parent (`mod new_module;` in `main.rs` or `lib.rs`).
- New files need tests. Add them in the same file or a test module.

## Adding dependencies

Before adding any crate:
1. Check it on crates.io — significant downloads, active repo, known maintainers.
2. Verify it actually solves the problem better than writing it yourself.
3. Run `cargo build` and `cargo test` after adding.

## Rules

- Never delete existing tests.
- Never modify protected files (PERSONALITY.md, scripts/evolve.sh, .github/workflows/, etc.).
- If build fails after 3 attempts, revert with `git checkout -- src/ Cargo.toml Cargo.lock`.
- Keep changes focused. One feature or fix per commit.
- Don't refactor surrounding code unless the plan calls for it.

## When to use build vs evolve

- **build**: General implementation work — features, fixes, new modules, dependency updates.
- **evolve**: Specifically about self-modification strategy, safety rules, and the evolution lifecycle.

Use `build` when you know what to do. Use `evolve` when deciding *how* to change yourself.
