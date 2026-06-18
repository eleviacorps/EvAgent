#!/usr/bin/env python3
"""Generate all 7 domain directories with domain.yaml + agent YAMLs + SKILL.md files.

Mirrors the agents referenced in evagent-mock-server/server.ts so the mock
server and the on-disk catalog stay in sync.
"""
from pathlib import Path
from textwrap import dedent

ROOT = Path("/home/z/my-project/domains")

# Each domain: (name, patterns, agents, skills)
# agents: list of (name, role, tools, skills, system_prompt)
# skills: list of (name, version, description, triggers, body)
DOMAINS = [
    {
        "name": "coding",
        "patterns": [r"\bcode\b", r"\bbug\b", r"\bfunction\b", r"\bapi\b", r"\brefactor\b", r"\bclass\b", r"\bcrash\b", r"\bimplement\b", r"\bbuild\b", r"\bcompile\b"],
        "agents": [
            ("planner", "planner", ["ReadFile", "ListDirectory", "SearchFiles"], ["api-design", "backend-patterns"], "You break coding tasks into clear sub-steps. Always propose a plan before execution."),
            ("architect", "architect", ["ReadFile", "ListDirectory", "SearchFiles"], ["api-design", "backend-patterns"], "You design the structure of a code change: files, interfaces, data flow."),
            ("code-writer", "executor", ["ReadFile", "WriteFile", "PatchFile", "Terminal"], ["tdd-workflow", "security-review"], "You write clean, idiomatic code. Match the existing style of the file you are editing."),
            ("reviewer", "reviewer", ["ReadFile", "SearchFiles"], ["security-review", "tdd-workflow"], "You review code for correctness, security, and style. Be specific about line numbers."),
        ],
        "skills": [
            ("api-design", 1, "REST API design patterns", ["rest", "api", "endpoint", "route"],
             "## Resource Naming\n- Use plural nouns: /users, /orders\n- Nest with hierarchy: /users/:id/orders\n- Use kebab-case for multi-word: /order-items\n\n## Status Codes\n- 200: Success\n- 201: Created\n- 400: Bad Request\n- 404: Not Found\n- 500: Server Error\n\n## Versioning\n- Prefer /v1/ prefix in URL\n- Or use Accept header content negotiation"),
            ("backend-patterns", 1, "Common backend service patterns", ["service", "backend", "server", "handler"],
             "## Service Layout\n- /handlers — HTTP handlers\n- /services — business logic\n- /repositories — data access\n- /models — domain types\n\n## Error Handling\n- Wrap domain errors in a Result type\n- Convert to HTTP status at the handler boundary\n- Log at the service boundary\n\n## Configuration\n- Read from env at startup\n- Validate before serving traffic\n- Fail fast on missing required config"),
            ("tdd-workflow", 1, "Test-driven development workflow", ["test", "tdd", "spec", "assert"],
             "## TDD Cycle\n1. Red — write a failing test\n2. Green — write the minimum code to pass\n3. Refactor — improve without changing behavior\n\n## Naming\n- Test files: *_test.go, *_spec.rb, *.test.ts\n- One behavior per test\n- Describe the behavior in the test name, not the implementation\n\n## Coverage\n- Aim for behavior coverage, not line coverage\n- Test edge cases: empty, single, many, invalid\n- Mock at the boundary, not inside"),
            ("security-review", 1, "Security review checklist for code", ["security", "auth", "secret", "vuln"],
             "## Input Validation\n- Validate all external input at the boundary\n- Use allowlists, not denylists\n- Escape output at the render boundary\n\n## Authentication\n- Hash passwords with bcrypt/argon2\n- Use constant-time comparison for tokens\n- Rotate session IDs after login\n\n## Secrets\n- Never commit secrets — use .env + vault\n- Log redaction for sensitive fields\n- Scope IAM permissions minimally"),
            ("database-migrations", 1, "Database migration patterns", ["migration", "schema", "db", "sql"],
             "## Migration Rules\n- Migrations are append-only — never edit a shipped migration\n- Each migration has up + down\n- Test the down migration in CI\n\n## Schema Changes\n- Additive changes are safe (new column, new table)\n- Drop in two phases: deprecate, then remove\n- Rename = add + backfill + switch + drop\n\n## Backfill\n- Backfill in batches to avoid table locks\n- Run backfill as a separate migration"),
        ],
    },
    {
        "name": "research",
        "patterns": [r"\bresearch\b", r"\bpaper\b", r"\bcite\b", r"\bstudy\b", r"\bsource\b", r"\bliterature\b", r"\bsurvey\b"],
        "agents": [
            ("researcher", "executor", ["WebFetch", "WebSearch", "MemoryWrite"], ["citation-management", "data-extraction"], "You gather sources for a research question. Prefer primary sources and peer-reviewed work."),
            ("source-verifier", "reviewer", ["WebFetch", "WebSearch"], ["source-verification"], "You verify the credibility, recency, and relevance of cited sources."),
            ("summarizer", "specialist", ["LLMComplete"], ["summary-generation"], "You compress a body of research into a faithful, well-structured summary."),
        ],
        "skills": [
            ("citation-management", 1, "Citation formatting and tracking", ["cite", "citation", "reference", "bib"],
             "## Citation Styles\n- APA: Author, A. A. (Year). Title. Journal, Vol(Issue), pages.\n- MLA: Author. Title. Journal, vol., no., Year, pp.\n- Chicago: Author. Title. Journal Vol, no. Issue (Year): pages.\n\n## Tracking\n- Keep a .bib file per project\n- Use DOI when available\n- Note retrieval date for web sources"),
            ("data-extraction", 1, "Extracting structured data from studies", ["extract", "table", "data"],
             "## Extraction Plan\n- Define the schema before reading\n- Two-pass: skim for relevance, then extract\n- Record page numbers / section refs\n\n## Quality Control\n- Have a second reviewer spot-check 10%\n- Flag ambiguous values, don't silently resolve\n- Keep raw extraction + cleaned version separate"),
            ("source-verification", 1, "How to verify a source's credibility", ["verify", "credible", "source"],
             "## Credibility Checklist\n- Author credentials\n- Publication venue (peer-reviewed?)\n- Funding source / conflicts of interest\n- Methodology transparency\n- Replication status\n\n## Recency\n- For fast-moving fields, prefer last 3 years\n- For foundational work, original publication is fine\n- Cite both seminal + recent work"),
            ("literature-review", 1, "Literature review methodology", ["review", "literature", "synthesis"],
             "## Search Strategy\n- Start with keywords, then snowball from references\n- Use multiple databases (Google Scholar, PubMed, arXiv)\n- Record search terms + dates for reproducibility\n\n## Synthesis\n- Group by theme, not by author\n- Note agreements + disagreements explicitly\n- Identify gaps the review does not cover\n\n## Writing\n- Each paragraph makes one claim supported by 2+ sources\n- Use 'while X found ..., Y argues ...' for tensions"),
        ],
    },
    {
        "name": "writing",
        "patterns": [r"\bwrite\b", r"\bessay\b", r"\bblog\b", r"\barticle\b", r"\bcopy\b", r"\bdraft\b", r"\bnarrative\b"],
        "agents": [
            ("outliner", "planner", ["LLMComplete", "MemoryRead"], ["narrative-crafting", "audience-adaptation"], "You produce a structured outline before any prose is written."),
            ("writer", "executor", ["LLMComplete", "MemoryRead"], ["brand-voice", "narrative-crafting"], "You write clear, vivid prose. Match the brand voice and audience."),
            ("editor", "reviewer", ["LLMComplete"], ["seo-content"], "You revise for clarity, flow, and correctness without losing the writer's voice."),
        ],
        "skills": [
            ("brand-voice", 1, "Maintaining brand voice in writing", ["brand", "voice", "tone"],
             "## Voice Dimensions\n- Formality: 1 (casual) → 5 (formal)\n- Warmth: 1 (cool) → 5 (warm)\n- Density: 1 (sparse) → 5 (dense)\n\n## Audit\n- Read 3 existing pieces and score each dimension\n- Define the target range per channel (blog vs. docs vs. tweet)\n- Write a 1-paragraph 'voice north star' as a reference"),
            ("seo-content", 1, "SEO-aware content structure", ["seo", "keyword", "search"],
             "## Keyword Strategy\n- Pick one primary keyword per page\n- Use it in: title, H1, first 100 words, meta description\n- Add 3-5 related terms naturally\n\n## Structure\n- H2 every 200-300 words\n- Short paragraphs (2-4 sentences)\n- Bullet lists for scannable answers\n- Internal links to 2-3 related pages\n\n## Avoid\n- Keyword stuffing\n- Clickbait titles that don't deliver\n- Thin content under 300 words for commercial intent"),
            ("narrative-crafting", 1, "Narrative structure patterns", ["story", "narrative", "arc"],
             "## Story Spine\n- Once upon a time...\n- And every day...\n- Until one day...\n- Because of that... (×3)\n- Finally...\n- And ever since then...\n\n## Hooks\n- Start mid-action (in media res)\n- Open with a question or contradiction\n- Lead with a specific sensory detail\n\n## Endings\n- Mirror the opening image\n- Pay off every setup\n- Land on a concrete image, not abstraction"),
            ("audience-adaptation", 1, "Adapting content for audience", ["audience", "reader", "level"],
             "## Audience Profile\n- Expertise: novice / practitioner / expert\n- Time: 2 min skim / 10 min read / 30 min deep\n- Goal: learn / decide / execute\n\n## Adaptation Levers\n- Vocabulary: jargon density, define-on-first-use\n- Examples: relatable to their daily work\n- Length: inverse to expertise for the same topic\n- Call-to-action: matched to their goal"),
        ],
    },
    {
        "name": "quant-trading",
        "patterns": [r"\btrade\b", r"\bstock\b", r"\bportfolio\b", r"\bbtc\b", r"\bprice\b", r"\bmarket\b", r"\bbacktest\b"],
        "agents": [
            ("market-analyst", "specialist", ["WebFetch", "LLMComplete"], ["technical-analysis", "market-regime"], "You analyze market structure: trend, volatility, regime."),
            ("risk-calc", "reviewer", ["LLMComplete"], ["risk-calculation", "backtesting-methodology"], "You quantify risk: VaR, drawdown, position sizing."),
            ("strategy-builder", "executor", ["LLMComplete", "Terminal"], ["backtesting-methodology", "technical-analysis"], "You implement trading strategies as testable code."),
        ],
        "skills": [
            ("technical-analysis", 1, "Technical analysis indicators", ["indicator", "rsi", "macd", "ma"],
             "## Trend Indicators\n- SMA / EMA — direction\n- MACD — momentum + trend\n- ADX — trend strength\n\n## Oscillators\n- RSI (>70 overbought, <30 oversold)\n- Stochastic\n- CCI\n\n## Volatility\n- Bollinger Bands — 20-period SMA ± 2σ\n- ATR — average true range\n- Implied vol from options chain\n\n## Pitfalls\n- Don't optimize parameters to past data (curve fitting)\n- Use walk-forward analysis\n- Confirm with multiple timeframes"),
            ("risk-calculation", 1, "Risk metrics for trading", ["risk", "var", "drawdown", "sharpe"],
             "## Position Sizing\n- Kelly: f* = (bp - q) / b where b = win/loss ratio\n- Fractional Kelly (¼) is more robust in practice\n- Fixed fractional: risk 1-2% of equity per trade\n\n## Portfolio Risk\n- VaR: 1-day 95% VaR = 1.65 × σ × portfolio_value\n- CVaR (expected shortfall) — average loss beyond VaR\n- Max drawdown — peak-to-trough decline\n\n## Risk-Adjusted Return\n- Sharpe = (return - rf) / σ\n- Sortino = (return - rf) / downside_σ\n- Calmar = annual_return / max_drawdown"),
            ("backtesting-methodology", 1, "How to backtest without lying", ["backtest", "test", "simulate"],
             "## Data\n- Use survivorship-bias-free data\n- Adjust for splits + dividends\n- Mind look-ahead bias in fundamentals\n\n## Methodology\n- In-sample / out-of-sample split (70/30)\n- Walk-forward analysis for parameter stability\n- Monte Carlo on trade order to estimate worst case\n\n## Reporting\n- Always show: CAGR, max DD, Sharpe, # trades, % winners\n- Show equity curve with both periods marked\n- Disclose transaction cost assumptions"),
            ("market-regime", 1, "Detecting market regimes", ["regime", "bull", "bear", "volatility"],
             "## Regime Types\n- Trending up: 50MA > 200MA, ADX > 25\n- Trending down: 50MA < 200MA, ADX > 25\n- Range-bound: ADX < 20, price oscillating\n- High-vol: VIX > 30 or ATR > 2× 90-day avg\n\n## Detection\n- Hidden Markov Models on returns + volatility\n- Or rule-based: simple and explainable\n\n## Application\n- Trend strategies fail in ranges; range strategies fail in trends\n- Size down in high-vol regimes\n- Carry strategies thrive in stable regimes"),
        ],
    },
    {
        "name": "media",
        "patterns": [r"\bvideo\b", r"\baudio\b", r"\bimage\b", r"\bedit\b", r"\brender\b", r"\bcolor\b", r"\bmix\b"],
        "agents": [
            ("asset-loader", "planner", ["ReadFile", "ListDirectory"], ["asset-management"], "You locate and validate source assets before any editing begins."),
            ("editor", "executor", ["Terminal", "PythonCode"], ["audio-mixing", "color-grading"], "You perform the actual cuts, transitions, and grading."),
            ("renderer", "specialist", ["Terminal"], ["motion-design"], "You export the final deliverable in the right format and codec."),
        ],
        "skills": [
            ("color-grading", 1, "Color grading fundamentals", ["color", "grade", "luts"],
             "## Workflow\n1. Balance: neutralize white balance + exposure\n2. Contrast: S-curve in luma\n3. Color: primary wheels (lift, gamma, gain)\n4. Look: creative LUT or selective hue shifts\n\n## Scopes\n- Waveform: exposure distribution\n- Vectorscope: hue + saturation\n- Parade: RGB channel balance\n\n## Pitfalls\n- Don't crush blacks below 5 IRE for broadcast\n- Skin tones sit on the skin-tone line (vector scope)\n- Grade in a calibrated environment, not on a laptop"),
            ("audio-mixing", 1, "Audio mixing for video/podcast", ["audio", "mix", "loudness"],
             "## Levels\n- Dialogue: -12 to -6 LUFS short-term\n- Music bed: -18 to -24 LUFS under dialogue\n- SFX: peak at -10 dBFS\n\n## Loudness Targets\n- Streaming (Spotify/YouTube): -14 LUFS integrated\n- Broadcast EBU R128: -23 LUFS\n- Podcast: -16 LUFS\n\n## EQ + Dynamics\n- High-pass dialogue at 80 Hz\n- De-ess harsh sibilance (4-8 kHz)\n- Compress dialogue 3:1, attack 10ms, release 100ms"),
            ("motion-design", 1, "Motion design principles", ["motion", "animation", "ease"],
             "## Easing\n- Linear: only for mechanical / steady motion\n- Ease-in: building momentum (object leaving frame)\n- Ease-out: arriving + settling (object entering frame)\n- Ease-in-out: rare, feels weightless\n\n## Timing\n- 200ms: micro interaction\n- 400ms: standard transition\n- 800ms: large element, dramatic\n- Over 1000ms: feels slow unless intentional\n\n## Principles (Disney 12)\n- Squash + stretch\n- Anticipation\n- Staging (one focal point)\n- Slow in / slow out\n- Arcs (organic motion)\n- Secondary action (supports primary)"),
            ("asset-management", 1, "Asset organization for media projects", ["asset", "file", "organize"],
             "## Folder Structure\n```\nproject/\n  01_assets/        (raw, read-only)\n    footage/\n    audio/\n    graphics/\n  02_project/       (project files)\n  03_renders/       (exports)\n  04_archive/       (compressed final + project file)\n```\n\n## Naming\n- date_shot_description.ext → 20250318_interview_take03.mov\n- Sequence: 001_, 002_ (zero-pad for sort)\n- No spaces, no special chars\n\n## Versioning\n- _v01, _v02 suffix on renders\n- Keep only the last 2 versions; archive older"),
        ],
    },
    {
        "name": "communication",
        "patterns": [r"\bemail\b", r"\bmessage\b", r"\bslack\b", r"\btweet\b", r"\bnotif\b", r"\bannounce\b"],
        "agents": [
            ("tone-calibrator", "planner", ["LLMComplete", "MemoryRead"], ["tone-calibration", "audience-adaptation"], "You set the right tone for the message and audience before drafting."),
            ("drafter", "executor", ["LLMComplete"], ["platform-adaptation", "tone-calibration"], "You write the message, respecting length limits and channel conventions."),
            ("reviewer", "reviewer", ["LLMComplete"], ["crisis-communication"], "You review for tone, clarity, and risk — especially in sensitive contexts."),
        ],
        "skills": [
            ("tone-calibration", 1, "Tone calibration for messages", ["tone", "voice", "register"],
             "## Tone Axes\n- Warmth: cold → warm\n- Formality: casual → formal\n- Directness: indirect → blunt\n\n## Defaults by Context\n- Customer issue → warm, formal, indirect (acknowledge feeling first)\n- Internal standup → warm, casual, direct\n- Crisis → warm, formal, direct (no hedging)\n- Sales outreach → warm, casual, direct (specific CTA)\n\n## Audit\n- Read aloud — does it sound like a person?\n- Cut hedging words: 'just', 'I think', 'maybe'\n- Replace 'we regret' with 'I'm sorry' for human warmth"),
            ("crisis-communication", 1, "Crisis comms principles", ["crisis", "outage", "incident"],
             "## First Response (within 1 hour)\n- Acknowledge the issue exists\n- Say what we know so far\n- Say what we're doing\n- Say when we'll update next\n\n## Honesty\n- Never speculate about cause\n- Never minimize impact\n- Never blame a vendor without confirmation\n\n## Cadence\n- Update every 30-60 min during active incident\n- Even if no new info: 'still investigating, next update at HH:MM'\n- Post-incident: detailed RCA within 48 hours\n\n## Audience\n- Customers: impact + what to do\n- Public: brief, factual\n- Internal: technical detail + coordination"),
            ("platform-adaptation", 1, "Adapt message per platform", ["slack", "email", "tweet", "platform"],
             "## Slack\n- Short, scannable\n- Use threads for context\n- Bold key asks\n- Emoji are fine for tone, not for content\n\n## Email\n- Subject line = the ask, not the topic\n- Lead with the ask in the first 2 sentences\n- One ask per email\n- Sign-off matches relationship (not 'Best,' to a teammate)\n\n## Tweet / X\n- 280 chars hard limit\n- Hook in first 8 words\n- One idea per tweet; threads for more\n- No passive voice — it kills engagement\n\n## LinkedIn\n- Long-form OK (300-1500 words)\n- Professional but personal voice\n- End with a question to drive comments"),
        ],
    },
    {
        "name": "study-notes",
        "patterns": [r"\bstudy\b", r"\bnotes\b", r"\bexam\b", r"\blearn\b", r"\bflashcard\b", r"\bmemorize\b"],
        "agents": [
            ("concept-mapper", "planner", ["LLMComplete"], ["concept-mapping"], "You build a concept map showing how ideas connect."),
            ("summarizer", "executor", ["LLMComplete"], ["summary-generation", "active-recall"], "You compress material into structured, reviewable notes."),
            ("quiz-builder", "specialist", ["LLMComplete", "MemoryWrite"], ["active-recall", "spaced-repetition"], "You generate practice questions calibrated to retention."),
        ],
        "skills": [
            ("active-recall", 1, "Active recall study method", ["recall", "quiz", "test"],
             "## Principle\n- Reading notes feels productive but isn't\n- Forcing yourself to retrieve is what builds memory\n- Testing > reviewing\n\n## Practice\n- After reading a section, close the book\n- Write down everything you remember\n- Compare to the source, mark gaps\n- Re-test on gaps the next day\n\n## Question Types\n- Definition: 'What is X?'\n- Application: 'Given Y, what would X predict?'\n- Comparison: 'How does X differ from Z?'\n- Edge case: 'What happens if X is null?'"),
            ("concept-mapping", 1, "Building concept maps", ["map", "diagram", "connect"],
             "## Steps\n1. List all key terms (10-20 max)\n2. Pick the most central concept, place in middle\n3. Add related concepts, connect with labeled edges\n4. Look for cross-links between branches\n\n## Edge Labels\n- 'is-a' (hierarchy)\n- 'has-a' (composition)\n- 'causes' (causal)\n- 'requires' (dependency)\n- 'contradicts' (tension)\n\n## Pitfalls\n- Too many nodes — split into multiple maps\n- Linear chains — look for cross-links\n- No edge labels — they force precision"),
            ("spaced-repetition", 1, "Spaced repetition scheduling", ["spaced", "repeat", "schedule"],
             "## Intervals\n- Day 1 → Day 2 → Day 4 → Day 8 → Day 16 → Day 32\n- After 5 successful recalls, item is 'graduated'\n- On failure, reset to Day 1\n\n## Implementation\n- Anki / RemNote / Mochi handle scheduling for you\n- Or simple: review box 1 daily, box 2 every 2 days, etc.\n\n## Card Design\n- Atomic: one fact per card\n- Cloze deletion for definitions\n- Image occlusion for diagrams\n- Avoid 'yes/no' cards — they don't test retrieval"),
            ("summary-generation", 1, "Generating effective study summaries", ["summary", "notes", "condense"],
             "## Structure\n- One-page maximum\n- Top: 3-5 key takeaways in plain language\n- Middle: definitions + formulas + diagrams\n- Bottom: open questions / gaps\n\n## Compression\n- Replace examples with their pattern: 'X, Y, Z' → 'n items of form W'\n- Replace prose with tables when comparing\n- Replace paragraphs with bullet hierarchies\n\n## Review-ability\n- Each line should be re-readable in <5 seconds\n- Use symbols: →, ∴, ∵, =, ≠, ⊃\n- Color-code by topic, not by importance"),
        ],
    },
]


