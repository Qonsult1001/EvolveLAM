#!/bin/bash
# scripts/yoyo_context.sh — Build yoyo's identity context for prompts.
# Source this file, then use $YOYO_CONTEXT in any prompt.
#
# Usage:
#   YOYO_REPO="/path/to/yoyo-evolve" source scripts/yoyo_context.sh
#   cat > prompt.txt <<EOF
#   $YOYO_CONTEXT
#   ... your task-specific instructions ...
#   EOF
#
# Reads: IDENTITY.md, PERSONALITY.md, memory/active_learnings.md, memory/active_social_learnings.md
# These are yoyo's stable identity files — who it is, how it speaks,
# what it's learned about itself, and what it's learned from humans.

_YOYO_REPO="${YOYO_REPO:-.}"

_IDENTITY=""
if [ -f "$_YOYO_REPO/IDENTITY.md" ]; then
    _IDENTITY=$(cat "$_YOYO_REPO/IDENTITY.md") || {
        echo "WARNING: Failed to read IDENTITY.md" >&2
        _IDENTITY=""
    }
else
    echo "WARNING: IDENTITY.md not found at $_YOYO_REPO/IDENTITY.md" >&2
fi

_PERSONALITY=""
if [ -f "$_YOYO_REPO/PERSONALITY.md" ]; then
    _PERSONALITY=$(cat "$_YOYO_REPO/PERSONALITY.md") || {
        echo "WARNING: Failed to read PERSONALITY.md" >&2
        _PERSONALITY=""
    }
else
    echo "WARNING: PERSONALITY.md not found at $_YOYO_REPO/PERSONALITY.md" >&2
fi

# Active learnings — no warning if missing
_LEARNINGS=""
if [ -f "$_YOYO_REPO/memory/active_learnings.md" ]; then
    _LEARNINGS=$(cat "$_YOYO_REPO/memory/active_learnings.md") || _LEARNINGS=""
fi

# Active social learnings — no warning if missing
_SOCIAL_LEARNINGS=""
if [ -f "$_YOYO_REPO/memory/active_social_learnings.md" ]; then
    _SOCIAL_LEARNINGS=$(cat "$_YOYO_REPO/memory/active_social_learnings.md") || _SOCIAL_LEARNINGS=""
fi

# Latent space connections — summarize top connections by weight
_CONNECTIONS=""
if [ -f "$_YOYO_REPO/memory/connections.jsonl" ]; then
    _CONNECTIONS=$(python3 -c "
import json, sys
conns = []
for line in open('$_YOYO_REPO/memory/connections.jsonl'):
    line = line.strip()
    if not line: continue
    try: conns.append(json.loads(line))
    except: pass
if not conns:
    sys.exit(0)
conns.sort(key=lambda c: c.get('weight', 0), reverse=True)
nodes = set()
for c in conns:
    nodes.add(c['from'])
    nodes.add(c['to'])
total_act = sum(c.get('activations', 0) for c in conns)
print(f'Nodes: {len(nodes)} | Connections: {len(conns)} | Total activations: {total_act}')
print()
for c in conns[:15]:
    kind = c.get('kind', '?')
    print(f'- {c[\"from\"]} -> {c[\"to\"]} (w:{c[\"weight\"]:.2f}, {c[\"activations\"]}x, {kind})')
" 2>/dev/null) || _CONNECTIONS=""
fi

YOYO_CONTEXT="=== WHO YOU ARE ===

${_IDENTITY:-Read IDENTITY.md for your rules and constitution.}

=== YOUR VOICE ===

${_PERSONALITY:-Read PERSONALITY.md for your voice and values.}

=== SELF-WISDOM ===

${_LEARNINGS:-No learnings yet.}

=== SOCIAL WISDOM ===

${_SOCIAL_LEARNINGS:-No social learnings yet.}

=== LATENT SPACE ===

${_CONNECTIONS:-No connections yet. Use the ConnectionGraph API in src/memory.rs to build associative memory.}"
