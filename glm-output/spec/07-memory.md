# 07 — Smart Memory System

## Overview
Memory provides persistent cross-session storage for agents. Three types: User Profile, Agent Notes, and Conversation History.

## Memory Store Types

### 1. User Profile (USER.md)
Stored per-user, injected into every agent prompt.
- Name, role, preferences, communication style
- Environment facts (OS, tools, project structure)
- Recurring corrections and preferences
- Max 2,200 chars

### 2. Agent Notes (MEMORY.md)
Durable facts that survive across sessions.
- Environment details (installed tools, API keys)
- Project conventions and workflows
- Tool quirks and learned workarounds
- Procedural knowledge (saved as "skills" separately)
- Max 2,200 chars

### 3. Conversation History (SQLite FTS5)
- Every session is stored in SQLite with full-text search
- Messages indexed with role + content
- FTS5 used for fast session_search across all history
- Sessions have TTL-based archival (default 30 days)
- Each session has: id, domain, total_tokens, total_cost, summary

## Memory Operations
- `memory_read(key?)` — Returns matching memory entries. Without key, returns all.
- `memory_write(key, content, type)` — Saves to memory (type: "user" | "agent").
- `memory_forget(key)` — Removes a memory entry.
- `session_search(query, limit?)` — FTS5 search across all past sessions.

## Memory Injection
When an agent is dispatched:
1. User profile is injected as system context
2. Agent notes relevant to the domain are injected
3. Past session summaries related to the prompt are retrieved
4. All injected as read-only context in the agent's system prompt
