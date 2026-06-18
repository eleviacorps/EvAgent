import { render } from "@opentui/solid"
import {
  createEffect,
  createSignal,
  For,
  Show,
  onMount,
  onCleanup,
} from "solid-js"
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui"

// ── Configuration ──
const WS_URL = "ws://127.0.0.1:9753/ws"

interface ChatMessage {
  role: "user" | "assistant" | "system"
  content: string
}

interface AgentInfo {
  task_id: string
  agent_name: string
  status: string
  progress: string
  tokens_used: number
}

const theme = {
  bg: "#050508" as const,
  surface: "#0a0a12" as const,
  border: "#1a1a2e" as const,
  text: "#d4d4e8" as const,
  muted: "#6b6b8d" as const,
  accent: "#4fc3f7" as const,
  green: "#4ade80" as const,
  red: "#ef4444" as const,
  amber: "#fbbf24" as const,
  panelBg: "#0d0d14" as const,
}

function App() {
  const [messages, setMessages] = createSignal<ChatMessage[]>([])
  const [input, setInput] = createSignal("")
  const [connected, setConnected] = createSignal(false)
  const [agents, setAgents] = createSignal<AgentInfo[]>([])
  const [tokens, setTokens] = createSignal(0)
  let wsRef: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null

  function connect() {
    wsRef = new WebSocket(WS_URL)
    wsRef.onopen = () => {
      setConnected(true)
      addMsg("system", "Connected")
      wsRef!.send(JSON.stringify({ type: "Ping" }))
    }
    wsRef.onmessage = (e) => {
      try {
        handleMsg(JSON.parse(e.data))
      } catch {}
    }
    wsRef.onclose = () => {
      setConnected(false)
      addMsg("system", "Disconnected, reconnecting...")
      scheduleReconnect()
    }
    wsRef.onerror = () => {
      if (wsRef?.readyState !== WebSocket.OPEN) scheduleReconnect()
    }
  }

  function scheduleReconnect() {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    reconnectTimer = setTimeout(connect, 2000)
  }

  function handleMsg(msg: any) {
    switch (msg.type) {
      case "Pong":
        setConnected(true)
        break
      case "DispatchResult":
        if (msg.aggregated) addMsg("assistant", msg.aggregated)
        break
      case "SubAgentUpdate":
        setAgents((prev) => {
          const i = prev.findIndex((a) => a.task_id === msg.task_id)
          const next = i >= 0 ? [...prev] : [...prev, {
            task_id: msg.task_id,
            agent_name: msg.agent_name,
            status: "running",
            progress: msg.progress || "",
            tokens_used: 0,
          }]
          if (i >= 0) {
            next[i] = { ...next[i], status: msg.status, progress: msg.progress, tokens_used: msg.tokens_used }
          }
          return next
        })
        setTokens((t) => t + (msg.tokens_used || 0))
        break
      case "Error":
        addMsg("system", msg.message || "Error")
        break
    }
  }

  function addMsg(role: "user" | "assistant" | "system", content: string) {
    setMessages((prev) => [...prev, { role, content }])
  }

  function sendMessage() {
    const text = input().trim()
    if (!text || !wsRef || wsRef.readyState !== WebSocket.OPEN) return
    addMsg("user", text)
    wsRef.send(JSON.stringify({
      type: "DispatchTask", goal: text, context: null, domain: "general",
    }))
    setInput("")
  }

  onMount(() => connect())
  onCleanup(() => wsRef?.close())

  const messages_list = () => messages()
  const agents_list = () => agents()

  return (
    <box width="100%" height="100%" backgroundColor={theme.bg} flexDirection="column">
      {/* Header */}
      <box width="100%" height={1} backgroundColor={theme.surface} paddingLeft={2}>
        <text fg={theme.accent} bold>EVAGENT</text>
        <text fg={theme.muted}>  ● general  </text>
        <text fg={theme.muted}>{tokens()} tokens</text>
        <text fg={connected() ? theme.green : theme.red} bold>  ●  {connected() ? "Online" : "Offline"}</text>
      </box>

      {/* Body */}
      <box width="100%" height="100%" flexGrow={1} flexDirection="row">
        {/* Sidebar */}
        <box width={28} backgroundColor={theme.surface} paddingLeft={1} paddingRight={1} paddingTop={1}>
          <text fg={theme.accent} bold uppercase>Agents</text>
          <Show when={agents_list().length > 0} fallback={<text fg={theme.muted} italic>  None active</text>}>
            <For each={agents_list()}>
              {(a) => (
                <box>
                  <text fg={a.status === "running" ? theme.accent : theme.green} bold>
                    {a.status === "running" ? "▶" : "✓"} {a.agent_name}
                  </text>
                </box>
              )}
            </For>
          </Show>
        </box>

        {/* Conversation */}
        <box flexGrow={1} flexDirection="column">
          <scrollbox flexGrow={1} paddingLeft={2} paddingRight={2} paddingTop={1}>
            <Show when={messages_list().length > 0} fallback={
              <text fg={theme.muted} italic>Type a message to start.</text>
            }>
              <For each={messages_list()}>
                {(msg) => (
                  <box width="100%" marginBottom={1}>
                    <Show when={msg.role === "user"}>
                      <box backgroundColor={theme.panelBg} paddingX={2} paddingY={0} borderColor={theme.border}>
                        <text fg={theme.accent} bold>You</text>
                      </box>
                      <box backgroundColor={theme.panelBg} paddingLeft={2} paddingRight={2} paddingBottom={1}>
                        <text fg={theme.text}>{msg.content}</text>
                      </box>
                    </Show>
                    <Show when={msg.role === "assistant"}>
                      <box backgroundColor={theme.surface} paddingX={2} paddingY={0} borderColor={theme.border}>
                        <text fg={theme.green} bold>EvAgent</text>
                      </box>
                      <box backgroundColor={theme.surface} paddingLeft={2} paddingRight={2}>
                        <text fg={theme.text}>{msg.content}</text>
                      </box>
                    </Show>
                    <Show when={msg.role === "system"}>
                      <text fg={theme.muted} italic>  {msg.content}</text>
                    </Show>
                  </box>
                )}
              </For>
            </Show>
          </scrollbox>

          {/* Input */}
          <box width="100%" height={3} backgroundColor={theme.surface} paddingLeft={2} paddingRight={2} borderTopColor={theme.border}>
            <box width="100%" height={1} backgroundColor={theme.panelBg} paddingLeft={1} marginTop={1} borderColor={theme.border}>
              <text fg={theme.green} bold>{">"}</text>
              <input
                value={input()}
                onChange={(e: any) => setInput(e.target?.value ?? "")}
                onSubmit={() => sendMessage()}
                placeholder="Type a message..."
                fg={theme.text}
                placeholderFg={theme.muted}
                flexGrow={1}
              />
            </box>
          </box>
        </box>
      </box>
    </box>
  )
}

render(() => <App />)
