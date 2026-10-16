# Bug Verification: stripped-remainder containment closes the archive escape

- **Slug**: archive-prefix-double-dot-escape
- **Tested**: 2026-10-10
- **Assessment**: ./assessment.md
- **Fix**: ./fix.md
- **Result**: verified

## Summary

The original symptom no longer reproduces. The assessment's reproduction was
exercised directly against the real extraction code path in two throwaway copies
of the crate — once against the shipped pre-fix `download.rs` (from `HEAD`) and
once against the fixed version — and the two diverge exactly as predicted: every
single-`..` shape and the doubled-separator shape escape pre-fix and are refused
post-fix, with no file written outside `dest` and no `.data-version` sentinel.
219 workspace tests pass, and the changed crate is lint- and format-clean with
its previously-dead test helpers now wired in.

## Checks Performed

| Check | Command / Action | Result | Notes |
|-------|------------------|--------|-------|
| Reproduction, pre-fix control | print-only probe over `HEAD`'s `download.rs` in a scratch copy | pass (bug reproduced) | `single-dot`, `padded`, `sideways` → `Ok` + file outside `dest`; `doubled-sep` → `Ok` + file at its literal absolute path |
| Reproduction, post-fix | same probe over the fixed code, with assertions re-enabled | pass | all hostile shapes → `Err`, `outside=[]`; positives still extract |
| Reproduction, assessment's exact shape | probe re-run with the GitHub root entry `oxeylyzer-data-files/` present alongside the hostile entry | pass | identical outcome on both pre-fix and post-fix, so the root entry is not load-bearing |
| New / updated tests | `cargo test -p oxeylyzer-resources --offline download::tests` | pass | 12 passed, 0 failed |
| Crate regression suite | `cargo test -p oxeylyzer-resources --offline` | pass | 39 passed, 0 failed (34 before the fix) |
| Workspace regression suite | `cargo test --workspace --offline` | pass | 219 passed, 0 failed across all 5 members + doc-tests |
| Format | `rustfmt --edition 2024 --check oxeylyzer-resources/src/download.rs` | pass | clean |
| Lint | `cargo clippy -p oxeylyzer-resources --offline`, and `--all-targets` | pass | 0 warnings; 0 `never used` warnings, confirming `files_under`/`walk` are no longer dead |
| Guard is load-bearing | guard neutered to `if false && !is_contained(..)` in the fix step | pass | 4 escape tests went red, then restored |
| Real hostile archive over the network | — | not-run | would require publishing/serving a crafted archive; the fix is exercised through `extract_bytes`, the same seam the shipped tests use |

## Output Excerpts

Pre-fix code (`HEAD`), same probe, assertions deliberately disabled so nothing
halts before the escape is visible:

```text
PROBE single-dot          entry=oxeylyzer-data-files/../escaped.txt              result=Ok  outside=["escaped.txt"] inside=[".data-version"]
PROBE sideways            entry=oxeylyzer-data-files/../someotherapp/evil/payload.txt
                                                                          result=Ok  outside=["someotherapp/evil/payload.txt"] inside=[".data-version"]
PROBE doubled-sep         result=Ok  target_exists=true
```

Post-fix code, same entries:

```text
PROBE single-dot          entry=oxeylyzer-data-files/../escaped.txt              result=Err outside=[] inside=[]
PROBE padded              entry=oxeylyzer-data-files/a/b/../../../escaped.txt    result=Err outside=[] inside=[]
PROBE sideways            entry=oxeylyzer-data-files/../someotherapp/evil/payload.txt result=Err outside=[] inside=[]
PROBE positive-in-bounds  entry=oxeylyzer-data-files/a/../b.txt                  result=Ok  outside=[] inside=[".data-version", "b.txt"]
PROBE doubled-sep         result=Err target_exists=false
test result: ok. 1 passed; 0 failed
```

Note `inside=[]` on the refused cases together with `.data-version` appearing
only on the successful ones: refusal aborts before the sentinel is written, so
FR-003 (no hostile archive recorded as a complete install) holds.

Suite totals:

```text
cargo test -p oxeylyzer-resources   → test result: ok. 39 passed; 0 failed
cargo test --workspace              → passed: 219  failed: 0
```

## Residual Risks

- **Windows is untested.** `Component::Prefix` (drive-letter) rejection and the
  behavior of repeated separators under Windows path semantics are only reachable
  there; CI runs tests on `ubuntu-latest` only (`.github/workflows/ci.yml:15`),
  and this verification ran on macOS. The logic mirrors the library rule that
  `zip` applies on all platforms, but the fix's own branch is not covered by a
  Windows run.
- **NUL-byte entries have no test.** Rejection still comes from the library
  (`enclosed_name()`), which is unchanged, but nothing pins it; the assessment
  left this open and `fix.md` carries it as a follow-up. A future hand-rolled
  rewrite would drop it silently.
- **No integration coverage of the real caller.** `ensure_data` →
  `download_and_extract` (the production path, reached from
  `oxeylyzer-tauri/src-tauri/src/lib.rs:1669` and `oxeylyzer-repl/src/repl.rs:220`)
  is never driven end to end: `oxeylyzer-tauri` contains no `#[cfg(test)]` at all,
  so its 0 tests are pre-existing, not caused by this fix. Verification therefore
  stops at the `extract_bytes` seam.
- **The archive itself is still unverified.** No checksum or signature exists on
  the download, so the containment rule is the only defense; that is unchanged by
  this fix and remains the reason the original severity was high.
- **Unrelated pre-existing warnings** observed while checking, untouched: a
  rustdoc private-link warning at `oxeylyzer-resources/src/dirs.rs:134`
  ([`DATA_VERSION`]), a clippy `redundant reference` at
  `oxeylyzer-repl/src/repl.rs:759`, and workspace `cargo fmt --check` diffs in
  `oxeylyzer-core/src/generate.rs`.
- **Docs now lag the code**, as `fix.md` lists: `tasks.md` T002 still prescribes
  the wrong order, `TODO.md` still describes the bug as unfixed, and
  `wiki/pages/archive-path-containment.md` still documents a live breach.

## Recommendation

Close the bug — verified against the real extraction code path on both sides of
the change, with the reproduction shown to discriminate pre-fix from post-fix
rather than merely asserted. Before merging, note two things the verification
could not settle: the fix's Windows-specific branch has no test run on Windows
(one `Component::Prefix` line in CI would cover it), and the follow-ups in
`fix.md` are still open — most importantly correcting `specs/001-archive-safety/tasks.md`
T002, which prescribes the ordering that caused this bug and will re-seed it into
the wiki on the next ingest.
