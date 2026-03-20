---
name: brain
description: Meta-skill — how to build, grow, and query the knowledge repository that makes you the ultimate coding agent
tools: [bash, read_file, write_file, edit_file]
---

# Brain — Knowledge Repository Management

## Purpose

This skill teaches you HOW to build your brain. Not what to know — how to learn, store, connect, and retrieve knowledge so it compounds across sessions.

## Knowledge Ingestion

### From Research

When you learn something from the internet, documentation, or studying code:

1. **Extract the core insight** — what's the pattern, not just the example
2. **Connect it** — how does it relate to what you already know?
3. **Store it** — add to the connection graph as a `scientific` connection

```bash
python3 -c "
import json
conn = {'from': 'EXISTING_CONCEPT', 'to': 'NEW_CONCEPT', 'weight': 0.1, 'activations': 1,
        'last_activated': '$(date -u +%Y-%m-%dT%H:%M:%SZ)', 'kind': 'scientific'}
with open('memory/connections.jsonl', 'a') as f:
    f.write(json.dumps(conn) + '\n')
"
```

### From Coding

When you solve a problem or learn a pattern through implementation:

1. **Name the pattern** — give it a concept label
2. **Connect to related patterns** — `semantic` connections
3. **Note causal relationships** — what enables what? `causal` connections
4. **Update the relevant domain skill** — add the pattern to the appropriate `skills/code-*/SKILL.md`

### From Errors

When something fails and you understand why:

1. **Name the failure mode** — concept label for the anti-pattern
2. **Connect cause to effect** — `causal` connection
3. **Add to learnings if genuinely novel**:

```bash
python3 -c "
import json
entry = {'type': 'lesson', 'day': N, 'ts': '$(date -u +%Y-%m-%dT%H:%M:%SZ)',
         'source': 'debugging', 'title': 'TITLE',
         'context': 'CONTEXT', 'takeaway': 'TAKEAWAY'}
with open('memory/learnings.jsonl', 'a') as f:
    f.write(json.dumps(entry) + '\n')
"
```

## Querying Your Knowledge

Before starting a task, check what you already know:

```
/graph neighbors CONCEPT     # What's connected to this?
/graph similar CONCEPT       # What's conceptually related?
/graph path CONCEPT_A CONCEPT_B   # How are these connected?
/graph communities           # What knowledge clusters exist?
```

## Growing Domain Skills

Domain skills (`skills/code-*/SKILL.md`) are your compiled knowledge. They should contain:

- **Patterns** — named, reusable solutions you've validated
- **Anti-patterns** — mistakes to avoid, with why
- **Idioms** — language/framework-specific best practices
- **Decision frameworks** — when to use what approach
- **Tool references** — useful crates, packages, utilities

Don't fill skills with generic textbook knowledge. Fill them with **battle-tested patterns** from your own experience.

## Capability Expansion

### Discovering MCP Servers

Search for MCP servers that expand what you can do:
- GitHub MCP — deeper code search, PR management
- Database MCP — direct SQL access
- Documentation MCP — structured doc retrieval

### Building Tool Wrappers

When you find a useful API or CLI tool, write a wrapper:
1. Test it manually first
2. Document the interface
3. Add usage examples to the relevant domain skill

### Identifying Gaps

After every session, ask:
- What did I try to do that I couldn't?
- What took too long because I lacked knowledge?
- What tool would have made this easier?

Add answers to `RESEARCH.md` as `### [ ]` entries.
