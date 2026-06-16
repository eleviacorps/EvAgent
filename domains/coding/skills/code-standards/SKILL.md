---
name: code-standards
domain: coding
version: 1
trigger_patterns:
  - "coding standards"
  - "code style"
  - "best practices"
  - "code quality"
  - "clean code"
applicable_agents:
  - code-reviewer
  - planner
  - architect
---
# Code Standards — Language-Agnostic

## Steps
1. **Naming** — Use descriptive names; booleans read as predicates; classes are nouns, functions are verbs
2. **Structure** — Keep functions small (≤20 lines); single responsibility per module/class
3. **Comments** — Explain WHY, not WHAT; prefer self-documenting code; update comments when code changes
4. **Error handling** — Use specific exceptions; never swallow exceptions silently; fail fast
5. **Testing** — Cover happy path, edge cases, and error conditions; test behavior not implementation
6. **Dependencies** — Minimize external deps; pin versions; regularly audit for updates
7. **Formatting** — Use automated formatters (Prettier, Black, gofmt); consistent style enforced by CI

## Examples
- Good: `isActive`, `getUserName()`, `class PaymentProcessor`
- Bad: `flag`, `process()`, `class Utils`
- Error handling: raise specific exceptions rather than returning error codes or null

## Anti-patterns
- God classes / long functions doing many things
- Copy-paste reuse (DRY — Don't Repeat Yourself)
- TODO comments left indefinitely
- Ignoring linter warnings
