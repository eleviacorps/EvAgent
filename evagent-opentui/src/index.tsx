// EvAgent TUI — based on OpenCode's @opentui/solid rendering
import { ensureSolidTransformPlugin } from "@opentui/solid/bun-plugin"
ensureSolidTransformPlugin()

import { render, TimeToFirstDraw } from "@opentui/solid"
import { createCliRenderer, RGBA } from "@opentui/core"
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui"
import { createSignal, For, Show, onMount, onCleanup } from "solid-js"

const empty = { topLeft:"", bottomLeft:"", vertical:"", topRight:"", bottomRight:"", horizontal:" ", bottomT:"", topT:"", cross:"", leftT:"", rightT:"" }
const border = { border:["left" as const,"right" as const], customBorderChars:{...empty, vertical:"┃"} }

// OpenCode dark theme
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

const WS = "ws://127.0.0.1:9753/ws"

function App() {
  const [msgs, setMsgs] = createSignal<{r:string;c:string}[]>([])
  const [inp, setInp] = createSignal("")
  const [on, setOn] = createSignal(false)
  let ws: WebSocket|null = null
  let sv: any

  function add(r:string,c:string) { setMsgs(p=>[...p,{r,c}]) }
  
  function send() {
    const t = inp().trim()
    if (!t||!ws||ws.readyState!==WebSocket.OPEN) return
    add("user",t)
    ws!.send(JSON.stringify({type:"DispatchTask",goal:t,context:null,domain:"general"}))
    setInp("")
  }

  onMount(()=>{
    ws = new WebSocket(WS)
    ws.onopen = () => { setOn(true); add("system","Connected"); ws!.send(JSON.stringify({type:"Ping"})) }
    ws.onmessage = e => {
      try {
        const m = JSON.parse(e.data)
        if (m.type === "DispatchResult" && m.aggregated) {
          add("assistant", m.aggregated)
          setTimeout(() => sv?.scrollToEnd?.(), 50)
        }
        if (m.type === "Error") add("system", m.message||"Error")
      } catch {}
    }
    ws.onclose = () => { setOn(false); setTimeout(()=>{ws=new WebSocket(WS)},2000) }
  })
  onCleanup(() => ws?.close())

  return (
    <box bg={C.bg} width="100%" height="100%" flexDirection="column">
      {/* Header */}
      <box bg={C.panel} width="100%" height={1} paddingLeft={2} alignItems="center">
        <text fg={C.blue} bold>EVAGENT</text>
        <text fg={C.muted}>  ●  </text>
        <text fg={on ? C.green : C.red}>● {on ? "Online" : "Offline"}</text>
      </box>

      {/* Body */}
      <box flexGrow={1} flexDirection="row" minHeight={0}>
        <box flexGrow={1} minHeight={0} paddingBottom={1} paddingLeft={2} paddingRight={1}>
          <scrollbox ref={(r:any)=>{sv=r}} flexGrow={1} stickyScroll={true} stickyStart="bottom"
            verticalScrollbarOptions={{trackOptions:{bg:C.bg,fg:C.border}}}
          >
            <box height={1}/>
            <Show when={msgs().length > 0} fallback={
              <box paddingLeft={1}><text fg={C.muted} italic>{on ? "Type a message" : "Connecting..."}</text></box>
            }>
              <For each={msgs()}>{m => <>
                <Show when={m.r==="user"}>
                  <box border={["left"]} customBorderChars={border.customBorderChars}
                    borderColor={C.blue} marginTop={1}
                  >
                    <box paddingTop={1} paddingBottom={1} paddingLeft={2} bg={C.panel}>
                      <text fg={C.text}>{m.c}</text>
                    </box>
                  </box>
                </Show>
                <Show when={m.r==="assistant"}>
                  <box border={["left"]} customBorderChars={border.customBorderChars}
                    borderColor={C.green} marginTop={1}
                  >
                    <box paddingTop={1} paddingBottom={1} paddingLeft={2} bg={C.el}>
                      <text fg={C.text}>{m.c}</text>
                    </box>
                  </box>
                </Show>
                <Show when={m.r==="system"}>
                  <box paddingLeft={3} paddingTop={1}>
                    <text fg={C.muted} italic>{m.c}</text>
                  </box>
                </Show>
              </>}</For>
            </Show>
          </scrollbox>
        </box>

        {/* Sidebar */}
        <box bg={C.panel} width={26} paddingLeft={1} paddingRight={1} paddingTop={1}>
          <text fg={C.blue} bold uppercase>Agents</text>
          <text fg={C.muted} italic>  None</text>
        </box>
      </box>

      {/* Input — OpenCode-style 3-line with border */}
      <box bg={C.panel} width="100%" height={3} paddingLeft={2} paddingRight={2} alignItems="center">
        <box bg={C.bg} width="100%" height={1} paddingLeft={1} borderColor={C.border} alignItems="center">
          <text fg={C.green} bold>{">"}</text>
          <input value={inp()} onChange={(e:any)=>setInp(e.target?.value??"")} onSubmit={send}
            placeholder="Type a message..." fg={C.text} placeholderFg={C.muted} flexGrow={1}
          />
        </box>
      </box>
    </box>
  )
}

// ── Bootstrap ──
async function main() {
  const renderer = await createCliRenderer({
    targetFps: 60,
    exitOnCtrlC: true,
    useMouse: true,
  })
  const keymap = createDefaultOpenTuiKeymap(renderer)
  await render(() => <App />, renderer)
}

main().catch(console.error)
