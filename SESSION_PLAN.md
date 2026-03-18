## Session Plan

### Task 1: Update KNOWN_MODELS with current Claude model aliases
Files: src/commands.rs
Description: The KNOWN_MODELS array used for `/model <Tab>` completion is missing the shorter model aliases that are now standard: `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5`. These are the current canonical model IDs. Add them alongside the existing dated variants for backwards compatibility. Also add `nul` to .gitignore (Windows artifact from bash /dev/null redirect).
Issue: none

### Task 2: Add shortest-path query to the connection graph (`/graph path`)
Files: src/memory.rs, src/commands.rs (handle_graph dispatch)
Description: Implement `shortest_path(from: &str, to: &str) -> Option<Vec<String>>` on ConnectionGraph using BFS across all edge types. Returns the sequence of concepts from source to target, or None if no path exists. Wire it into the REPL as `/graph path <from> <to>`. Add "path" to GRAPH_SUBCOMMANDS. Tests: path found (A→B→C), no path between disconnected concepts, direct neighbor path, self-path returns just the concept. This makes the connection graph useful as an exploration tool — "how are these two ideas connected?"
Issue: none

### Issue Responses
(No community issues today.)
