---
name: tdd-workflow
domain: coding
version: 1
trigger_patterns:
  - "test-first"
  - "red-green-refactor"
  - "write tests before code"
  - "TDD approach"
applicable_agents:
  - planner
  - code-reviewer
  - e2e-runner
---
# TDD Workflow — Red-Green-Refactor

## Steps
1. **Red** — Write a failing test that defines the desired behavior
2. **Green** — Write the minimal code to make the test pass
3. **Refactor** — Clean up code while keeping tests green
4. Repeat for each incremental behavior

## Examples
- Implementing a sorting function: write test for sorted output first, then implement quicksort, then extract helper methods
- API endpoint: write integration test for 200 response, build route handler, then extract validation middleware

## Anti-patterns
- Writing too many tests at once (lose focus)
- Skipping the refactor step (accumulates tech debt)
- Writing production code before the test (violates the cycle)
- Tests that test implementation details instead of behavior
