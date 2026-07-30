# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Layout: multi-context

This repo has three contexts: `apps/api` (Rust backend), `apps/web` (React/Vite frontend), `packages/ui` (shared UI package).

```
/
├── CONTEXT-MAP.md
├── docs/adr/                          ← system-wide decisions
├── apps/
│   ├── api/
│   │   ├── CONTEXT.md
│   │   └── docs/adr/                  ← api-specific decisions
│   └── web/
│       ├── CONTEXT.md
│       └── docs/adr/                  ← web-specific decisions
└── packages/
    └── ui/
        ├── CONTEXT.md
        └── docs/adr/                  ← ui-specific decisions
```

## Before exploring, read these

- **`CONTEXT-MAP.md`** at the repo root — points at one `CONTEXT.md` per context. Read each one relevant to the topic.
- **`docs/adr/`** at the repo root for system-wide decisions, plus `apps/api/docs/adr/`, `apps/web/docs/adr/`, `packages/ui/docs/adr/` for context-scoped decisions touching the area you're about to work in.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in the relevant `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders) — but worth reopening because…_
