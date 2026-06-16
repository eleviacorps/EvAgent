---
name: backend-patterns
domain: coding
version: 1
trigger_patterns:
  - "backend architecture"
  - "server patterns"
  - "microservices"
  - "monolith vs microservices"
applicable_agents:
  - architect
  - planner
  - code-reviewer
---
# Backend Architecture Patterns

## Steps
1. Identify domain boundaries and choose between monolithic, modular, or microservices architecture
2. Apply layered architecture: Controller → Service → Repository → Data
3. Implement middleware pipeline (auth, logging, error handling, rate limiting)
4. Choose communication pattern: sync (REST/gRPC) vs async (message queues/events)
5. Design data flow with proper caching strategy (in-memory, Redis, CDN)
6. Implement health checks, graceful shutdown, and observability

## Examples
- Modular monolith with well-bounded modules communicating via interfaces
- Event-driven microservices with Kafka/RabbitMQ for cross-service communication
- CQRS pattern separating read and write models for high-traffic systems

## Anti-patterns
- Premature microservices decomposition
- Shared databases between services (tight coupling)
- Synchronous call chains that increase latency
- Ignoring distributed transactions and eventual consistency
