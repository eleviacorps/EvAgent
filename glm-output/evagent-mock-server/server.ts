/**
 * EvAgent mock WebSocket server.
 *
 * Implements the EXACT same protocol as the Rust core (see
 * evagent-core/src/server.rs and spec/02-terminal-ui.md) so the Web GUI and
 * OpenTUI terminal can be tested end-to-end without compiling Rust.
 *
 * Pipeline per client message:
 *   DispatchTask { goal, context, domain }  →
 *     1. route domain via regex (synthesize from name if domain="general")
 *     2. spawn N mock sub-agents (planner, architect, executor, reviewer)
 *     3. stream SubAgentUpdate { status: Running, progress: "..." } as they "work"
 *     4. on completion: SubAgentUpdate { status: Completed, tokens_used, progress }
 *     5. final DispatchResult { session_id, outputs[], aggregated }
 *
 *   Ping  →  Pong
 *
 * Run:
 *   bun install
 *   bun run server.ts
 *
 * Then open ../evagent-web/index.html in a browser.
 */

import { WebSocketServer, WebSocket } from "ws";
import { randomUUID } from "node:crypto";

const PORT = Number(process.env.EVAGENT_PORT ?? 9753);
const HOST = process.env.EVAGENT_HOST ?? "127.0.0.1";

// ---------- domain + agent catalog (mirrors domains/*/*.yaml) ----------

interface Domain {
  name: string;
  patterns: RegExp[];
  agents: string[];
}

const DOMAINS: Domain[] = [
  {
    name: "coding",
    patterns: [/\bcode\b/i, /\bbug\b/i, /\bfunction\b/i, /\bapi\b/i, /\brefactor\b/i, /\bclass\b/i, /\bcrash\b/i],
    agents: ["planner", "architect", "code-writer", "reviewer"],
  },
  {
    name: "research",
    patterns: [/\bresearch\b/i, /\bpaper\b/i, /\bcite\b/i, /\bstudy\b/i, /\bsource\b/i],
    agents: ["researcher", "source-verifier", "summarizer"],
  },
  {
    name: "writing",
    patterns: [/\bwrite\b/i, /\bessay\b/i, /\bblog\b/i, /\barticle\b/i, /\bcopy\b/i, /\bdraft\b/i],
    agents: ["outliner", "writer", "editor"],
  },
  {
    name: "quant-trading",
    patterns: [/\btrade\b/i, /\bstock\b/i, /\bportfolio\b/i, /\bbtc\b/i, /\bprice\b/i, /\bmarket\b/i],
    agents: ["market-analyst", "risk-calc", "strategy-builder"],
  },
  {
    name: "media",
    patterns: [/\bvideo\b/i, /\baudio\b/i, /\bimage\b/i, /\bedit\b/i, /\brender\b/i],
    agents: ["asset-loader", "editor", "renderer"],
  },
  {
    name: "communication",
    patterns: [/\bemail\b/i, /\bmessage\b/i, /\bslack\b/i, /\btweet\b/i, /\bnotif\b/i],
    agents: ["tone-calibrator", "drafter", "reviewer"],
  },
  {
    name: "study-notes",
    patterns: [/\bstudy\b/i, /\bnotes\b/i, /\bexam\b/i, /\blearn\b/i, /\bflashcard\b/i],
    agents: ["concept-mapper", "summarizer", "quiz-builder"],
  },
];

function routeDomain(prompt: string, hint?: string): { domain: string; confidence: number } {
  if (hint && hint !== "general") {
    return { domain: hint, confidence: 0.95 };
  }
  for (const d of DOMAINS) {
    for (const p of d.patterns) {
      if (p.test(prompt)) return { domain: d.name, confidence: 0.92 };
    }
  }
  // Fallback: tokenize + simple keyword overlap.
  const tokens = prompt.toLowerCase().split(/\W+/).filter((t) => t.length > 2);
  let best: { domain: string; confidence: number } = { domain: "coding", confidence: 0.4 };
  for (const d of DOMAINS) {
    const sample = (d.name + " " + d.agents.join(" ")).toLowerCase();
    const overlap = tokens.filter((t) => sample.includes(t)).length;
    const conf = Math.min(0.85, 0.4 + overlap * 0.1);
    if (conf > best.confidence) best = { domain: d.name, confidence: conf };
  }
  return best;
}

// ---------- mock LLM ----------

function mockLlm(prompt: string, agentName: string, domain: string): { content: string; tokens: number } {
  const stamp = new Date().toISOString().slice(11, 19);
  const content = [
    `## [${agentName}] @ ${stamp}`,
    "",
    `Domain: **${domain}**  |  Agent role: **${agentName}**`,
    "",
    "Received goal:",
    "```",
    prompt.slice(0, 280),
    "```",
    "",
    "Mock plan:",
    "1. Read inputs and confirm scope",
    `2. Apply ${domain}-specific patterns`,
    "3. Produce a draft response",
    "4. Validate against constraints",
    "",
    "Output (mock):",
    `→ Processed "${prompt.slice(0, 60)}${prompt.length > 60 ? "…" : ""}" through the ${agentName} pipeline.`,
    "→ No real model was called. Set llm.provider=\"openai-compatible\" in config.yaml to call a real LLM.",
  ].join("\n");
  const tokens = Math.max(40, Math.round(content.length / 4));
  return { content, tokens };
}

