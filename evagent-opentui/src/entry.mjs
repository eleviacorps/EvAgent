// EvAgent TUI — JS-free bootloader
// This file has NO JSX, so Bun won't try to resolve JSX runtime.
// It registers the SolidJS transform plugin first, then loads the TSX app.

import { ensureSolidTransformPlugin } from "@opentui/solid/bun-plugin"

// Register the SolidJS transform plugin before any JSX code loads
ensureSolidTransformPlugin()

// Now dynamically import the TUI app (plugin will transform its JSX)
const { render } = await import("@opentui/solid")
const { createCliRenderer, RGBA } = await import("@opentui/core")
const { createDefaultOpenTuiKeymap } = await import("@opentui/keymap/opentui")
const solid = await import("solid-js")

// Build color constants
const C = {
  bg: RGBA.fromInts(10,10,10),
  panel: RGBA.fromInts(20,20,20),
  el: RGBA.fromInts(30,30,30),
  border: RGBA.fromInts(40,40,40),
  text: RGBA.fromInts(220,220,220),
  muted: RGBA.fromInts(128,128,128),
  blue: RGBA.fromInts(92,156,245),
  green: RGBA.fromInts(127,216,143),
  red: RGBA.fromInts(224,108,117),
}

const empty = { topLeft:"", bottomLeft:"", vertical:"", topRight:"", bottomRight:"", horizontal:" ", bottomT:"", topT:"", cross:"", leftT:"", rightT:"" }
const border = { border:["left","right"], customBorderChars:{...empty, vertical:"┃"} }

const { createSignal, createEffect, For, Show, onMount, onCleanup } = solid

const WS = "ws://127.0.0.1:9753/ws"

function App() {
  const [msgs, setMsgs] = createSignal([])
  const [inp, setInp] = createSignal("")
  const [connected, setConnected] = createSignal(false)
  let ws = null
  let sv = null

  function add(r, c) { setMsgs(p => [...p, {r, c}]) }

  function send() {
    const t = inp().trim()
    if (!t || !ws || ws.readyState !== WebSocket.OPEN) return
    add("user", t)
    ws.send(JSON.stringify({type:"DispatchTask", goal:t, context:null, domain:"general"}))
    setInp("")
  }

  onMount(() => {
    ws = new WebSocket(WS)
    ws.onopen = () => {
      setConnected(true)
      add("system", "Connected")
      ws.send(JSON.stringify({type:"Ping"}))
    }
    ws.onmessage = e => {
      try {
        const m = JSON.parse(e.data)
        if (m.type === "DispatchResult" && m.aggregated) {
          add("assistant", m.aggregated)
          if (sv) setTimeout(() => sv.scrollToEnd?.(), 50)
        }
        if (m.type === "Error") add("system", m.message || "Error")
      } catch {}
    }
    ws.onclose = () => { setConnected(false) }
  })
  onCleanup(() => ws?.close())

  // Use the runtime API to build the UI without JSX
  // Since we're using the plugin, we can use a simple approach:
  // Use the render function with a component

  return {
    type: "fragment",
    children: [
      {
        type: "box",
        props: { bg: C.bg, width: "100%", height: "100%", flexDirection: "column" },
        children: [
          { type: "box", props: { bg: C.panel, width: "100%", height: 1, paddingLeft: 2, alignItems: "center" },
            children: [
              { type: "text", props: { fg: C.blue, bold: true }, children: "EVAGENT" },
              { type: "text", props: { fg: C.muted }, children: "  ●  " },
              { type: "text", props: { fg: connected() ? C.green : C.red }, children: "● " + (connected() ? "Online" : "Offline") },
            ]
          },
        ]
      }
    ]
  }
}

// Bootstrap
async function main() {
  const renderer = await createCliRenderer({
    targetFps: 60,
    exitOnCtrlC: true,
    useMouse: true,
  })
  const keymap = createDefaultOpenTuiKeymap(renderer)
  await render(() => solid.createComponent(App, {}), renderer)
}

main().catch(console.error)
