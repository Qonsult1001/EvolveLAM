---
name: research
description: Search the web and read documentation when stuck or learning something new. Also search and read ML/AI research papers on arXiv.
tools: [bash]
---

# Research

You have internet access through bash. Use it when you're stuck,
when you're implementing something unfamiliar, or when you want
to see how others solved a problem.

## How to search

```bash
curl -s "https://lite.duckduckgo.com/lite?q=your+query" | sed 's/<[^>]*>//g' | head -60
```

## How to read a webpage

```bash
curl -s [url] | sed 's/<[^>]*>//g' | head -100
```

## How to read Rust docs

```bash
curl -s https://docs.rs/[crate]/latest/[crate]/ | sed 's/<[^>]*>//g' | head -80
```

## How to study other agents

```bash
curl -s https://raw.githubusercontent.com/[org]/[repo]/main/src/main.rs | head -200
```

## How to search arXiv papers

```bash
# Search by keyword (returns Atom XML — extract titles, IDs, and summaries)
curl -s "http://export.arxiv.org/api/query?search_query=all:linear+attention&max_results=5" \
  | grep -E '<title>|<id>|<summary>' | sed 's/<[^>]*>//g' | sed 's/^ *//'
```bash
# Search within specific categories (cs.CL, cs.LG, cs.AI, stat.ML, etc.)
curl -s "http://export.arxiv.org/api/query?search_query=cat:cs.CL+AND+all:embedding&max_results=5" \
  | grep -E '<title>|<id>|<summary>' | sed 's/<[^>]*>//g' | sed 's/^ *//'
```bash
# Read a specific paper's abstract by arXiv ID
curl -s "http://export.arxiv.org/api/query?id_list=2305.13245" \
  | grep -E '<title>|<author>|<summary>' | sed 's/<[^>]*>//g' | sed 's/^ *//'
```bash
# Read the full paper (PDF → text, requires pdftotext)
curl -sL "https://arxiv.org/pdf/2305.13245" -o /tmp/paper.pdf \
  && pdftotext /tmp/paper.pdf - | head -200
```bash
# Browse recent papers in a category via the RSS feed
curl -s "https://rss.arxiv.org/rss/cs.LG" | grep -E '<title>|<link>' | sed 's/<[^>]*>//g' | head -40
```

## Rules

- Have a specific question before searching. No aimless browsing.
- Prefer official docs over random blogs.
- When studying other projects, note what's good AND what you'd do differently.
- When reading papers, focus on the method and results sections relevant to your question.
- Prefer recent papers but check citation counts — seminal older papers matter too.

## When to research

- You're implementing something you've never done before
- You hit an error you don't understand
- You want to see how Claude Code or other agents handle something
- A community issue references a concept you're unfamiliar with
- You're choosing between multiple approaches and want to see conventions
- You need to understand a specific ML/AI technique, architecture, or training method
- You want to find prior work or baselines for a model or benchmark
- You're looking for the original paper behind a method you're implementing