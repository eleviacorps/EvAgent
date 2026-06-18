// EvAgent TUI — debug build
console.error("[evagent] start")

import { ensureSolidTransformPlugin } from "@opentui/solid/bun-plugin"
console.error("[evagent] plugin imported")
ensureSolidTransformPlugin()
console.error("[evagent] plugin registered")

import { render } from "@opentui/solid"
import { createCliRenderer, RGBA } from "@opentui/core"
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui"
import { createSignal, For, Show, onMount, onCleanup } from "solid-js"
console.error("[evagent] all imports done")

const WS = "ws://127.0.0.1:9753/ws"

function App() {
  console.error("[evagent] App() called")
  const [msgs, setMsgs] = createSignal([])
  const [inp, setInp] = createSignal("")
  const [on, setOn] = createSignal(false)
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
    console.error("[evagent] onMount")
    ws = new WebSocket(WS)
    ws.onopen = () => { console.error("[evagent] ws open"); setOn(true) }
    ws.onmessage = e => {
      try {
        const m = JSON.parse(e.data)
        if (m.type === "DispatchResult" && m.aggregated) { add("assistant", m.aggregated); if (sv) setTimeout(() => sv.scrollToEnd?.(), 50) }
        if (m.type === "Error") add("system", m.message || "Error")
      } catch(err) { console.error("[evagent] parse error:", err) }
    }
    ws.onclose = () => { console.error("[evagent] ws close"); setOn(false) }
    ws.onerror = (e) => { console.error("[evagent] ws error") }
  })
  onCleanup(() => { ws?.close() })

  console.error("[evagent] rendering JSX...")

  return (
    <box bg={RGBA.fromInts(8,8,14)} width="100%" height="100%" flexDirection="column">
      <box bg={RGBA.fromInts(18,18,28)} width="100%" height={1} paddingLeft={2} alignItems="center">
        <text fg={RGBA.fromInts(92,156,245)} bold>EVAGENT</text>
        <text fg={RGBA.fromInts(120,120,160)}>  ●  </text>
        <text fg={on ? RGBA.fromInts(80,220,120) : RGBA.fromInts(220,80,80)}>● {on ? "Online" : "Offline"}</text>
      </box>

      <box flexGrow={1} flexDirection="row" minHeight={0}>
        <box flexGrow={1} minHeight={0} paddingBottom={1} paddingLeft={2} paddingRight={1}>
          <scrollbox ref={(r)=>{sv=r}} flexGrow={1} stickyScroll={true} stickyStart="bottom">
            <box height={1}/>
            <Show when={msgs().length > 0} fallback={
              <box paddingLeft={1}><text fg={RGBA.fromInts(120,120,160)} italic>Type a message</text></box>
            }>
              <For each={msgs()}>{m =>
                <box marginTop={1} paddingLeft={1}>
                  <text fg={RGBA.fromInts(200,200,220)}>{m.c}</text>
                </box>
              }</For>
            </Show>
          </scrollbox>
        </box>
      </box>

      <box bg={RGBA.fromInts(18,18,28)} width="100%" height={3} paddingLeft={2} paddingRight={2} alignItems="center">
        <box bg={RGBA.fromInts(8,8,14)} width="100%" height={1} paddingLeft={1} alignItems="center">
          <text fg={RGBA.fromInts(80,220,120)} bold>{">"}</text>
          <input value={inp()} onChange={(e)=>setInp(e.target?.value??"")} onSubmit={send}
            placeholder="Type a message..." fg={RGBA.fromInts(200,200,220)} placeholderFg={RGBA.fromInts(120,120,160)} flexGrow={1}
          />
        </box>
      </box>
    </box>
  )
}

console.error("[evagent] main() start")

async function main() {
  console.error("[evagent] createCliRenderer...")
  const renderer = await createCliRenderer({ targetFps: 30, exitOnCtrlC: true, useMouse: false })
  console.error("[evagent] renderer ok, keymap...")
  const keymap = createDefaultOpenTuiKeymap(renderer)
  console.error("[evagent] keymap ok, render()...")
  await render(() => <App />, renderer)
  console.error("[evagent] render done")
}

main().catch(err => console.error("[evagent] main error:", err))
console.error("[evagent] script end")
