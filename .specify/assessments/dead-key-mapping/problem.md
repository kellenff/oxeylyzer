# Problem Definition: Dead-Key Modeling Is Undocumented, Not Missing

- **Slug**: dead-key-mapping
- **Created**: 2026-10-12T16:41:00Z
- **Inputs used**: intake.md ✓ | research.md ✓ | user input (slug only)

## Problem Statement

A person authoring corpus rules for a diacritic language cannot tell, from the code or docs, how this project models dead keys — because two mechanisms describe the same thing and neither says so. The capability itself is not missing: `[multiple]` with `*` is documented in the README and used by 7 of 51 shipped configs, and upstream's own `german_deadkey.toml` is built on it. What is missing is any statement of that equivalence, so the readable surface contradicts the working one.

This is not hypothetical. It has already produced a concrete, evidenced failure: a reader took `with_dead_key`'s docstring at face value, concluded the implementation was inverted, and filed it as a "confirmed bug" carrying a **high** severity claim that it "corrupts counts for any language written with diacritics." The claim was wrong — the docstring line quoted as the contract *does not exist in the source* — but it consumed a full triage cycle (three artifacts under `.specify/bugs/with-dead-key-inverted/`, a scratch probe crate, a 173-test regression run, and a wiki page) to disprove. The same misreading then propagated into `TODO.md`, the Time Machine queue's skip note, and this assessment's own intake, each restating it with slightly different detail. Three of those four records still carry the uncorrected framing.

## Affected Users & Stakeholders

- **Users: corpus-config authors** — the people the README addresses at `:196-204` ("Coming up with good rules"), writing `corpus_configs/*.toml` for a new language. They must choose between dedicated keys for common accents (`é` in French, all of esperanto/portuguese) and a simulated accent key (`*`, czech at 6.51% of characters). Nothing in the codebase tells them `with_dead_key` and `multiple` are the same choice, so a reader who finds the `pub` helper reasonably assumes it is the intended path and that the config surface is incomplete. Upstream issue **#34** shows this persona is real and active.
- **Users: library consumers of `oxeylyzer-core`** — `with_dead_key` is `pub`, doctested, and part of the crate's documented API surface. Its doctest calls `.build()` and asserts nothing, so it passes whichever way the mapping goes and pins no behavior for anyone reading it as a contract.
- **Stakeholder: the maintainer** — owns the direction question this assessment cannot answer (dedicated key vs. simulated alternation vs. true OS dead-key state machine are three different features; only the second exists, and `README.md:198` indicates that is deliberate). Also owns whether the hand-bumped `DATA_VERSION = "1.0.0"` sentinel may move, since any change to shared cleaning defaults invalidates every shipped `language_data/*.json` and re-scores every existing layout.
- **Stakeholder: this assessment pipeline** — the record it produces is only as good as the code it reads. `TODO.md`, `features-queue.yml`, and `intake.md` all carried a claim that source review disproved. Each is a place the next reader will trust.

## Goals

- A reader of `corpus_cleaner.rs` can determine the direction of the mapping and the relationship between `with_dead_key` and `multiple` without executing code or reading a wiki page.
- The behavior the docstring example implies is pinned by an assertion, so a future change of direction fails a test instead of passing silently.
- `TODO.md`, the Time Machine queue entry, and the intake record agree with the `.specify/bugs/with-dead-key-inverted/` verdict and with each other. One framing survives.
- The stale parts of the TODO at `corpus_transposition.rs:225` stop claiming work that is already done (`.shift_char(...)` and `.repeat_key(...)` are wired at `:243-244` and consumed at `corpus_cleaner.rs:396-397`, per commits `f5b76fa` and `3102e17`).
- A corpus-config author can find, from the README or a config comment, that `["á","´a"]` in `[multiple]` is the supported way to express a dead key.

## Non-Goals

- **No new `CorpusConfig` field.** Research found no demand for one, and `multiple` already expresses the mapping; a second spelling would be a second mechanism to keep consistent.
- **No true OS dead-key state machine.** Two-char *input* sequences are not expressible in `HashMap<char, Vec<char>>` and nothing needs them. The `SlidingWindow` machinery exists but is allocated only when `repeat_key` is on.
- **No change to any shipped corpus config or `default.toml`.** In particular, no touching the `[one_to_one]` rule that already normalizes `´` → `'` — that collision is a constraint on any future design, not a defect to fix here.
- **No regeneration of `language_data/*.json`, and no `DATA_VERSION` bump.**
- **No re-litigating the inversion verdict.** `.specify/bugs/with-dead-key-inverted/` returned `invalid`, fix `not-applied`, test `verified`. Reopening it is out of scope.
- **No GUI work.** Whether Tauri should expose corpus-config editing was not investigated (no Tauri source read) and is not decided here.
- **Not deciding whether `with_dead_key` should be deleted.** That is a solution choice; it stays open for `shape`.

