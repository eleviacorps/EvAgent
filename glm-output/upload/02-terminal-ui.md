# 02 — Terminal UI Specification (OpenTUI/SolidJS)

## Overview
The TUI uses `@opentui/solid` — the same terminal rendering engine as OpenCode. It renders SolidJS components directly in the terminal. The TUI connects to the Rust core via WebSocket at `ws://127.0.0.1:9753/ws`.

## Bootstrap Sequence
1. Import and call `ensureSolidTransformPlugin()` from `@opentui/solid/bun-plugin`
2. Create a `CliRenderer` with `createCliRenderer({ targetFps: 60, exitOnCtrlC: true, useMouse: true })`
3. Create a keymap with `createDefaultOpenTuiKeymap(renderer)`
4. Call `render(() => <App />, renderer)` passing the renderer
5. The `render()` function from `@opentui/solid` takes the component and renderer

## Component Tree
```
<box bg=#0a0a0a width=100% height=100% flexDirection=column>
  <Header />                          // height=1, bg=#141414
  <box flexGrow=1 flexDirection=row>   // Main body
    <Conversation />                   // flexGrow=1, scrollbox
    <Sidebar />                        // width=26, bg=#141414
  </box>
  <Input />                            // height=3, bg=#141414
</box>
```

## Header
- Fixed height=1, background `#141414`
- Shows "EVAGENT" in blue `#5c9cf5` bold
- Connection status dot: green `#7fd88f` when connected, red `#e06c75` when disconnected
- Token count on the right in muted `#808080`

## Conversation Area (scrollbox)
- Flex-grow, fills remaining space
- `stickyScroll={true}` with `stickyStart="bottom"` — auto-scrolls to newest messages
- Vertical scrollbar with track bg=`#0a0a0a`, fg=`#282828`
- **Messages**:
  - **User messages**: `box border={["left"]}` with `customBorderChars={SplitBorder.customBorderChars}` (┃ character), borderColor=blue `#5c9cf5`, marginTop=1. Content box: paddingTop=1, paddingBottom=1, paddingLeft=2, bg=panel
  - **Assistant messages**: Same left-border style, borderColor=green `#7fd88f`, bg=element `#1e1e1e`
  - **System messages**: No border, paddingLeft=3, muted italic, no background
- Each message type has a separator (blank line or margin)
- Empty state: "Type a message to start." in muted italic

## Sidebar
- Width=26 characters, bg=#141414
- Shows "AGENTS" header in blue bold uppercase
- Active agents listed with icon: ▶ running (cyan), ✓ completed (green), ✗ failed (red)
- Agent name in white
- Empty state: "None" in muted italic

## Input Bar
- Height=3, bg=panel `#141414`
- Contains a bordered input box: bg=background `#0a0a0a`, borderColor=border `#282828`
- Green `>` prompt prefix in `#7fd88f` bold
- Text input with placeholder "Type a message..." in muted
- Text fg=white `#dcdcdc`
- On Enter: sends DispatchTask via WebSocket, clears input

## Color Palette (OpenCode Dark)
```typescript
bg:       #0a0a0a (RGBA 10,10,10)
panel:    #141414 (RGBA 20,20,20)
element:  #1e1e1e (RGBA 30,30,30)
border:   #282828 (RGBA 40,40,40)
text:     #dcdcdc (RGBA 220,220,220)
muted:    #808080 (RGBA 128,128,128)
primary:  #5c9cf5 (RGBA 92,156,245)
green:    #7fd88f (RGBA 127,216,143)
red:      #e06c75 (RGBA 224,108,117)
cyan:     #56b6c2 (RGBA 86,182,194)
```

## WebSocket Message Protocol
**Send** (TUI → Core):
```json
{"type": "DispatchTask", "goal": "user message", "context": null, "domain": "general"}
{"type": "Ping"}
```

**Receive** (Core → TUI):
```json
{"type": "Pong"}
{"type": "DispatchResult", "session_id": "...", "outputs": [...], "aggregated": "response text"}
{"type": "SubAgentUpdate", "task_id": "...", "agent_name": "...", "status": "running|Completed|Failed", "progress": "...", "tokens_used": 0}
{"type": "Error", "message": "error text"}
```
