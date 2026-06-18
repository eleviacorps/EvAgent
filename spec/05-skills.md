# 05 — Skill Injection System

## Overview
Skills are reusable knowledge modules injected into agent prompts. They provide domain-specific expertise, code patterns, and conventions. Each skill is a SKILL.md file with YAML frontmatter.

## Skill File Format (SKILL.md)
```markdown
---
name: api-design
version: 1
domain: coding
description: REST API design patterns
triggers: ["rest", "api", "endpoint", "route"]
---

# REST API Design Patterns

## Resource Naming
- Use plural nouns: /users, /orders
- Nest with hierarchy: /users/:id/orders
- Use kebab-case for multi-word: /order-items

## Status Codes
- 200: Success
- 201: Created
- 400: Bad Request
- 404: Not Found
- 500: Server Error
```

## Skill Loading
- Skills are scanned from `domains/*/skills/` at startup (background async task)
- Stored in SQLite with versioning
- Injected into agent system prompts based on domain match
- Skills can have `triggers` — auto-injected when prompt matches trigger keywords

## Skill Categories
| Domain | Example Skills |
|--------|---------------|
| coding | api-design, backend-patterns, tdd-workflow, security-review, database-migrations |
| research | citation-management, data-extraction, source-verification, literature-review |
| writing | brand-voice, seo-content, narrative-crafting, audience-adaptation |
| quant-trading | technical-analysis, risk-calculation, backtesting-methodology, market-regime |
| media | color-grading, audio-mixing, motion-design, asset-management |
| communication | tone-calibration, crisis-communication, platform-adaptation |
| study-notes | active-recall, concept-mapping, spaced-repetition, summary-generation |

## Skill Injection
When an agent is dispatched for a domain, the SkillLoader provides:
1. Domain-level skills (always injected)
2. Trigger-matched skills (if prompt matches trigger keywords)
3. Agent-specific skills (configured per agent)
