# Wiki Lint Report — 2026-10-06

Full pass, re-run after fixing finding 1. 28 pages, 11 registered sources, cutoff `updated < 2026-07-08`.

| # | Check | Severity | Page | Finding | Suggested fix |
|---|-------|----------|------|---------|---------------|
| 1 | contradictions | semantic | scoring-cache-and-layout-cache.md | Open `⚠ conflict:` from S010's ingest. T005/SC-006 claim the misleading `TODO: optimize` was replaced by documenting where the scoring cost actually sits. The marker is gone (`410324f`), but no replacement comment exists at the cost site — `score_swap_cached`'s doc comment still reads only "Scores a potential key swap using the cache", and the redundant recompute at the `usage_score` binding is uncommented. Cost is documented in spec and plan, not code | User decision: add the comment the requirement asked for, then resolve the marker. Or amend T005/SC-006 to say the documentation lives in the spec, not the source |
| 2 | contradictions | semantic | exact-integer-scoring-is-an-invariant.md | Internal mismatch: line 17 reads "Three requirements exist only to hold this line" but the table immediately below has **four** rows — FR-001, FR-008, SC-001, SC-005. Two are requirements and two are success criteria, so the count is wrong twice over | Change "Three requirements" to "Four requirements and success criteria". Written by this session's ingest; not auto-fixed because it is prose |
| 3 | citations | semantic | why-coverage-is-scoped-by-measurement.md | The "What was already protected when the work started" table (lines 24-30) lists five behaviors mapped to source files and carries no citation on the heading, the table, or any row. It asserts which behaviors were already covered | Add (S006) to the section heading or the first row. Pre-existing from S006's ingest, never caught because earlier lint passes flagged only newly-added blocks |
| 4 | citations | semantic | swap-moves-two-characters.md | The delta formula block (lines 12-17) and its lead-in "That makes each affected finger's post-swap usage derivable from its cached value in O(1):" carry no citation; the sentence after the block ends with (S010) and the preceding paragraph also ends with (S010), so the claim is sourced but the block itself is ambiguous | Add (S010) to the lead-in sentence so the formula reads as sourced without reading past it. Marginal — flagging for consistency with row 5 |
| 5 | citations | semantic | scoring-cache-and-layout-cache.md | The five-step numbered list (lines 22-32) describing `score_swap_cached` has no citation on the steps; the lead-in paragraph ends with (S011) | Add (S011) to step 3, which is the only step making a claim about current code behavior rather than restating the lead-in. Marginal |
| 6 | citations | semantic | exact-integer-scoring-is-an-invariant.md | The requirements table (lines 19-24) carries no per-row citation; the lead-in ends with (S010) | None required — the lead-in scopes the whole table. Listed for consistency with row 3, which has no scoped lead-in |

## Closed since the first pass

The previous report's finding 1 is **resolved**. `best_found_swap` asserted the chosen swaps match only *when the two scores were already equal*, and on a score mismatch printed `"scores not the same"` and passed; `score_swaps_no_accept` tolerated `SMALLEST_SCORE + 1000`, i.e. `i64::MIN + 1000`, a value `score_with_precision` cannot return short of overflow. Both are fixed in `oxeylyzer-core/src/generate.rs`: the nested `if let`s are removed so neither `None` outcome skips silently, the scores are asserted before the swaps, and the unreachable sentinel is gone.

Both tests passed unchanged against current code, so the fix cost no score — it removed the ability to hide one. 214 tests pass, clippy clean, `cargo bench --no-run` compiles. The marker on `why-obsolete-rs-stays.md` is now `✔ resolved:`, keeping both the original claim and what the code actually did visible.

## Mechanical checks

No mechanical findings. `auto_fix: index-and-links` had nothing to do:

- **index-drift** — clean. 28 pages, 28 index lines, no line pointing at a missing file.
- **links** — clean. No dead relative links; every `S0nn` cited across all pages is registered in `sources.md`, and no registered source is uncited.
- **orphans** — clean. Every page is linked from at least one other page; minimum inbound count is 2 (`layout-shape-is-the-boards-shape.md`, `shipped-data-files-layout.md`, `why-one-shared-naming-rule.md`).
- **stale** — clean. All 28 pages `updated: 2026-10-06`; all 11 sources `Last ingested 2026-10-06`. No page predates its source's re-ingest.
- Frontmatter — all 28 pages carry `title`, `type`, `sources`, `updated`. Types match SCHEMA.md's table; no type outside `concept`, `decision`, `component`, `reference`, `howto`.
- Word cap — no page exceeds `ingest.page_max_words` (600). Largest is `formatting-rust-without-cargo-fmt.md` at ~422.
- Conformance — 7 `✔ resolved:` markers correctly closed, 1 `⚠ conflict:` open (row 1).

Contradiction check was bounded as configured: 110 page pairs sharing a source ID or a link, out of 378 possible. The 7 new scoring pages share S010/S011, so all their pairwise claims were compared. No incompatible claims found among them — the four pages that describe `score_swap_cached` agree that it still recomputes `finger_usage` for both affected fingers and that the delta is unexploited.