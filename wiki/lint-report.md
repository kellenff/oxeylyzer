# Wiki Lint Report — 2026-10-06

No findings.

All six checks pass:

- **index-drift** — 4/4 index link texts match page frontmatter titles exactly; the set of index targets equals the set of files in `pages/`.
- **links** — 11 relative links across 4 pages, 0 broken. Citations use S001, S002, S003; all three are registered, and each page's `sources` frontmatter matches the `Pages touched` cells that name it.
- **orphans** — 0. Inbound page links with INDEX excluded: `archive-path-containment.md` 4, `data-archive-pipeline.md` 3, `testing-zip-extraction-in-crate.md` 2, `why-extraction-aborts-instead-of-skipping.md` 2.
- **contradictions** — 1 marker, resolved in place. `archive-path-containment.md:26` records that `plan.md` and `tasks.md` T002 disagreed on strip-vs-check order, names the S003 and S002 evidence that settles it, and states that T002 remains uncorrected in the artifact because the ingest guardrail forbids editing spec files.
- **stale** — 0. All four pages `updated: 2026-10-06`. No source re-ingested after any page's `updated` date.
- **citations** — 0 uncited prose paragraphs under `require_citations: true`. All four pages under the 600-word cap (599, 283, 364, 169).

## Mechanical fixes applied

None in this pass. The five findings from the earlier report were semantic, so they were fixed by hand rather than auto-fixed: the conflict marker was restored and closed, six paragraphs gained citations, and S002 was split into two rows with a usable dedup key each. Lint modified no page prose, no marker, and no `sources.md` history.