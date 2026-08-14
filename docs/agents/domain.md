# Domain documents

Panopticon is a single-context repository. Engineering skills use the root glossary and root decision records.

## Before exploring, read these

- Read `CONTEXT.md` at the repository root.
- Read the ADRs in `docs/adr/` that affect the work area.
- Read `docs/ARCHITECTURE.md` for runtime layers and module ownership.

If a document does not exist, continue without it. Create domain documents only after a real term or decision exists.

## File structure

Current layout:

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-dwm-thumbnails.md
│   └── 0007-runtime-seams.md
└── src/
```

## Use the glossary's vocabulary

Use the terms from `CONTEXT.md` in Issue titles, proposals, hypotheses, and test names. Do not use a synonym that the glossary rejects.

If a required concept is absent, reconsider the term. Record a real glossary gap for `/grill-with-docs`.

## Flag ADR conflicts

If work contradicts an ADR, state the conflict before a change. Do not override the ADR without a recorded decision.

> Contradicts ADR-0007 (runtime seams). Reopen this decision because ...
