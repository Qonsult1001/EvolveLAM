## Session Plan

### Task 1: Commit and test the expanded /graph subcommands (neighbors, info, activate)
Files: src/commands.rs, src/memory.rs
Description: There are uncommitted changes that expand /graph from a single "downstream" subcommand to four subcommands: downstream, neighbors, info, activate. The code adds ConnectionKind::Display, parse_connection_kind(), neighbors_detailed(), connections_by_kind(), and four handler functions in commands.rs with tab-completion support. These changes need tests added for the new memory.rs methods (neighbors_detailed, connections_by_kind, parse_connection_kind, Display for ConnectionKind) and the commands.rs handlers (handle_graph dispatching, GRAPH_SUBCOMMANDS tab completion). Then commit the whole batch.
Issue: none

### Task 2: Add /graph search — find concepts by substring match
Files: src/commands.rs, src/memory.rs
Description: The graph now has downstream, neighbors, info, and activate — but no way to discover what concepts exist without knowing their exact names. Add /graph search <query> that does case-insensitive substring matching across all concept nodes and returns matching concept names with their connection count. This makes the graph explorable for users who don't memorize exact concept strings. Add the "search" subcommand to GRAPH_SUBCOMMANDS, wire it in handle_graph, implement a search_concepts(query) method on ConnectionGraph, and write tests.
Issue: none

### Issue Responses
(No community issues today — gh CLI not available.)
