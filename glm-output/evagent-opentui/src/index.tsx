/**
 * EvAgent OpenTUI terminal UI.
 *
 * Component tree per spec/02-terminal-ui.md:
 *   <box bg=#0a0a0a width=100% height=100% flexDirection=column>
 *     <Header />
 *     <box flexGrow=1 flexDirection=row>
 *       <Conversation />
 *       <Sidebar />
 *     </box>
 *     <Input />
 *   </box>
 *
 * Connects to ws://127.0.0.1:9753/ws and renders:
 *   - User messages (blue ┃ left-border)
 *   - Assistant messages (green ┃ left-border)
 *   - System messages (muted italic, no border)
 *   - Sub-agent updates in the sidebar with ▶/✓/✗ status icons
 */

import { createSignal, For, Show, onCleanup } from "solid-js";
import { Box, Text, TextInput, ScrollBox, BorderChars, SplitBorder } from "@opentui/solid";

// OpenCode dark palette
const COLORS = {
  bg: "#0a0a0a",
  panel: "#141414",
  element: "#1e1e1e",
  border: "#282828",
  text: "#dcdcdc",
  muted: "#808080",
  primary: "#5c9cf5",
  green: "#7fd88f",
  red: "#e06c75",
  cyan: "#56b6c2",
};

const WS_URL = (() => {
  const env = (process as any)?.env ?? {};
  return env.EVAGENT_WS_URL ?? `ws://127.0.0.1:9753/ws`;
})();

// ---- Shared signals ----
const [connected, setConnected] = createSignal(false);
const [messages, setMessages] = createSignal<Array<{ id: string; role: "user" | "assistant" | "system"; text: string }>>([]);
const [agents, setAgents] = createSignal<Array<{ id: string; name: string; status: string; tokens: number; progress: string }>>([]);
const [tokenCount, setTokenCount] = createSignal(0);

let ws: WebSocket | null = null;
let reconnectDelay = 2000;

function connect() {
  try {
    ws = new WebSocket(WS_URL);
  } catch (e) {
    setTimeout(connect, reconnectDelay);
    return;
  }
  ws.onopen = () => {
    setConnected(true);
    reconnectDelay = 2000;
    pushMessage("system", `Connected to ${WS_URL}`);
  };
  ws.onclose = () => {
    setConnected(false);
    pushMessage("system", `Disconnected — retrying in ${reconnectDelay / 1000}s`);
    setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 1.5, 10000);
  };
  ws.onerror = () => {};
  ws.onmessage = (ev) => {
    let msg: any;
    try { msg = JSON.parse(ev.data); } catch { return; }
    handleServerMessage(msg);
  };
}

function pushMessage(role: "user" | "assistant" | "system", text: string) {
  setMessages((prev) => [...prev, { id: Math.random().toString(36).slice(2), role, text }]);
}

function handleServerMessage(msg: any) {
  if (msg.type === "Pong") return;
  if (msg.type === "Error") {
    pushMessage("system", "⚠ " + msg.message);
    return;
  }
  if (msg.type === "SubAgentUpdate") {
    const { task_id, agent_name, status, progress, tokens_used } = msg;
    setAgents((prev) => {
      const idx = prev.findIndex((a) => a.id === task_id);
      const row = { id: task_id, name: agent_name, status, tokens: tokens_used ?? 0, progress: progress ?? "" };
      if (idx >= 0) { const next = [...prev]; next[idx] = row; return next; }
      return [...prev, row];
    });
    if (status === "Completed" && tokens_used) {
      setTokenCount((t) => t + tokens_used);
    }
    return;
  }
  if (msg.type === "DispatchResult") {
    pushMessage("assistant", msg.aggregated ?? "");
    setAgents([]);
    return;
  }
}

function sendGoal(text: string) {
  if (!ws || ws.readyState !== WebSocket.OPEN) return;
  pushMessage("user", text);
  setAgents([]);
  ws.send(JSON.stringify({ type: "DispatchTask", goal: text, context: null, domain: "general" }));
}

