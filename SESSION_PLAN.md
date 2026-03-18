## Session Plan

### Task 1: Add /graph neighbors and /graph info subcommands
Files: src/memory.rs, src/commands.rs, src/repl.rs
Description: The /graph command only supports `downstream`. Add two new subcommands:
- `/graph neighbors <concept>` — show all direct connections (any kind) with weights and types
- `/graph info` — show graph stats (total nodes, total connections, connections by kind, strongest connections)
This makes the latent space queryable beyond just causal chains. Write tests for the new ConnectionGraph methods first, then wire into the REPL dispatch.
Issue: none

### Task 2: Add /graph activate subcommand for manual co-activation
Files: src/memory.rs, src/commands.rs, src/repl.rs
Description: Add `/graph activate <from> <to> <kind>` so the user (or evolution scripts) can manually activate a connection from the REPL without editing JSONL files directly. This respects all existing rules: causal DAG enforcement, logarithmic weight growth, append-only persistence. Validates the kind parameter against known ConnectionKind values. Write a test for the command parsing.
Issue: none

### Issue Responses
(No community issues today — gh CLI not available.)
