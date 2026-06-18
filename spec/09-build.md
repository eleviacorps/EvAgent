# 09 — Build Instructions

## Prerequisites
- Rust (latest stable via rustup)
- Bun (v1.3+)
- Windows 10+ (or any OS with git-bash)

## Building the Core
```bash
cd evagent-core
cargo build --release
cargo run -- start        # Start on default port 9753
cargo run -- start --port 9760  # Custom port
```

## Building the OpenTUI Terminal
```bash
cd evagent-opentui
bun install
bun src/index.tsx         # Run TUI
```

## Building the Web GUI
```bash
# No build step — open evagent-web/index.html in browser
```

## Project Structure
```
evagent/
├── evagent-core/          # Rust core engine
│   ├── src/
│   │   ├── main.rs        # CLI entry, command routing
│   │   ├── server.rs      # WebSocket server, HTTP routes, LLM calls
│   │   ├── config.rs      # Configuration loading
│   │   ├── models.rs      # Shared data types
│   │   ├── intent_router.rs  # Intent classification
│   │   ├── dispatcher.rs  # Sub-agent dispatch engine
│   │   ├── agent_registry.rs # Agent YAML scanning
│   │   ├── skill_loader.rs   # SKILL.md parsing
│   │   ├── session.rs     # Session management (SQLite)
│   │   ├── permissions.rs # Tool/action permissions
│   │   └── errors.rs      # Error types
│   └── Cargo.toml
├── evagent-opentui/       # OpenTUI terminal app
│   ├── src/
│   │   └── index.tsx      # Main TUI component
│   ├── entry.ts           # Bootstrap with plugin init
│   └── package.json
├── evagent-web/           # Web GUI
│   └── index.html         # Single HTML file
├── domains/               # Domain definitions
│   ├── coding/agents/     # Agent YAML files
│   ├── coding/skills/     # Skill markdown files
│   ├── research/agents/
│   ├── research/skills/
│   └── ... (7 domains)
├── evagent.bat            # One-command launcher
└── spec/                  # Specification docs
```

## Config (config.yaml)
```yaml
core:
  max_concurrent_agents: 5
  default_timeout_secs: 120
store:
  db_path: "hermes.db"
  session_ttl_days: 30
  archive_after_days: 7
dispatch:
  max_concurrent: 5
  default_token_budget: 4096
security:
  default_network_access: false
server:
  host: "127.0.0.1"
  port: 9753
routing:
  min_confidence: 0.6
```

## Environment Variables (.env)
```
EVAGENT_API_KEY=sk-...       # API key for LLM
EVAGENT_BASE_URL=https://...  # API base URL
EVAGENT_MODEL=model-name      # Model to use
```
