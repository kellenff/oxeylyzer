# Decision: Dead Key and Shift Mapping — Not Worth Building As Specified

- **Slug**: dead-key-mapping
- **Decided**: 2026-10-12T17:46:37Z
- **Verdict**: **kill**
- **Artifacts reviewed**: intake.md ✓ | research.md ✓ | problem.md ✓ | concept.md ✓

## Scorecard

| Criterion | Rating | Justification |
|-----------|--------|---------------|
| Problem validity | **weak** | The problem as *specified* — "dead-key mapping is missing, so diacritic-language counts are wrong" — was tested against source and falsified: `multiple`+`*` is the dead-key model, documented at `README.md:177`, used by 7 of 51 shipped configs, and upstream's own `german_deadkey.toml` uses it. What survives is a documentation-clarity defect, not a capability gap — lower stakes than what was pitched. |
| Evidence strength | **strong** | Unusually well-evidenced, and it is what kills the idea. Executed proof (S012) that `with_dead_key` output is byte-identical to `with_mappings`; the `invalid` verdict from `.specify/bugs/with-dead-key-inverted/` with fix `not-applied` and test `verified`; `git log -S 'with_dead_key'` showing exactly three commits (`0541468`, `13327a7`, `23652b7`) and zero callers since 2026-02-08; parsed character distributions across 9 languages; a `data-files`-branch fetch confirming `german_deadkey.toml` exists and uses `[multiple]`. |
| Value vs. inaction | **weak** | Research could not name a single character count that is wrong. Czech's 6.51% `*` and French's dedicated `é` at 2.36% match the README's stated intent. Cost of inaction is comprehension only, and in-repo transitions are already safeguarded by `cargo test` (~200 tests), `cargo clippy`, and benchmark compilation. |
| Feasibility / appetite | **strong** | Deletion of a zero-caller `pub` method is a compile-time fail-safe change, small appetite, no dependency or architecture change, and it inherits A's doc work. Demonstrated capacity exists in-repo. |
| Strategic fit | **unknown** | `.specify/memory/constitution.md` is an unratified placeholder (all `[PRINCIPLE_N]` / `[SECTION_N]` templates, no version or ratification date), so **no project principles exist to check against**. Not glossed: strategic fit is scored `unknown` because there is nothing to read, not because the idea is neutral to strategy. |
| Risk posture | **weak** | The serious risks are unmitigated *and mutual*. Fork-local deletion carries no downstream risk (user-established private fork) — so there is no reason to avoid it. But the underlying question that gates whether deletion is even the right move (wanted model: simulated alternation vs. true OS dead-key state machine) is unresolved, and if the answer is "state machine," `with_dead_key`'s signature edits to `with_mappings`' input shape — a real feature that deletion would then be a mistake to have made. |

## Verdict & Rationale

**Kill the feature.** The idea was assessed honestly and lost on its own evidence, not on appetite or taste.

Three findings converge. First, the premise failed: dead-key mapping is not missing — it is `multiple` with `*`, documented by the maintainer at `README.md:177` as exactly the accent-key mechanism for Spanish and French, in production in 7 of 51 configs, and used by the project's own `german_deadkey.toml` on the `data-files` branch. Second, the helper at the center of the idea is provably redundant, not merely unused: `.with_dead_key([('á','a')], '´')` produces output byte-identical to `.with_mappings([('á', vec!['´','a'])])`, and it has had zero callers since `0541468` added it. Third, the high severity that motivated the whole assessment was a misreading — the "corrupted counts for diacritic languages" claim rested on a docstring line that **does not exist in the source**, and the triage that disproved it is recorded and verified.

On the evidence rules for this stage, the honest verdict is **kill**, not `go`. There is no `go` to write: the one concept option that is *feasible* (a small documentation-and-assertion change) has `weak` problem validity and `weak` value, and the one that matches the *original ask* (a `[dead_key]` config surface) has zero demand across 51 shipped configs, one adjacent user issue whose author is commenting dead keys *out* of their keylogger to avoid the problem, and no differentiable mechanism — only typing.

