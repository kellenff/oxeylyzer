# Wiki Lint Report — 2026-10-06

Full pass. 30 pages, 12 registered sources, cutoff `updated < 2026-07-08`. One finding, mechanical, fixed in place.
Page prose, claims, conflict markers, and `sources.md` history were not modified.

| # | Check | Severity | Page | Finding | Suggested fix |
|---|-------|----------|------|---------|---------------|
| 1 | links | mechanical | config-loader-is-the-contract.md | Body cites `(S004, S006)` on the "same mistake, different artifact" claim, but frontmatter declares only `sources: [S006]`. S004 (`specs/002-data-version-upgrade/`) does support the claim — its T005 changed `ensure_data` to branch on `needs_data()`, which is exactly the loader-mismatch pattern described — so the citation is sound and the frontmatter is the incomplete side | **Applied.** Added S004 to the `sources:` list, so the declared set now matches the cited set |

## Mechanical checks — clean

- **index-drift** — all 30 pages present in `INDEX.md`, exactly one line each, no duplicates; every entry sits in the
  section matching its frontmatter `type`; no entries point at missing files. Verified by walking `pages/` against the
  index.
- **links** — every `./x.md` relative link resolves to an existing page; every `S0xx` cited in any page body or
  frontmatter exists in `sources.md` (S001–S012). Row 1 above is the only defect, and it is a declaration gap rather
  than a bad citation. Frontmatter/body agreement is now checked on every page.
- **orphans** — all 30 pages have at least one inbound link from another page. Minimum is 1
  (`exact-integer-scoring-is-an-invariant.md`, `tauri-setup-hook-runs-before-webview.md`).
- **Conformance** — 8 `✔ resolved:` markers closed, 0 `⚠ conflict:` open.
- **Word cap** — no page exceeds `page_max_words` (600). Largest is `archive-path-containment.md` at 596.
- **Frontmatter** — all 30 pages carry `title`, `type`, `sources`, `updated`.

## Semantic checks — clean

- **contradictions** — no `⚠ conflict:` markers remain. The bounded pairwise pass compared only pages sharing a source
  ID or a link, not all 435 pairs: the S010/S011 scoring cluster (8 pages) agrees on `score_swap_cached` recomputing
  `finger_usage` as a two-affected-finger delta that remains unexploited, and the two S012 pages agree the cleaner maps
  corpus char → keystrokes. No incompatible claims.
- **stale** — all 30 pages carry `updated: 2026-10-06`; all 12 sources carry `Last ingested 2026-10-06`. No page
  predates its source's most recent re-ingest. Nothing within the 90-day threshold.
- **citations** — zero prose claims lack a source ID. Verified block by block, excluding fenced code, tables,
  blockquotes, and cross-reference lines, which are structural rather than synthesized claims.

## Verification notes

Two defects in my own checking script produced false results this pass, both caught by hand-verifying before acting:

**Fence tracking was wrong.** The first citation scan stripped ``` lines before testing blocks, so the *contents* of
code fences were classified as prose and seven pages came back "uncited." All seven were fenced code — `enclosed_name()`
Rust, `DataCurrency` enum, the `ensure_data` call tree, the `Cargo.toml` snippet, the `new_usage(f)` formula, a path
literal. Re-run with fence state tracked across blocks, the count is 0. No page was edited on the strength of the
faulty scan; had I trusted it, I would have "fixed" seven pages that were never wrong.

**This pass added two checks the earlier ones lacked** — frontmatter/body citation agreement, and index duplicate and
section-mismatch detection. Row 1 was found only by the first. The earlier reports' clean `links` verdict was therefore
incomplete rather than wrong.

Finding 1 is the only outstanding item, and it is now closed.