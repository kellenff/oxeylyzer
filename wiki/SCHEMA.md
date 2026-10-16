# Wiki Schema

This file defines how this wiki is structured and maintained. Edit it to
change the rules; `/speckit.wiki.*` commands read it before writing anything.

## Scope
1. Durable knowledge about the Oxeylyzer workspace: how its crates and the
   desktop app actually behave, the decisions behind them and the alternatives
   rejected, external facts the code depends on, and procedures that took real
   effort to work out. Feature and bug artifacts under `specs/` and
   `.specify/` are the primary sources; source files and upstream docs are
   cited when they prove a claim. Transient status is out of scope.

## Page types
| Type | Holds | Example title |
|------|-------|---------------|
| concept | domain ideas and definitions that outlive any feature | "Idempotency keys" |
| decision | choices made, alternatives rejected, and why | "Why SQLite over Postgres" |
| component | how a part of the system actually works | "Auth middleware" |
| reference | distilled external facts (APIs, papers, vendor limits) | "Stripe rate limits" |
| howto | procedures that took effort to figure out | "Local TLS setup" |

## Rules
- Pages live in `pages/`, one topic per page, kebab-case filenames.
- Every synthesized claim cites a source ID from `sources.md` (e.g. `(S003)`).
- Pages cross-reference with relative links: `[title](./other-page.md)`.
- A page that outgrows 600 words is split, and both halves link to each other.
- Conflicting claims are kept side by side under a `> ⚠ conflict:` marker
  until resolved — never silently overwritten.
- Once a disagreement is settled, the marker becomes `> ✔ resolved:` and names
  the settling evidence. Both claims stay visible; only the state changes. Lint
  counts `⚠ conflict:` as open and `✔ resolved:` as closed, so a settled
  question stops being reported without its history being erased.
- Frontmatter per page: `title`, `type`, `sources`, `updated` (ISO date).

## Maintenance workflows
- Grow: `/speckit.wiki.ingest <source>` — the only way knowledge enters.
- Use: `/speckit.wiki.query <question>` — answers come from pages, cited.
- Check: `/speckit.wiki.lint` — drift, orphans, contradictions, staleness.
- Resume: `/speckit.wiki.status` — the session-resume entry point.