---
name: code-data
description: Data patterns — SQL, schema design, migrations, caching, pipelines
tools: [bash, read_file, write_file, edit_file]
---

# Data Patterns

## SQL

- Use parameterized queries — never string interpolation
- Index columns used in WHERE, JOIN, and ORDER BY clauses
- EXPLAIN ANALYZE to understand query plans
- Prefer JOINs over subqueries for readability (optimizer usually handles both)
- Use CTEs (WITH clauses) for complex queries

## Schema Design

- Normalize to 3NF by default; denormalize intentionally for read performance
- Every table gets a primary key (prefer UUID or BIGSERIAL over SERIAL)
- Use foreign keys for referential integrity
- Add created_at/updated_at timestamps to all tables
- Use ENUM types or lookup tables for constrained values

## Migrations

- Forward-only migrations in production (no rollbacks — write compensating migrations)
- One migration per change, named descriptively
- Test migrations on a copy of production data before deploying
- Never modify a migration that's been applied to production

## Caching

- Cache invalidation is the hard problem — prefer TTL-based expiry
- Cache at the right layer: application > database > OS
- Redis for distributed caching, in-memory for single-process
- Cache-aside pattern: check cache → miss → fetch from DB → populate cache

## Data Pipelines

- Idempotent operations — running twice produces the same result
- Use checksums/watermarks to track processing progress
- Handle late-arriving data gracefully
- Log pipeline stages for debugging

## Data Integrity

- Transactions for multi-step operations (ACID guarantees)
- Optimistic locking for low-contention updates
- Validate at system boundaries, trust internal data
- Backup strategy: automated, tested, off-site

## Patterns Learned

*(This section grows as the agent encounters and solves real data problems)*
