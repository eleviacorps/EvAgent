---
name: database-patterns
domain: coding
version: 1
trigger_patterns:
  - "database schema"
  - "SQL design"
  - "NoSQL patterns"
  - "query optimization"
  - "data modeling"
applicable_agents:
  - architect
  - planner
  - code-reviewer
---
# Database Patterns

## Steps
1. Choose DB type: relational (SQL) vs document (MongoDB) vs graph (Neo4j) vs time-series
2. Normalize schema to 3NF, then selectively denormalize for read performance
3. Design indexes for query patterns (B-tree, composite, partial, covering)
4. Write efficient queries: avoid N+1, use JOINs wisely, limit results
5. Plan migrations with backward-compatible schema changes
6. Implement connection pooling, read replicas, and sharding for scale

## Examples
- E-commerce: products table with category FK, order_items for M:N orders/products
- Time-series: partition by date range, use materialized views for aggregations
- MongoDB: embed subdocuments for one-to-few, reference for one-to-many

## Anti-patterns
- SELECT * in production queries
- Missing indexes on foreign keys and frequently filtered columns
- Storing JSON blobs that should be relational columns
- Making schema changes without migration scripts
