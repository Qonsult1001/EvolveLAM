## Session Plan

### Task 1: Populate the connection graph with self-knowledge from learnings
Files: src/memory.rs, src/commands.rs, src/commands_project.rs, src/repl.rs
Description: The entire latent space infrastructure exists (ConnectionGraph, weights, BFS, DAG enforcement, temporal decay, search, paths, downstream queries) but the graph has zero content. Add a `/graph populate` subcommand that reads `memory/learnings.jsonl`, extracts concept tags from each learning's title and takeaway fields, and ingests them as connections (semantic type) into the connection graph. For each learning, parse key concepts (split on common delimiters, extract noun phrases), create connections between co-occurring concepts within the same learning. Write the updated graph back to `memory/connections.jsonl`. This tests whether the architecture works end-to-end and gives the graph real content to query. Write tests for the concept extraction and connection generation logic.
Issue: none

### Task 2: Add `/graph stats` to show connection graph health metrics
Files: src/memory.rs, src/commands.rs, src/commands_project.rs, src/repl.rs
Description: Add a `/graph stats` subcommand that loads the connection graph and displays: total node count, total edge count, edges by kind (semantic/causal/temporal/mathematical/scientific), average weight, strongest connection, most-connected node (highest degree), and isolated nodes (degree 0). This gives visibility into the graph's state — essential for knowing whether `/graph populate` actually worked and for tracking graph growth over sessions. Write tests for the stats computation.
Issue: none

### Issue Responses
(No issues available — gh CLI not installed.)
