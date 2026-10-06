# Wiki Lint Report — 2026-10-06

No findings.

All six checks pass across 7 pages.

- **index-drift** — 7/7 index link texts match page frontmatter titles exactly; the set of index targets equals the set of files in `pages/`; one `##` heading per page type (concept 2, decision 3, component 1, howto 1, reference 0).
- **links** — 28 page-to-page links plus 7 index→page lines, 0 broken. Citations name S001–S005, all registered, none unknown. Every page's `sources` frontmatter matches the `Pages touched` cells naming it, no registry row claims a page that omits it, and no page body cites an S-id its frontmatter omits.
- **orphans** — 0. Inbound page links, INDEX excluded, range 2 to 6.
- **contradictions** — 2 markers, both resolved in place. `archive-path-containment.md` records the plan.md vs tasks.md T002 ordering conflict settled against the `zip` 2.4.2 implementation. `data-version-currency.md` records tasks.md T010's "map to `Absent`" against the shipped `dirs.rs` behavior of mapping an unparseable-but-non-empty sentinel to `Stale`. Each names the source artifacts and the code evidence.
- **stale** — 0. All seven pages `updated: 2026-10-06`; age 0 days against `stale_after_days: 90`. No source's `Last ingested` date is later than any page citing it. No claim states a bare test count.
- **citations** — 0 uncited prose paragraphs under `require_citations: true`. All pages under the 600-word cap; largest is 597.

## Resolved since the previous report

The three findings from the last pass are fixed.

**Rows 1–2 — the abort/breach contradiction.** `why-extraction-aborts-instead-of-skipping.md` asserted flatly that "a hostile archive entry aborts extraction … and extraction does not report success," while `archive-path-containment.md` records that `oxeylyzer-data-files/../X.txt` is *accepted* and extracts with `Ok(())`. The abort claim is now scoped to entries the containment check **refuses**, and a new section on that page documents that accepted entries bypass the abort path entirely. The containment page states the same point from its side, so the two reconcile. The page now also cites S002, which the new material depends on.

**Row 3 — bare test count.** "All 34 pre-existing tests pass" is replaced with the claim's actual value: the traversal test uses three `..`, which both orders reject, and that count is what leaves the single-`..` shape undetected. No page states a number that rots.

To keep `archive-path-containment.md` under the 600-word cap while adding two claims, ~30 words of navigation-only prose were trimmed (a forward reference to a heading two lines below, and two closing paragraphs tightened). The page is now 597 words.

## Mechanical fixes applied

None. No index drift, no broken links, no orphans. No page prose, conflict marker, or `sources.md` history was modified by lint itself — the three findings above were semantic and were fixed by hand.