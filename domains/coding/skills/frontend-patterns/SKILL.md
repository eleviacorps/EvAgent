---
name: frontend-patterns
domain: coding
version: 1
trigger_patterns:
  - "frontend architecture"
  - "UI patterns"
  - "component design"
  - "state management"
applicable_agents:
  - architect
  - planner
  - code-reviewer
---
# Frontend / UI Patterns

## Steps
1. Choose component architecture: atomic design, feature-based, or page-based
2. Implement state management: local (useState) → context → global (Redux/Zustand)
3. Design responsive layouts with mobile-first approach
4. Handle loading, empty, error, and edge-case states for every component
5. Implement performance optimizations: code-splitting, lazy loading, memoization
6. Ensure accessibility (ARIA labels, keyboard navigation, contrast ratios)

## Examples
- Atomic design: atoms (button) → molecules (form field) → organisms (login form)
- Container/Presenter pattern separates data fetching from rendering
- Custom hooks encapsulate reusable stateful logic

## Anti-patterns
- Prop drilling through 5+ levels (use context or composition instead)
- Global state for everything (keep local what's local)
- Premature optimization without profiling
- Ignoring accessibility until the end
