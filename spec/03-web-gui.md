# 03 — Web GUI Specification (HTML/CSS/JS)

## Overview
A single HTML file with embedded CSS and JavaScript that connects to the Rust core via WebSocket at `ws://127.0.0.1:9753/ws`. No build step required — open directly in browser.

## Layout Structure
```
┌─────────────────────────────────────────────────────┐
│ Header (EVAGENT brand, domain, tokens, cost, conn)  │
├──────────┬──────────────────────────────┬───────────┤
│ Left     │      Conversation Stream     │  Right    │
│ 220px    │                              │  200px    │
│          │                              │           │
│ Agents   │  User messages (right-align) │ Activity  │
│ Info     │  AI responses (left-align)   │ Feed      │
│          │  System messages (center)    │           │
│          │  Agent cards (full width)    │           │
├──────────┴──────────────────────────────┴───────────┤
│ Input: > Type a message...                          │
└─────────────────────────────────────────────────────┘
```

## CSS Design (Dark Theme)
- Background: `#050508`, Surfaces: `#0a0a12`, `#0d0d1a`
- Borders: `#1a1a2e`, Text: `#d4d4e8`, Muted: `#6b6b8d`
- Accent (primary): `#4fc3f7` (cyan-blue)
- Success: `#4ade80` (green), Error: `#ef4444` (red), Warning: `#fbbf24` (amber)
- Font: `'JetBrains Mono', 'Fira Code', monospace`, 14px base
- Thin scrollbar: 6px width, `#1a1a2e` thumb

## Header
- Fixed top bar, `#0a0a12` background
- Left: EVAGENT brand in accent bold, domain indicator
- Right: token count, cost, runtime, connection status dot

## Conversation Stream
- Flexible area, fills remaining height
- Messages with rounded corners (6px), max-width 85%
- **User messages**: Right-aligned, `#1a1a2e` bg, `#2a2a4e` border
- **Assistant responses**: Left-aligned, `#0d0d1a` bg, `#1a1a3e` border
- **System messages**: Center-aligned, muted italic, no bg/border
- **Agent cards**: Full-width bordered cards with header (name + status) and body (tools, diff, progress)

## Agent Card
```
┌─────────────────────────────────────┐
│ ▶ Code Writer               ● done │
├─────────────────────────────────────┤
│ Editing engine.py                   │
│ ━━━━━━━━━━━━━━━━━━━━━━━━ 100%      │
│ Tools: ReadFile, EditFile           │
│ +28 -6 lines                        │
└─────────────────────────────────────┘
```
- Card header: accent name left, green status dot right
- Card body: progress text, progress bar (2px height, accent fill), tool list, diff summary

## Left Panel (220px)
- Agents section with running/completed/failed icons
- Info section: agent count, tokens, domain

## Right Panel (200px)
- Activity feed showing recent tool calls and events
- Each entry: icon + name + target + timestamp

## Input Bar
- Fixed bottom bar, `#0a0a12` background
- Bordered input: `#0d0d1a` bg, `#1a1a2e` border, 8px radius
- Green `>` prompt prefix
- Placeholder "Type a message..." in muted italic
- On Enter: sends DispatchTask, clears input, scrolls chat

## WebSocket Connection
- Auto-connect on page load, reconnect with 2s backoff
- States: Connecting (amber dot), Connected (green dot), Disconnected (red dot)
- Message handling mirrors the TUI protocol
- On DispatchResult: add assistant message, scroll to bottom
- On SubAgentUpdate: update agent list, add card on completion

## Responsive Behavior
- Below 800px: hide left and right panels, conversation fills full width
- Messages truncate at reasonable lengths with ellipsis
- Activity feed items trim long paths