## Success Metrics

| Metric | Baseline (measured) | Target |
|---|---|---|
| `with_dead_key` doctests asserting an output | **0 of 1** — the doctest at `corpus_cleaner.rs:276-281` calls `.build()` and asserts nothing | 1 of 1 |
| Docstring stating which side of the mapping is input | **no** — "Adds mappings that involve a dead key followed by another character" names neither side | yes |
| Records agreeing with the `invalid` verdict | **1 of 4** — `.specify/bugs/` corrected; `TODO.md`, `features-queue.yml`, `intake.md` all still carry the inversion framing | 4 of 4 |
| Stale claims in the TODO at `corpus_transposition.rs:225` | **2 of 3** — "custom shift" and "repeat char" are both implemented and wired | 0 of 3 |
| Tests pinning dead-key direction | **0** (`git log -S 'with_dead_key'` across all branches: `0541468`, `13327a7`, `23652b7` — no test commit) | ≥ 1 |
| Callers of `with_dead_key` outside its own doctest | **0**, unchanged since `0541468` (2026-02-08) | unchanged — this metric exists to prove nothing depends on the current spelling |
| Shipped configs or data files modified | **0** | **0** |
| README statements equating `multiple` and dead keys | **0** — `README.md:177` describes `*` as "an accent key" without naming dead keys | ≥ 1 (qualitative: whether a config author would find it) |

No metric requires re-running corpus cleaning. That matters: no corpus *text* is available locally (`language_data_raw/` holds 11 files, none for czech/french/hungarian; `static/text/` holds only `test/`), so any success signal depending on measured counts is unreachable without fetching the `data-files` archive.

## Cost of Inaction

Low, and mostly already paid. Diacritic languages are modeled correctly today — czech's 6.51% `*`, french's dedicated `é` at 2.36%, esperanto and portuguese at 0% `*` with every accent on its own key are all consistent with the README's stated intent, not with a defect. No character count is known to be wrong. Scores are unaffected.

What persists is a recurring comprehension cost with a demonstrated ability to generate false work: one invalid high-severity bug report, one superseded assessment, one skipped queue feature, and one `TODO.md` entry that has now misled four separate records including this pipeline's own intake. `with_dead_key` stays dead code with an unasserted doctest, so the next reader has the same chance to misread it. Each occurrence costs a triage cycle, not a feature.

The honest asymmetry: inaction risks another wasted investigation; action risks changing nothing that users touch. Neither is large, and research found no demand signal pushing either way.

## Open Questions

Six carried forward from research; none resolved here, and the first three gate any solution.

1. [NEEDS CLARIFICATION: which model does the maintainer want — *dedicated key*, *simulated alternation* (`*`), or a *true OS dead-key state machine*? Intake framed this as a two-way direction choice; research found three distinct features, of which only the second is implemented. Everything downstream depends on this answer.]
2. [NEEDS CLARIFICATION: is `with_dead_key` intended as public API for library consumers, or internal dead code safe to delete? It is `pub`, doctested, and has had zero callers since it was added.]
3. [NEEDS CLARIFICATION: if a `[dead_key]` config section were ever wanted, what would it buy over `["á","´a"]` in `[multiple]`? Research identified only a shorter spelling. Note the constraint: `default.toml`'s `[one_to_one]` already maps `´` → `'`, so the intuitive accent-char spelling collides with inherited normalization.]
4. [NEEDS CLARIFICATION: would upstream accept any change to `default.toml`'s `´` normalization, or must dead keys work around it?]
5. [NEEDS CLARIFICATION: does the Tauri GUI need to expose corpus-config editing at all? Unexamined — no Tauri source was read during research.]
6. Not measured: whether re-cleaning any corpus with vs. without a dead-key model changes scores. Blocked locally by absent corpus text; needs the `data-files` archive.

Also uninvestigated: prior art outside this repo (how other layout analyzers handle dead keys). No external fetch was attempted beyond this project's own GitHub issues and branch contents.

## Next stage

`/speckit.assess.shape slug=dead-key-mapping` — Open Question 1 is the hinge. If the maintainer wants the *simulated alternation* model that already exists, `shape` is choosing between documenting it and deleting the redundant helper; if they want a *true dead-key state machine*, that is a different and much larger feature with its own appetite, and this problem definition does not cover it.