def write_domain(d):
    base = ROOT / d["name"]
    (base / "agents").mkdir(parents=True, exist_ok=True)
    (base / "skills").mkdir(parents=True, exist_ok=True)

    # domain.yaml
    patterns_yaml = "".join(f"  - {p!r}\n" for p in d["patterns"])
    agents_yaml = "".join(f"  - {a[0]}\n" for a in d["agents"])
    skills_yaml = "".join(f"  - {s[0]}\n" for s in d["skills"])
    domain_yaml = (
        f"name: {d['name']}\n"
        f"patterns:\n{patterns_yaml}"
        f"agents:\n{agents_yaml}"
        f"skills:\n{skills_yaml}"
        f"priority: 1\n"
    )
    (base / "domain.yaml").write_text(domain_yaml)

    # agents
    for name, role, tools, skills, system_prompt in d["agents"]:
        tools_yaml = "".join(f"  - {t}\n" for t in tools)
        skills_yaml = "".join(f"  - {sk}\n" for sk in skills)
        sp_indented = system_prompt.replace("\n", "\n  ")
        agent_yaml = (
            f"name: {name}\n"
            f"role: {role}\n"
            f"domain: {d['name']}\n"
            f"tools:\n{tools_yaml}"
            f"skills:\n{skills_yaml}"
            f"token_budget: 4096\n"
            f"system_prompt: |\n  {sp_indented}\n"
        )
        (base / "agents" / f"{name}.yaml").write_text(agent_yaml)

    # skills
    for name, version, description, triggers, body in d["skills"]:
        triggers_yaml = "".join(f"  - {t}\n" for t in triggers)
        md = (
            "---\n"
            f"name: {name}\n"
            f"version: {version}\n"
            f"domain: {d['name']}\n"
            f"description: {description}\n"
            f"triggers:\n{triggers_yaml}"
            "---\n\n"
            f"# {name.replace('-', ' ').title()}\n\n"
            f"{body}\n"
        )
        (base / "skills" / f"{name}.md").write_text(md)


def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    total_agents = 0
    total_skills = 0
    for d in DOMAINS:
        write_domain(d)
        total_agents += len(d["agents"])
        total_skills += len(d["skills"])
        print(f"  {d['name']:15s} — {len(d['agents'])} agents, {len(d['skills'])} skills")
    print(f"\nTotal: {total_agents} agents, {total_skills} skills across {len(DOMAINS)} domains")


if __name__ == "__main__":
    main()
