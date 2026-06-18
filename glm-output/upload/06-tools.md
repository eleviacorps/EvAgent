# 06 — Smart Tools System

## Overview
Tools are capabilities available to sub-agents during execution. Each tool has a name, description, parameters, and a handler. Tools are invoked by the agent during its execution loop.

## Tool Registry
All tools are registered at startup. Each tool has:
```
name: string           — Unique identifier
description: string    — What the tool does
parameters: JSON Schema — Expected parameters
handler: function      — Implementation
domain: string         — Which domain owns this tool
```

## Core Tools

### File Tools
- **ReadFile(path, offset?, limit?)** — Read file contents with pagination. Lines 1-indexed.
- **WriteFile(path, content)** — Write content to file, overwrites entire file.
- **PatchFile(path, old_string, new_string)** — Find-and-replace with fuzzy matching (9 strategies).
- **SearchFiles(pattern, target, path, file_glob?)** — Regex search inside files or find by name.
- **ListDirectory(path)** — List directory contents sorted by modification time.

### Execution Tools
- **Terminal(command, timeout?)** — Execute shell command. Returns stdout + exit code. 180s default timeout.
- **PythonCode(code)** — Execute Python script inline. 5-minute timeout, 50KB stdout cap.
- **BackgroundProcess(command)** — Start long-lived process, returns session_id for lifecycle management.

### Web Tools
- **WebFetch(url)** — HTTP GET request. Returns body text. For JSON/plain-text endpoints only.
- **WebSearch(query)** — Search web for information. Returns top results.
- **WebExtract(url)** — Extract content from web page via browser.

### Knowledge Tools
- **SkillSearch(query, domain?)** — Search registered skills by content.
- **MemoryRead(key?)** — Read from persistent memory store.
- **MemoryWrite(key, content)** — Write to persistent memory store.
- **SessionSearch(query)** — Search past conversation sessions via FTS5.

### LLM Tools
- **LLMComplete(prompt, model?, max_tokens?)** — Direct LLM call for sub-agent self-queries.
- **LLMEmbed(text)** — Get embedding vector for text (for semantic routing).

## Tool Permissions
Each tool has a permission level:
- `allow` — Available to any agent
- `restrict` — Requires explicit permission (e.g. WriteFile, Terminal)
- `deny` — Blocked by default, configurable per agent profile

## Tool Execution
1. Agent returns structured tool call in its output
2. Dispatcher validates the tool call against permissions
3. Executes the tool handler
4. Returns tool result back to agent for next iteration
5. Agent can chain multiple tool calls (up to max_tool_calls config)
