# EVAGENT — GLM 5.2 Master Implementation Prompt

## System Role
You are building **EvAgent**, a complete multi-domain AI agent orchestration system. You have access to the full codebase at `D:\Programming\AiProjects\EvAgent\` with the following structure already in place:
- Rust core engine (`evagent-core/`) with WebSocket server, intent router, dispatcher, SQLite storage
- OpenTUI terminal app (`evagent-opentui/`) using SolidJS + `@opentui/solid`
- Web GUI (`evagent-web/index.html`) as single HTML file
- 7 domain directories with 23 agent YAMLs and 43 skill markdowns

## What Already Works
- Core engine starts, binds to port 9753, accepts WebSocket connections
- Intent router classifies prompts into domains (coding, research, etc.)
- LLM direct call works (deepseek-v4-flash-free via reqwest)
- Session management with SQLite persistence
- Agent and skill scanning from domain directories

## What Needs to Be Built/Fixed

### 1. Fix OpenTUI Terminal Rendering
The TUI currently shows a blank screen. The app uses `@opentui/solid` with the SolidJS JSX transform plugin. Files: `evagent-opentui/src/index.tsx`, `evagent-opentui/src/entry.ts`.
- Ensure `ensureSolidTransformPlugin()` is called before importing JSX components
- `createCliRenderer()` must be called explicitly with `{ targetFps: 60, exitOnCtrlC: true, useMouse: true }`
- `render(() => <App />, renderer)` — the renderer object MUST be passed
- The component tree should match the OpenCode dark theme exactly:
  - Header: bg `#141414`, EVAGENT in blue `#5c9cf5`, connection dot
  - Conversation: scrollbox with left-border styled messages (┃)
  - Sidebar: 26 chars wide, dark bg, agent list
  - Input: 3-line height with bordered input box

### 2. Implement Full WebSocket Message Protocol
Both TUI and Web GUI must handle all message types:
- **Send**: `DispatchTask { goal, context, domain }`, `Ping`
- **Receive**: `Pong`, `DispatchResult { session_id, aggregated }`, `SubAgentUpdate { task_id, agent_name, status, progress, tokens_used }`, `Error { message }`

### 3. Parallel Agent Dispatch
The dispatcher currently has mock echo sub-agents. Replace with real parallel LLM calls:
- Use `tokio::task::JoinSet` for parallel execution
- Max 5 concurrent agents (configurable)
- Each sub-agent gets: goal + domain context + injected skills → calls LLM → returns output
- Progress updates sent via broadcast channel during execution
- Aggregator collects all outputs into final response

### 4. Skill Injection
Skills from `domains/*/skills/*.md` are loaded at startup. When an agent is dispatched:
- Load domain-relevant skills from SQLite cache
- Inject skill content into the agent's system prompt
- Skills with matching trigger keywords get auto-injected

### 5. Smart Memory
Implement the three-tier memory system:
- User profile (persistent preferences, environment facts)
- Agent notes (durable cross-session knowledge)
- Conversation history (FTS5-indexed SQLite storage)
- Memory is loaded and injected as context for each dispatch

### 6. Tool System
Each sub-agent has access to tools. The agent can request tool calls during execution:
- File tools: ReadFile, WriteFile, PatchFile, SearchFiles
- Terminal: run commands
- Web: fetch URLs, search
- Memory: read/write memory entries
- Tools are permission-gated and validated before execution

### 7. Web GUI Polish
Improve `evagent-web/index.html`:
- Match the OpenCode dark theme exactly
- Agent cards with progress bars and tool lists
- Activity feed panel showing recent actions
- Responsive layout (hide side panels on narrow screens)
- Auto-scroll to bottom on new messages

## Implementation Order
1. Fix the OpenTUI JSX transform/renderer passing (blocking issue)
2. Implement real parallel agent dispatch with JoinSet
3. Wire skill injection into agent system prompts
4. Add memory storage and retrieval
5. Implement tool execution for sub-agents
6. Polish web GUI to match dark theme
7. Add workflow configurations (fan-out, pipeline, supervisor)

## Key Files
- `evagent-core/src/server.rs` — WebSocket server, dispatch handler, LLM calls
- `evagent-core/src/dispatcher.rs` — Sub-agent execution engine
- `evagent-core/src/intent_router.rs` — Domain routing
- `evagent-core/src/skill_loader.rs` — Skill loading and caching
- `evagent-core/src/agent_registry.rs` — Agent YAML scanning
- `evagent-core/src/session.rs` — SQLite session storage
- `evagent-core/src/permissions.rs` — Tool/action permissions
- `evagent-opentui/src/index.tsx` — TUI components
- `evagent-web/index.html` — Web GUI

## Constraints
- All Rust files must stay under 500 lines each
- The TUI must use `@opentui/solid` with explicit renderer creation (not the default)
- WebSocket port is always `127.0.0.1:9753` with path `/ws`
- LLM API calls use `reqwest` async client (not blocking)
- API key read from `.env` file via `dotenvy`
