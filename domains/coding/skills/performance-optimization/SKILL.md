---
name: performance-optimization
domain: coding
version: 1
trigger_patterns:
  - "performance tuning"
  - "slow code"
  - "optimization"
  - "profiling"
  - "latency improvement"
applicable_agents:
  - code-reviewer
  - architect
  - planner
---
# Performance Optimization

## Steps
1. **Profile first** — Never optimize without data; use profiling tools (cProfile, Chrome DevTools, perf)
2. **Identify bottlenecks** — Focus on the slowest path: CPU, I/O, memory, or network
3. **Algorithmic improvements** — Replace O(n²) with O(n log n); add caching
4. **Reduce allocations** — Pool objects, reuse buffers, avoid unnecessary copies
5. **Optimize I/O** — Batch operations, use async/parallel, connection pooling
6. **Database queries** — Add missing indexes, reduce N+1, use query optimization
7. **Measure again** — Confirm improvement; establish performance regression tests

## Examples
- API latency: add Redis cache for expensive queries, reduce response payload size
- Frontend: lazy load images, code-split routes, debounce search inputs
- Batch processing: use chunked processing with progress tracking

## Anti-patterns
- Optimizing before profiling (premature optimization)
- Micro-optimizations on non-bottleneck code
- Caching without invalidation strategy (stale data)
- Adding complexity for marginal gains
