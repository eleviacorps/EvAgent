---
name: api-design
domain: coding
version: 1
trigger_patterns:
  - "REST API design"
  - "GraphQL schema"
  - "endpoint design"
  - "API best practices"
applicable_agents:
  - architect
  - planner
  - code-reviewer
---
# API Design Patterns

## Steps
1. Identify resources and define RESTful endpoints or GraphQL types
2. Apply consistent naming: plural nouns, kebab-case, version prefix
3. Design request/response schemas with proper status codes
4. Implement pagination, filtering, sorting for list endpoints
5. Add input validation, authentication, rate limiting
6. Document with OpenAPI/Swagger or GraphQL introspection

## Examples
- `GET /api/v1/users` — list with `?page=1&limit=20&sort=name`
- `POST /api/v1/users` — create with 201 response + location header
- GraphQL: type Query { users(page: Int, limit: Int): [User!]! }

## Anti-patterns
- Verb-based endpoints (`/getUsers`) instead of resource-based (`/users`)
- Ignoring idempotency for PUT/DELETE
- Returning 500 for validation errors (use 400)
- Over-fetching / under-fetching without consideration
