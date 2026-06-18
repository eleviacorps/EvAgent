# 04 — Auto Agent Delegation Engine

## Overview
When a user sends a prompt, the Intent Router classifies it into a domain, then delegates to the appropriate team of sub-agents. Each sub-agent runs as a parallel task with its own LLM call and tool access.

## Intent Routing Pipeline
```
User Prompt
    ↓
1. Regex Pattern Matching — O(n) against registered domain patterns
   - Each domain has 10-24 regex patterns (e.g. \bcode\b → coding)
   - First match wins if confidence > 0.8
    ↓
2. Embedding Cosine Similarity — fallback when no regex match
   - Tokenize prompt and domain patterns into term-frequency vectors
   - Compute cosine similarity, threshold > 0.6
    ↓
3. LLM-based Routing — final fallback
   - Call the configured LLM with: "Classify this prompt into one of: [domains]"
   - Use the model's response as the routing decision
    ↓
Domain + Confidence Score
```

## Domain Configuration
Each domain has:
- `name`: string identifier (e.g. "coding", "research")
- `patterns`: array of regex patterns for matching
- `agents`: array of agent references that can handle this domain
- `skills`: array of skill references available to these agents
- `priority`: number (higher = more important)

## Agent Types
1. **Planner** — Breaks down tasks into sub-steps
2. **Architect** — Designs the approach/solution structure
3. **Executor** — Implements the plan (code-writer, content-writer, etc.)
4. **Reviewer** — Reviews output for quality, correctness
5. **Tester** — Validates the output with tests
6. **Specialist** — Domain-specific agent (market-analyst, video-editor, etc.)

## Delegation Workflow
```
User: "Build a BTC trading dashboard"
    ↓
Router → domain: "quant-trading" (confidence: 0.92)
    ↓
Dispatcher spawns parallel sub-agents:
    ├── Planner      → "Plan the dashboard architecture"
    ├── Architect    → "Design data flow and components"
    ├── Code Writer  → "Implement the dashboard"
    └── Reviewer     → "Review for correctness"
    ↓
Each agent gets:
    - The original prompt + domain context
    - Relevant skills injected into system prompt
    - Tool access (read/write files, search, execute code)
    - Token budget (4096 default)
    ↓
Aggregator collects all outputs
    ↓
Response sent back to user via WebSocket
```

## Parallel Execution
- All sub-agents run concurrently using `tokio::task::JoinSet`
- Maximum concurrent agents: configurable (default 5)
- Progress updates sent via WebSocket for each agent
- On timeout (configurable, default 120s): mark agent as Failed
- On all complete: aggregate results and send DispatchResult

## Sub-Agent Process
Each sub-agent:
1. Receives `DispatchTask { goal, context, domain, agent_name }`
2. Loads relevant skills from SkillLoader
3. Calls the LLM with combined system prompt (domain + skills + goal)
4. Returns `SubAgentOutput { output, tokens_used, wall_clock_ms }`