// ---------- protocol types (mirror models.rs) ----------

type ServerMessage =
  | { type: "Pong" }
  | { type: "DispatchResult"; session_id: string; outputs: any[]; aggregated: string }
  | { type: "SubAgentUpdate"; task_id: string; agent_name: string; status: "Running" | "Completed" | "Failed"; progress: string; tokens_used: number }
  | { type: "Error"; message: string };

interface ClientMessage {
  type: "DispatchTask" | "Ping";
  goal?: string;
  context?: any;
  domain?: string;
}

// ---------- WebSocket plumbing ----------

const wss = new WebSocketServer({ host: HOST, port: PORT });

wss.on("listening", () => {
  console.log(`[evagent-mock] listening on ws://${HOST}:${PORT}/ws`);
  console.log(`[evagent-mock] open ../evagent-web/index.html to test`);
});

wss.on("connection", (ws: WebSocket) => {
  const id = randomUUID().slice(0, 8);
  console.log(`[ws:${id}] connected`);

  ws.on("message", (data) => {
    let msg: ClientMessage;
    try {
      msg = JSON.parse(data.toString());
    } catch (e: any) {
      send(ws, { type: "Error", message: `invalid JSON: ${e.message}` });
      return;
    }

    if (msg.type === "Ping") {
      send(ws, { type: "Pong" });
      return;
    }

    if (msg.type === "DispatchTask") {
      const goal = msg.goal ?? "";
      const route = routeDomain(goal, msg.domain);
      const domain = route.domain;
      console.log(`[ws:${id}] DispatchTask domain=${domain} conf=${route.confidence.toFixed(2)} goal="${goal.slice(0, 60)}"`);

      const agents = DOMAINS.find((d) => d.name === domain)?.agents ?? ["general-assistant"];
      const sessionId = randomUUID();
      handleDispatch(ws, { goal, context: msg.context ?? null, domain, agents, sessionId });
      return;
    }

    send(ws, { type: "Error", message: `unknown message type: ${(msg as any).type}` });
  });

  ws.on("close", () => console.log(`[ws:${id}] closed`));
  ws.on("error", (err) => console.log(`[ws:${id}] error: ${err.message}`));
});

async function handleDispatch(
  ws: WebSocket,
  ctx: { goal: string; context: any; domain: string; agents: string[]; sessionId: string }
) {
  const outputs: any[] = [];

  for (const agentName of ctx.agents) {
    const taskId = randomUUID();
    const phases = ["loading skills", "calling LLM", "storing result"];
    for (const phase of phases) {
      send(ws, {
        type: "SubAgentUpdate",
        task_id: taskId,
        agent_name: agentName,
        status: "Running",
        progress: phase,
        tokens_used: 0,
      });
      await sleep(180 + Math.random() * 220);
    }

    const { content, tokens } = mockLlm(ctx.goal, agentName, ctx.domain);
    outputs.push({
      task_id: taskId,
      agent_name: agentName,
      output: content,
      tokens_used: tokens,
      wall_clock_ms: 600 + Math.floor(Math.random() * 400),
      status: "Completed",
    });

    send(ws, {
      type: "SubAgentUpdate",
      task_id: taskId,
      agent_name: agentName,
      status: "Completed",
      progress: `done in ${outputs[outputs.length - 1].wall_clock_ms} ms`,
      tokens_used: tokens,
    });
  }

  const aggregated = outputs
    .map((o) => `## ${o.agent_name}\n\n${o.output}`)
    .join("\n\n---\n\n");

  send(ws, {
    type: "DispatchResult",
    session_id: sessionId,
    outputs,
    aggregated,
  });
}

function send(ws: WebSocket, msg: ServerMessage) {
  if (ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// Tiny HTTP health check on the same port (so curl works too)
// Note: ws library doesn't natively serve HTTP on the same port, so we use
// a separate tiny HTTP server on PORT+1 for /health probes.
import http from "node:http";
http.createServer((req, res) => {
  if (req.url === "/health" || req.url === "/") {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ service: "evagent-mock", status: "ok" }));
  } else {
    res.writeHead(404);
    res.end("not found");
  }
}).listen(PORT + 1, HOST, () => {
  console.log(`[evagent-mock] health check on http://${HOST}:${PORT + 1}/health`);
});

// Graceful shutdown
process.on("SIGINT", () => {
  console.log("\n[evagent-mock] shutting down");
  wss.close();
  process.exit(0);
});
