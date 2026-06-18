# 08 — Parallel Agentic Workflows

## Overview
EvAgent supports complex multi-agent workflows where agents communicate, hand off tasks, and work in parallel streams.

## Workflow Types

### 1. Fan-Out (Parallel)
One task, multiple agents working simultaneously.
```
User: "Build a full-stack todo app"
    ↓
Planner → breaks into: frontend, backend, database, tests
    ↓
Dispatcher spawns each as parallel sub-agent:
    ├── Frontend Dev   → React components
    ├── Backend Dev    → API routes
    ├── DB Engineer    → Schema + migrations
    └── Tester         → Test suite
    ↓
All run concurrently in JoinSet
    ↓
Aggregator merges results
```

### 2. Pipeline (Sequential)
Output of one agent feeds into the next.
```
User: "Write and publish a blog post"
    ↓
Researcher → gathers sources and outline
    ↓  (passes outline + sources)
Writer → drafts the article
    ↓  (passes draft)
Editor → proofreads and formats
    ↓  (passes final)
Publisher → publishes to configured platforms
```

### 3. Supervisor + Workers
A supervisor agent coordinates worker agents.
```
User: "Analyze our Q4 metrics"
    ↓
Supervisor (Data Analyst)
    ├── Data Collector  → Gather raw data
    ├── Metrics Calc    → Compute KPIs
    ├── Chart Builder   → Generate visualizations
    └── Writer          → Draft analysis report
    ↓
Supervisor reviews all outputs, synthesizes final response
```

### 4. Bidirectional (Consultation)
Agents consult each other during execution.
```
Code Writer encounters ambiguous requirement
    ↓
Consult Architect for design guidance
    ↓
Architect responds with clarification
    ↓
Code Writer continues with clear direction
```

## Workflow Configuration
Workflows are defined in YAML:
```yaml
name: fullstack-app
type: fan-out
agents:
  - name: planner
    role: decompose
  - name: frontend-dev
    depends_on: [planner]
  - name: backend-dev
    depends_on: [planner]
  - name: tester
    depends_on: [frontend-dev, backend-dev]
aggregator: merge
timeout: 300
```
