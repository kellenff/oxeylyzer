# Wiki Lint Report — 2026-10-06

Remediation pass over the previous lint report. Config: `lint.stale_after_days: 90`,
`lint.auto_fix: index-and-links`, `ingest.require_citations: true`. 22 pages, 9 sources.

**Remediation surfaced a real defect in shipped code.** Finding 2 below was written as a
question about a test; verifying it required tracing where `Shape` actually comes from,
and the answer is that the naming rule is fed the wrong shape. SC-002 is violated. This is
a code bug, not a wiki problem — the wiki now records it accurately.

| # | Check | Severity | Page | Finding | Status |
|---|-------|----------|------|---------|--------|
| 1 | contradictions | semantic | layout-row-separator-convention.md | S008 vs S009 on the shipped layout census. S009 claimed all 815 are 3×10. | **Resolved.** Re-enumerated from the published `data-files.zip` — 815 `.dof` files, all parsed, 0 unparsed. 809 `[10,10,10]`, 5 `[10,11,10]`, 1 `[10,10,10,2]`. S008 was right, S009 wrong. Also corrected S008: the odd layout is `[10,10,10,2]`, not four rows of ten (`shai/dusk.dof`). Method recorded on the page |
| 2 | contradictions | semantic | auto-layout-naming.md | The four-row board rationale did not reproduce; `skip(10)` starts row 1, not row 0, so the test passes against pre-change code. | **Resolved** — and it led to finding 3 |
| 3 | — | semantic | auto-layout-naming.md | **New, not previously known.** `auto_name` takes `skip` from `board.shape()` — the form factor's fingering table, not the layout's rows. `ansi` declares a 14-key first row for layouts whose first row is 10 keys, so 4 shipped `ansi` layouts get different names: `english/halmak` `shnt`→`,.ae`, `shai/gust` `eiac`→`ydht`, `shai/stand` `stnd`→`yxra`, `shai/stronk` `strn`→`kyma`. **SC-002 violated** — it required all 815 names unchanged. Verified by running the real `libdof` parse path | New page [Layout Shape Is The Board's Shape](./pages/layout-shape-is-the-boards-shape.md). Code fix not attempted — see "Next step" |
| 4 | contradictions | semantic | formatting-rust-without-cargo-fmt.md | Two accounts of what damaged `generate_batch`'s match arm. | **Resolved** — immaterial; the procedure is identical either way. Marker flipped to `✔ resolved:`, both claims kept |
| 5 | contradictions | semantic | data-version-currency.md | Marker open though the prose already settled it: T010 says `Absent`, shipped code says `Stale`. | **Resolved** — settled by S005's shipped behavior. Marker flipped, both claims kept |
| 6 | stale | semantic | sources.md | `dirs.rs` changed in `10ad47f` after S005's ingest row was written. | **Not a real finding.** `Last ingested` is already 2026-10-06, the same date the pages were updated in that same commit — there is no date to bump. Page content already matches current code (line 81 documents the corrected rustdoc). An attempt to annotate the date inline was reverted: prose in a date column breaks the staleness parse |
| 7 | citations | semantic | auto-layout-naming.md | 3 uncited blocks. | **Fixed** — measured blocks now carry (S009) |
| 8 | citations | semantic | layout-shape-is-the-boards-shape.md | Measurement blocks uncited. | **Fixed** |
| 9 | citations | semantic | why-one-shared-naming-rule.md | `**Rejected alternatives:**` table header uncited. | **Fixed** — header now reads "all from S009" |
| 10 | citations | semantic | layout-row-separator-convention.md | New census block uncited. | **Fixed** |

## Checks clean

`index-drift` — 22 pages, all in `INDEX.md`; every target exists; no duplicates. Two
component lines rewritten to describe the pages as they now stand.

`links` — 0 dead relative links; every `(Snnn)` resolves to S001–S009.

`orphans` — 0. Inbound 2–12. The new page is linked from `auto-layout-naming.md`.

`citations` — 0 uncited claims remain. The 25 blocks a naive scan flags are all
`Related:` link footers and tables, which assert nothing.

`stale` — 0. All 22 pages `updated: 2026-10-06`.

`contradictions` — **0 open markers**, up from 4. 6 `✔ resolved:` markers now.

`registry consistency` — all 9 rows match page frontmatter in both directions.

## Why the shape bug survived the test suite

Every `auto_name` test builds its shape by hand: `Shape::from(vec![10,10,10])`,
`[10,10,10,10]`, `[13,12,12]`, `[10,2]`. **No real board produces any of those.**
`board.shape()` yields `[10,10,10,6]` for `ortho` and `[14,14,13,12,8]` for `ansi`. The
tests are correct about the rule and silent about the input, and the one input that
reaches production — `ansi` — is the one no test uses.

## Next step

**Fix the code, not the wiki.** `auto_name` should derive the row from the layout's own
`main` layer widths, not `board.shape()`. Cheapest correct form: pass the layer's first-row
width from the caller that already has it, or compute it from `keys` and the layer at the
call site. A regression test should parse a real `.dof` — `shai/stronk.dof` is `ansi`
and 31 keys — and assert the name, since that is the case that fails today.

Until that is fixed, `/speckit.wiki.query` answers "what shape is this layout?" from
these pages will correctly say: the board's, not the layout's.