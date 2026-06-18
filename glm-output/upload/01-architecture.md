# EVAGENT — Master Build Specification for GLM 5.2

## Project Identity
Build a complete AI agent orchestration system called **EvAgent**. It manages multiple AI sub-agents across domains (coding, research, writing, trading, study, communication, media) with parallel execution, skill injection, smart tools, and persistent memory.

## Technology Stack
- **Core Engine**: Rust (tokio async, reqwest, serde, parking_lot)
- **Terminal UI**: SolidJS + @opentui/solid (terminal rendering, same engine as OpenCode)
- **Web GUI**: Single HTML file with vanilla JS + CSS
- **LLM Provider**: OpenAI-compatible API (deepseek-v4-flash-free via opencode-ai/zen/v1)
- **Persistence**: SQLite via rusqlite
- **Build**: Cargo (Rust) + Bun (TypeScript/JSX)

## Architecture Overview
```
┌─────────────────────────────────────────────────────────┐
│                    Web GUI (HTML/JS)                    │
│              ws://127.0.0.1:9753/ws                     │
├─────────────────────────────────────────────────────────┤
│              OpenTUI Terminal (SolidJS)                 │
│              ws://127.0.0.1:9753/ws                     │
├─────────────────────────────────────────────────────────┤
│                  Rust Core Engine                       │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │
│  │ Intent   │ │ Dispatch │ │ Session  │ │ Permission │  │
│  │ Router   │ │  Engine  │ │ Manager  │ │  Engine    │  │
│  └─────────┘ └──────────┘ └──────────┘ └────────────┘  │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐  │
│  │ Skill   │ │  Agent   │ │  Config  │ │   LLM      │  │
│  │ Loader  │ │ Registry │ │  Manager │ │   Client   │  │
│  └─────────┘ └──────────┘ └──────────┘ └────────────┘  │
└─────────────────────────────────────────────────────────┘
```