// ---- Components ----

function Header() {
  return (
    <Box bg={COLORS.panel} height={1} width="100%" flexDirection="row" alignItems="center" paddingLeft={1} paddingRight={1}>
      <Text fg={COLORS.primary} bold>EVAGENT</Text>
      <Box flexGrow={1} />
      <Text fg={connected() ? COLORS.green : COLORS.red}>
        {connected() ? "●" : "○"}
      </Text>
      <Text fg={COLORS.muted}> {tokenCount()} tok</Text>
    </Box>
  );
}

function Conversation() {
  return (
    <Box flexGrow={1} flexDirection="column" bg={COLORS.bg}>
      <ScrollBox flexGrow={1} stickyScroll stickyStart="bottom">
        <For each={messages()} fallback={
          <Text fg={COLORS.muted} italic>  Type a message to start.</Text>
        }>
          {(m) => (
            <Show
              when={m.role !== "system"}
              fallback={<Text fg={COLORS.muted} italic>  {m.text}{"\n"}</Text>}
            >
              <Box
                border={["left"]}
                borderColor={m.role === "user" ? COLORS.primary : COLORS.green}
                customBorderChars={SplitBorder.customBorderChars}
                marginTop={1}
                paddingTop={1}
                paddingBottom={1}
                paddingLeft={2}
                bg={m.role === "user" ? COLORS.panel : COLORS.element}
              >
                <Text fg={m.role === "user" ? COLORS.primary : COLORS.green} bold>
                  {m.role === "user" ? "you" : "assistant"}
                </Text>
                <Text fg={COLORS.text}>{"\n"}{m.text}</Text>
              </Box>
            </Show>
          )}
        </For>
      </ScrollBox>
    </Box>
  );
}

function Sidebar() {
  return (
    <Box width={26} bg={COLORS.panel} flexDirection="column" paddingLeft={1} paddingRight={1}>
      <Text fg={COLORS.primary} bold uppercase>AGENTS</Text>
      <For each={agents()} fallback={
        <Text fg={COLORS.muted} italic>None</Text>
      }>
        {(a) => (
          <Box flexDirection="row">
            <Text fg={a.status === "Running" ? COLORS.cyan : a.status === "Completed" ? COLORS.green : COLORS.red}>
              {a.status === "Running" ? "▶" : a.status === "Completed" ? "✓" : "✗"}
            </Text>
            <Text fg={COLORS.text}> {a.name}</Text>
            <Text fg={COLORS.muted}> {a.tokens ? `${a.tokens}t` : ""}</Text>
          </Box>
        )}
      </For>
    </Box>
  );
}

function Input() {
  const [value, setValue] = createSignal("");
  const onSubmit = (v: string) => {
    if (!v.trim()) return;
    sendGoal(v.trim());
    setValue("");
  };
  return (
    <Box height={3} bg={COLORS.panel} flexDirection="row" alignItems="center" paddingLeft={1} paddingRight={1}>
      <Text fg={COLORS.green} bold>{"> "}</Text>
      <Box flexGrow={1} bg={COLORS.bg} border={["all"]} borderColor={COLORS.border}>
        <TextInput
          value={value()}
          onChange={setValue}
          onSubmit={onSubmit}
          placeholder="Type a message..."
          fg={COLORS.text}
        />
      </Box>
    </Box>
  );
}

// ---- App root ----

export default function App() {
  connect();
  onCleanup(() => { try { ws?.close(); } catch {} });

  // Heartbeat
  const heartbeat = setInterval(() => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "Ping" }));
    }
  }, 30000);
  onCleanup(() => clearInterval(heartbeat));

  return (
    <Box bg={COLORS.bg} width="100%" height="100%" flexDirection="column">
      <Header />
      <Box flexGrow={1} flexDirection="row">
        <Conversation />
        <Sidebar />
      </Box>
      <Input />
    </Box>
  );
}
