import { render, TimeToFirstDraw } from "@opentui/solid"
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui"
import {
  createEffect,
  createSignal,
  For,
  onMount,
  onCleanup,
  Show,
  createMemo,
  batch,
} from "solid-js"
import { createCliRenderer, MouseButton } from "@opentui/core"

// ── Configuration ──
const WS_URL = "ws://127.0.0.1:9753/ws"

// ── Types ──
interface ChatMessage {
  role: "user" | "assistant" | "system"
  content: string
  id: string
}

interface AgentInfo {
  task_id: string
  agent_name: string
  status: string
  progress: string
  tokens_used: number
}

// ── Theme ──
const theme = {
  bg: "#050508" as RGBA,
  surface: "#0a0a12" as RGBA,
  border: "#1a1a2e" as RGBA,
  text: "#d4d4e8" as RGBA,
  muted: "#6b6b8d" as RGBA,
  accent: "#4fc3f7" as RGBA,
  green: "#4ade80" as RGBA,
  red: "#ef4444" as RGBA,
  amber: "#fbbf24" as RGBA,
  panelBg: "#0d0d14" as RGBA,
}

// ── App Component ──
function App() {
  const [messages, setMessages] = createSignal<ChatMessage[]>([])
  const [input, setInput] = createSignal("")
  const [connected, setConnected] = createSignal(false)
  const [agents, setAgents] = createSignal<AgentInfo[]>([])
  const [tokens, setTokens] = createSignal(0)
  const [cost, setCost] = createSignal(0)
  let wsRef: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let scrollBoxRef: any

  function connect() {
    wsRef = new WebSocket(WS_URL)
    wsRef.onopen = () => {
      setConnected(true)
      addMessage("system", "Connected to EvAgent core")
      wsRef!.send(JSON.stringify({ type: "Ping" }))
    }
    wsRef.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data)
        handleMessage(msg)
      } catch (err) {
        // ignore
      }
    }
    wsRef.onclose = () => {
      setConnected(false)
      addMessage("system", "Connection lost. Reconnecting...")
      scheduleReconnect()
    }
    wsRef.onerror = () => {
      if (wsRef?.readyState !== WebSocket.OPEN) {
        scheduleReconnect()
      }
    }
  }

  function scheduleReconnect() {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    reconnectTimer = setTimeout(connect, 2000)
  }

  function handleMessage(msg: any) {
    switch (msg.type) {
      case "Pong":
        setConnected(true)
        break
      case "DispatchResult":
        if (msg.aggregated) {
          addMessage("assistant", msg.aggregated)
          // Auto-scroll
          requestAnimationFrame(() => scrollBoxRef?.scrollToEnd?.())
        }
        break
      case "SubAgentUpdate":
        setAgents((prev) => {
          const idx = prev.findIndex((a) => a.task_id === msg.task_id)
          if (idx >= 0) {
            const next = [...prev]
            next[idx] = { ...next[idx], status: msg.status, progress: msg.progress, tokens_used: msg.tokens_used }
            return next
          }
          return [...prev, {
            task_id: msg.task_id,
            agent_name: msg.agent_name,
            status: msg.status || "running",
            progress: msg.progress || "",
            tokens_used: msg.tokens_used || 0,
          }]
        })
        setTokens((t) => t + (msg.tokens_used || 0))
        if (msg.status === "Completed") {
          addMessage("system", `✅ Agent ${msg.agent_name} completed`)
        }
        break
      case "Error":
        addMessage("system", msg.message || "Error")
        break
    }
  }

  function addMessage(role: "user" | "assistant" | "system", content: string) {
    const id = Math.random().toString(36).slice(2)
    setMessages((prev) => [...prev, { role, content, id }])
  }

  function sendMessage() {
    const text = input().trim()
    if (!text || !wsRef || wsRef.readyState !== WebSocket.OPEN) return
    addMessage("user", text)
    wsRef.send(JSON.stringify({
      type: "DispatchTask",
      goal: text,
      context: null,
      domain: "general",
    }))
    setInput("")
  }

  onMount(() => {
    connect()
  })

  onCleanup(() => {
    wsRef?.close()
  })

  // ── Keyboard handler ──
  const keymap = createDefaultOpenTuiKeymap()

  return (
    <box
      backgroundColor={theme.bg}
      width="100%"
      height="100%"
      padding={0}
    >
      {/* Header */}
      <box
        backgroundColor={theme.surface}
        width="100%"
        height={1}
        paddingLeft={2}
        paddingRight={2}
      >
        <text fg={theme.accent} bold>
          EVAGENT
        </text>
        <text fg={theme.muted}>  ● general  </text>
        <text fg={theme.muted}>
          {tokens()} tokens  ${cost().toFixed(2)}
        </text>
        <text fg={theme.muted}>
          {"  "}
          <text
            fg={connected() ? theme.green : theme.red}
            bold
          >
            ●
          </text>
          {" "}
          {connected() ? "Connected" : "Disconnected"}
        </text>
      </box>

      {/* Main Content */}
      <box width="100%" height="100%" flexGrow={1} flexDirection="row">
        {/* Sidebar */}
        <box
          backgroundColor={theme.surface}
          width={30}
          height="100%"
          paddingLeft={1}
          paddingRight={1}
          paddingTop={1}
        >
          <box flexDirection="column" gap={1}>
            <text fg={theme.accent} bold uppercase>
              Agents
            </text>
            <Show
              when={agents().length > 0}
              fallback={<text fg={theme.muted} italic>No agents active</text>}
            >
              <For each={agents()}>
                {(agent) => (
                  <box>
                    <text
                      fg={
                        agent.status === "running"
                          ? theme.accent
                          : agent.status === "Completed"
                          ? theme.green
                          : theme.red
                      }
                      bold
                    >
                      {agent.status === "running" ? "▶" : agent.status === "Completed" ? "✓" : "✗"}
                    </text>
                    <text fg={theme.text}> {agent.agent_name}</text>
                  </box>
                )}
              </For>
            </Show>

            <box paddingTop={1}>
              <text fg={theme.accent} bold uppercase>
                Info
              </text>
              <box>
                <text fg={theme.muted}>Agents: </text>
                <text fg={theme.text}>{agents().filter((a) => a.status === "Completed").length}/{agents().length}</text>
              </box>
              <box>
                <text fg={theme.muted}>Tokens: </text>
                <text fg={theme.text}>{tokens()}</text>
              </box>
            </box>
          </box>
        </box>

        {/* Conversation */}
        <box flexGrow={1} height="100%" flexDirection="column">
          <scrollbox
            ref={scrollBoxRef}
            flexGrow={1}
            width="100%"
            paddingLeft={2}
            paddingRight={2}
            paddingTop={1}
            gap={1}
          >
            <Show
              when={messages().length > 0}
              fallback={
                <box paddingTop={2}>
                  <text fg={theme.muted} italic>
                    {connected() ? "Type a message to start." : "Connecting to EvAgent core..."}
                  </text>
                </box>
              }
            >
              <For each={messages()}>
                {(msg) => (
                  <box width="100%">
                    <Switch>
                      <Match when={msg.role === "user"}>
                        <box
                          backgroundColor={theme.panelBg}
                          paddingX={2}
                          paddingY={1}
                          borderColor={theme.border}
                        >
                          <text fg={theme.accent} bold>
                            You
                          </text>
                        </box>
                        <box
                          backgroundColor={theme.panelBg}
                          paddingLeft={2}
                          paddingRight={2}
                          paddingBottom={1}
                          borderColor={theme.border}
                        >
                          <text fg={theme.text}>{msg.content}</text>
                        </box>
                      </Match>
                      <Match when={msg.role === "assistant"}>
                        <box
                          backgroundColor={theme.surface}
                          paddingX={2}
                          paddingY={1}
                          borderColor={theme.border}
                        >
                          <text fg={theme.green} bold>
                            EvAgent
                          </text>
                        </box>
                        <box
                          backgroundColor={theme.surface}
                          paddingLeft={2}
                          paddingRight={2}
                          paddingBottom={1}
                          borderColor={theme.border}
                        >
                          <text fg={theme.text}>{msg.content}</text>
                        </box>
                      </Match>
                      <Match when={msg.role === "system"}>
                        <box paddingX={2} paddingY={0}>
                          <text fg={theme.muted} italic>
                            {msg.content}
                          </text>
                        </box>
                      </Match>
                    </Switch>
                  </box>
                )}
              </For>
            </Show>
          </scrollbox>

          {/* Input */}
          <box
            backgroundColor={theme.surface}
            width="100%"
            height={3}
            paddingLeft={2}
            paddingRight={2}
            paddingTop={0}
            borderTopColor={theme.border}
          >
            <box
              backgroundColor={theme.panelBg}
              width="100%"
              height={1}
              paddingLeft={1}
              paddingRight={1}
              marginTop={1}
              borderColor={theme.border}
            >
              <text fg={theme.green} bold>
                {" >"}
              </text>
              <input
                value={input()}
                onChange={(v: string) => setInput(v)}
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

// ── Entry Point ──
render(() => <App />)
