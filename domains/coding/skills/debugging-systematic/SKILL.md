---
name: debugging-systematic
domain: coding
version: 1
trigger_patterns:
  - "how to debug"
  - "bug investigation"
  - "root cause analysis"
  - "troubleshooting"
applicable_agents:
  - build-error-resolver
  - code-reviewer
  - e2e-runner
---
# Systematic Debugging Methodology

## Steps
1. **Reproduce** — Get a consistent reproduction case; note exact input, environment, and steps
2. **Isolate** — Divide the system; binary search to find the failing component
3. **Hypothesize** — Form a specific, testable hypothesis about the root cause
4. **Test** — Validate the hypothesis with a focused experiment (log, unit test, or minimal repro)
5. **Fix** — Apply the smallest possible change to address root cause
6. **Verify** — Confirm the fix works and doesn't break existing tests
7. **Learn** — Add regression test, document root cause, consider monitoring

## Examples
- Performance regression: bisect git history to find the commit, profile the hot path, add benchmark
- Race condition: add thread sanitizer, reproduce with stress testing, add proper synchronization
- Memory leak: heap dump analysis, check object retention paths, fix unreleased references

## Anti-patterns
- Changing things randomly without a hypothesis (shotgun debugging)
- Fixing symptoms instead of root causes
- Not writing a regression test after fixing
- Debugging by adding print statements to production
