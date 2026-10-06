# Wiki Lint Report — 2026-10-06

Findings from the previous pass were resolved between passes: one code fix, one
registry correction, two marker state changes, one citation appended. This
report is the re-run.

| # | Check | Severity | Page | Finding | Suggested fix |
|---|-------|----------|------|---------|---------------|
| 1 | contradictions | semantic | data-version-currency.md | One marker remains open, and it should. `specs/002-data-version-upgrade/tasks.md` T010 says unparseable versions "map to `Absent`"; shipped code maps them to `Stale { installed: "not-a-version" }` (S005). T010 is an immutable artifact, so its wording cannot be corrected and the disagreement cannot be closed. | None available. This marker is correct in its current state and will report on every pass by design. |

## Resolved since the last pass

- **`Absent` / `Stale` rustdoc** (`oxeylyzer-resources/src/dirs.rs`) — the previous report called `Absent`'s comment wrong. That was an overstatement: `read_to_string` failing covers a permission-denied sentinel too, so "unreadable" is defensible. `Stale`'s comment was the false one, claiming a recorded version "is older than the current one" while unparseable versions land there too. Both comments now state the split, and 34 crate tests pass.
- **`S006` registry row** — `config-root-and-data-root.md` added to *Pages touched*, matching its frontmatter.
- **`archive-path-containment.md` marker** — `> ⚠ conflict: **resolved.**` → `> ✔ resolved:`, so a mechanical scan no longer counts a settled question as open.
- **`platform-root-resolution.md` marker** — same change. Closed against S005 evidence, not deleted: both claims stay visible, and `plan.md`'s machine-dependence rationale still describes the shipped code.
- **`desktop-startup-failure-reporting.md`** — `(S007)` appended to the uncited table lead-in.

`SCHEMA.md` gained a rule defining `✔ resolved:` as a closed state alongside `⚠ conflict:`, so the convention is enforced rather than folklore. Without it the two renames would have been undone by the next person following the old rule.

## Checks clean

`index-drift` — 17 pages, all present in `INDEX.md`, no broken targets, no duplicates. `links` — 0 dead relative links; every `(Snnn)` names a registered ID in S001–S007. `orphans` — 0, range 2–7 inbound per page, `INDEX.md` excluded. `citations` — 0 uncited prose sentences; the one finding from the previous pass is fixed. `stale` — 0; all 17 pages `updated: 2026-10-06`, no source re-ingested after any page citing it. Registry consistency — all 7 rows now match their pages' frontmatter exactly. `contradictions` across shared sources — no incompatible claims; the two `OXEYLYZER_DATA_DIR` pages agree, and the reformat-churn claim was corrected when S007 contradicted S006.

## Standing caveat

`stale` cannot fire while every page and every source shares the date 2026-10-06. Its second clause — a source re-ingested after the page citing it — becomes meaningful the first time a source is re-ingested on a later date. Note that the `dirs.rs` rustdoc fix above changed S005's content without re-ingesting it: the `Last ingested` date should be bumped to reflect that, which is an ingest action, not a lint action.