Why not `needs-clarification`: it would be defensible on the unresolved model question alone (Open Question 1), and I considered it. It is rejected because clarification is available immediately from one person, and because every path from the unresolved question leads somewhere that does not need a specification: if the answer is "simulated alternation" (what exists), the work is a deletion and some sentences; if the answer is "true state machine," that is a *different feature* — two-character input sequences the current `HashMap<char, Vec<char>>` cannot express — which must come back through intake, not be resurrected from this assessment. Running `specify` on "delete a dead method and fix three stale records" would be process theatre: `problem.md`'s success metrics are mostly denumerable file edits with grep-checkable targets, not a feature design.

**The evidence strength is the decisive fact, and it is strong evidence *against*.** A `go` verdict is barred to me by construction: the stage's rules require evidence `adequate`+, and enough of it arrived to show there is nothing here worth specifying. That this assessment ends by killing its own subject is a successful outcome, and four of the six criteria agreeing is not a close call.

**Two things that would reopen this, recorded so the kill is revisitable:**

1. **Demand appears.** Today there is none: no issue, no config field in any of 51 shipped configs, no caller. A user asking "how do I express a dead key" — or issue #34 escalating into one — would make the documentation already recommended here worth doing, and would make Option B's deletion worth deciding on demand rather than on taste.
2. **The answer to Open Question 1 is "true OS dead-key state machine."** That is a genuinely different feature: it needs two-character input matching, which `with_dead_key`'s one-char-keyed map cannot express, and which today's `SlidingWindow` (allocated only when `repeat_key` is on) could serve. It is not covered by this problem definition, this scorecard, or any of the three options. **It should be re-intaken as its own idea** — the intake's framing of this as a two-way direction choice was wrong, research corrected it to three distinct models, and only the second is implemented.

**A note on precedent, carried from `concept.md`:** `generate/obsolete.rs` was verified as *live* — `best_swap`/`score_swap` are the correctness oracle for the cached scoring path, called from `generate.rs:1343,1364` and `benches/generation.rs:35,39,88`. A future dead-code pass that mistakes it for a candidate would break the scoring tests. Recorded here because this kill creates the impression that dead code in this repo has been audited; it has been audited for `with_dead_key` only.

## If needs-clarification

Not applicable — verdict is `kill`. Retained for completeness: the single question that would have driven a `needs-clarification` verdict is Open Question 1 (which of the three dead-key models is wanted), and it is answered by the maintainer directly rather than by another assessment stage.

## If go — Handoff to `/speckit.specify`

Not applicable — no handoff is made. Nothing from this assessment proceeds to `specify`.

## Open Questions Disposition

All six questions carried from `research.md` and `problem.md` remain **open and unresolved**, carried forward as record rather than as blockers. None was answered at any stage of this pipeline; each was either deferred to the maintainer or left uninvestigated. They are preserved here so that reopening this assessment, or a future true-dead-key intake, starts from them:

1. **Which model is wanted** — dedicated key, simulated alternation (`*`, what exists), or true OS dead-key state machine? The hinge. Never answered.
2. **Is `with_dead_key` public API for library consumers?** Assumed internal by the user's private-fork statement; not verified against any external consumer, and no longer blocking, since nothing is being changed.
3. **What would a `[dead_key]` section buy over `["á","´a"]` in `[multiple]`?** Only a shorter spelling was identified, against the constraint that `default.toml`'s `[one_to_one]` already maps `´` → `'`.
4. **Would upstream accept a change to `default.toml`'s `´` normalization?** Not asked; upstream was read-only throughout.
5. **Does the Tauri GUI need corpus-config editing at all?** Never investigated — no Tauri source was read at any stage.
6. **Does re-cleaning any corpus with vs. without a dead-key model change scores?** Not measured; blocked by absent local corpus text and not worth the `data-files` archive for an idea with no evidence of a defect.

Additionally uninvestigated and unclaimed: prior art outside this repository (how other layout analyzers model dead keys). No fetch beyond this project's own GitHub issues and branch contents was made.

**Assessment closed.** Records remain at `.specify/assessments/dead-key-mapping/` for future reference.