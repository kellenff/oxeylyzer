# Wiki Lint Report — 2026-10-13

Scope: full pass (34 pages, 14 sources, INDEX.md, sources.md, SCHEMA.md).
Config: `wiki/`, auto_fix: `index-and-links`, stale_after_days: 90, require_citations: true, page_max_words: 600.

## Status: all findings resolved

The five findings from the initial pass were applied later the same day.
Re-ran every check; **0 open findings**.

| # | Check | Severity | Page | Finding | Resolution |
|---|-------|----------|------|---------|------------|
| 1 | links | mechanical | sources.md (S014 row) | `archive-path-containment.md` cited S014; row omitted it | applied — appended to S014 `Pages touched` |
| 2 | links | mechanical | sources.md (S014 row) | same for `why-extraction-aborts-instead-of-skipping.md` | applied — appended to S014 |
| 3 | links | mechanical | sources.md (S002, S003 rows) | `zip9-normalizes-paths-that-zip2-refused.md` cited both; neither row listed it | applied — appended to S002 and S003 |
| 4 | links | mechanical | sources.md (S004 row) | `config-loader-is-the-contract.md` cited S004; row omitted it (pre-existing) | applied — appended to S004 |
| 5 | schema | mechanical | SCHEMA.md | `## Scope` was the unset placeholder | applied — scope sentence written |

## Checks run (re-verified after fixes)

| Check | Result |
|-------|--------|
| index-drift | clean — all 34 pages in INDEX, no dead or duplicate index lines |
| links | clean — no `./…md` target missing; every `sources.md` page name exists; registry ↔ frontmatter now fully consistent |
| orphans | clean — every page linked from at least one other page |
| contradictions | clean — 0 `⚠ conflict:` open; 7 pages carry closed `✔ resolved:` markers (`data-version-currency`, `formatting-rust-without-cargo-fmt`, `auto-layout-naming`, `archive-path-containment`, `platform-root-resolution`, `layout-row-separator-convention`, `why-obsolete-rs-stays`) |
| stale | clean — no page older than 90 days; no page predating a re-ingest of a source it cites |
| citations | clean — every page carries ≥1 S-id; no unknown S-id cited; all frontmatter `sources` resolve |
| schema | clean — frontmatter fields present and types in-schema; largest page 501/600 words |

## Suggested (non-blocking)

- `sources.md` rows S007–S013 render as one unwrapped text blob in the raw file (all rows intact and parseable; the source file appears to have lost its newlines there). Cosmetic only — no data lost, registry checks pass. Reformat if the file is ever hand-edited.