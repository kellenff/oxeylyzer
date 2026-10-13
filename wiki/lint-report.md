# Wiki Lint Report — 2026-10-13

| # | Check | Severity | Page | Finding | Suggested fix |
|---|-------|----------|------|---------|---------------|
| 1 | citations | semantic | `probing-crate-behavior-from-a-scratch-crate.md` | Carries an S013-derived pointer — "go one step further and diff the pre-fix and post-fix outputs; see [A/B A Fix With Worktrees And A Scratch Binary]" — but the page declares `sources: [S012]` only and still reads `updated: 2026-10-06`. The S013 ingest added that sentence while deliberately leaving the frontmatter untouched. | **Applied** (follow-up edit, 2026-10-13): added `S013` to `sources` and set `updated: 2026-10-13`. Provenance sweep now clean. |

## Counts per check

| Check | Severity | Findings | Notes |
|-------|----------|----------|-------|
| index-drift | mechanical | 0 | 32 pages on disk, 32 index lines, no duplicates, no dangling targets. Type sections match frontmatter `type` for all 32. |
| links | mechanical | 0 | Every relative `./page.md` target exists; every cited S-id (`S001`–`S013`) is registered in `sources.md`. |
| orphans | structural | 0 | Every page has ≥1 inbound link from another page. Lowest in-degree is 1 (`ab-fix-with-worktrees.md`, `formatting-rust-without-cargo-fmt.md`, `tauri-setup-hook-runs-before-webview.md`). |
| contradictions | semantic | 0 | 0 open `> ⚠ conflict:` markers; all 7 are `> ✔ resolved:`. Pages sharing S013 (`one-scoring-standard`, `scoring-cache-and-layout-cache`, `exact-integer-scoring-is-an-invariant`) assert one consistent standard: `score() == score_with_precision(p) == cache.total_score()`, with `analyze` on `usize::MAX` as the acknowledged exception. |
| stale | semantic | 0 | None older than 90 days (threshold `2026-07-15`; oldest page is `2026-10-06`). No page's own source predates it. |
| citations | semantic | 1 | All substantive prose paragraphs carry S-ids; the one gap was `# 1`, now applied. Declared-vs-cited source sets match on every page. |

## Fixes applied vs. suggested

- **Applied:** 1 — finding `# 1` fixed by follow-up edit (frontmatter `sources` + `updated`). The `index-drift` and `links` mechanical classes were already clean; `INDEX.md` needed no regeneration and no link repair.
- **Suggested:** none outstanding.

## Verification detail

- **Marker sweep:** 7 `✔ resolved` (in `archive-path-containment`, `auto-layout-naming`, `data-version-currency`, `formatting-rust-without-cargo-fmt`, `layout-row-separator-convention`, `platform-root-resolution`, `why-obsolete-rs-stays`), 0 `⚠ conflict`.
- **Word caps:** none over the 600-word limit; largest are `archive-path-containment.md` (596), `data-version-currency.md` (582), `testing-zip-extraction-in-crate.md` (577).
- **Frontmatter:** all 32 pages have `title`, `type`, `sources`, `updated` and open with `---`.
- **Provenance sweep:** cross-referenced every `sources.md` "pages touched" entry against each page's `updated` date; the only mismatch is finding `# 1`.

## Highest-value finding

Finding `# 1` was the only one, and it was a provenance gap rather than a wrong claim: the affected page's content is correct, but it contained S013-derived navigation while declaring only S012. Left unaddressed it would have made the next freshness check misreport the page as untouched by the S013 ingest. It is now resolved.

## Next step

No open findings remain. The wiki is clean: `/speckit.wiki.query <question>` is ready to test what it knows.