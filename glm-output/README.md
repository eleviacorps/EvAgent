# EvAgent

Multi-domain AI agent orchestration system. Rust core engine, OpenTUI/SolidJS terminal, single-file Web GUI, 7 domains with parallel sub-agent dispatch, skill injection, smart memory, and a tool system.

## Quick start (mock server — runs anywhere Bun is installed)

The Rust core requires a Rust toolchain to compile. For end-to-end testing without Rust, a Bun/TypeScript mock server implements the exact same WebSocket protocol:

```bash
cd evagent-mock-server
bun install
bun run server.ts
```

Then open `evagent-web/index.html` in a browser. Send a message — you'll see `SubAgentUpdate` events stream in as mock agents "run", followed by a final `DispatchResult`.

## Full start (real Rust core)

```bash
# 1. Build & start core
cd evagent-core
cargo build --release
cargo run --release -- start            # default port 9753
cargo run --release -- start --port 9760 # custom port

# 2. Open Web GUI (no build step)
open ../evagent-web/index.html

# 3. Run TUI (requires Bun)
cd ../evagent-opentui
bun install
bun src/index.tsx
```

## Configuration

- `config.yaml` — top-level config (server port, dispatch limits, LLM provider, storage).
- `.env` — copy from `.env.example`, fill in `EVAGENT_API_KEY` etc. when switching `llm.provider` from `mock` to `openai-compatible`.
- `domains/<name>/agents/*.yaml` — agent definitions.
- `domains/<name>/skills/*.md` — skill markdown with YAML frontmatter.

## Layout

```
evagent/
├── evagent-core/         # Rust core engine
├── evagent-mock-server/  # Bun/TS mock of the WS protocol (for testing without Rust)
├── evagent-opentui/      # OpenTUI SolidJS terminal
├── evagent-web/          # Single HTML file Web GUI
├── domains/              # 7 domains × agents + skills
├── config.yaml
├── .env.example
├── evagent.bat
└── spec/                 # The 10 spec markdown files
```

## WebSocket protocol

Send (client → core):
```json
{"type": "DispatchTask", "goal": "...", "context": null, "domain": "general"}
{"type": "Ping"}
```

Receive (core → client):
```json
{"type": "Pong"}
{"type": "DispatchResult", "session_id": "...", "outputs": [...], "aggregated": "..."}
{"type": "SubAgentUpdate", "task_id": "...", "agent_name": "...", "status": "running|Completed|Failed", "progress": "...", "tokens_used": 0}
{"type": "Error", "message": "..."}
```

See `spec/` for the full specification.
