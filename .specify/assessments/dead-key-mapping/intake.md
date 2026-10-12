# Idea Intake: Dead Key and Shift Mapping for Corpus Cleaning

- **Slug**: dead-key-mapping
- **Created**: 2026-10-12T13:20:21Z
- **Source**: codebase pointer — `.specify/extensions/time-machine/features-queue.yml` (id `dead-key-mapping`, status `skipped`), `TODO.md`, `.specify/bugs/with-dead-key-inverted/`, `oxeylyzer-core/src/corpus_cleaner.rs`, `oxeylyzer-repl/src/corpus_transposition.rs`
- **Type**: new-capability (wiring an existing-but-unreachable helper into the config surface)

## Idea (as captured)

Quoted verbatim from the Time Machine queue entry:

> **dead-key-mapping — "Dead Key and Shift Mapping"**
> "Map dead keys, custom shift, and repeat characters when cleaning corpora so languages with diacritics and layered layouts produce accurate character counts."
> Files: `oxeylyzer-repl/src/corpus_transposition.rs`, `oxeylyzer-core/src/corpus_cleaner.rs`

Skip note recorded in the queue on 2026-10-06 (verbatim, lightly compressed):

> Skipped 2026-10-06 — real bug, deferred to TODO.md (corpus_transposition.rs:225, corpus_cleaner.rs:282). dead-key `HashMap<char, Vec<char>>` cannot express a two-char dead-key sequence; `CorpusConfig` has no dead-key field; `CorpusCleanerIterator` `SlidingWindow` (line ~415) already provides sequence-matching machinery, no new infrastructure needed.
> Open question: do dead keys generate uppercase forms, or leave that to the existing `uppercase_versions` flag on the `multiple` table?

Companion item in `TODO.md` ("Findings real but not scheduled"), titled "`with_dead_key` inverted contradicts own docs":

> **Status**: confirmed bug, not yet fixed. `with_dead_key` has no callers and no test; becomes a live bug the moment a config surface is added. Fix sketch: decide direction (code vs docstring), extend sequence matching reusing the existing sliding window, add a `dead_key` section to `CorpusConfig`, wire it through `From<CorpusConfig> for CorpusCleaner`, and add a direction-explicit test ("input `´a` yields `a`" — or the opposite, per whichever direction is decided).

Note: the bug triage at `.specify/bugs/with-dead-key-inverted/` subsequently returned verdict **invalid** — the implementation direction (corpus char → keystroke sequence, `"á"` → `"´a"`) was judged correct and the docstring judged ambiguous; no code changed. The `TODO.md` entry predates that correction and still frames the item as a confirmed inversion bug.

## Restated

Give corpus-cleaning configuration a way to express dead-key (and custom shift/repeat) mappings, so that corpora for languages with diacritics are cleaned against the right keystroke sequences and the resulting character counts — which feed every layout score — are accurate. The core builder helper `with_dead_key` already exists but has no callers, no tests, and no `CorpusConfig` field that could reach it.

## Origin & Context

- **Raised by**: the time-machine queue analysis (2026-10-05/06), seeded from in-code TODOs (`corpus_transposition.rs:225` "add proper dead key mapping, custom shift and repeat char"; `corpus_transposition.rs:110` "TODO: adapt for cleaner"). [NEEDS CLARIFICATION: which human stakeholder, if any, originally asked for diacritic-language corpus accuracy]
- **Triggering event**: queue run skipped the item as unreachable; the unskip premise is that the skip note's reasons may no longer all hold — `CorpusConfig` now wires `shift_key` and `repeat_key` through `From<CorpusConfig> for CorpusCleaner`, leaving dead keys as the one unmapped concept in that struct.
- **Noted inconsistency for later stages**: the queue skip note says the `HashMap<char, Vec<char>>` shape "cannot express a two-char dead-key sequence"; the bug assessment concluded that framing was a misread of the cleaner's direction (mapping is corpus char → keystrokes, where `"´a"` as *output* is expressible today). Research should settle which gap is real: expressing dead-key *input* sequences, or simply exposing `with_dead_key` via config.
- **Downstream relevance**: 14 published corpus configs use the mapping table (per the 004 skip note), including czech/french/esperanto with `uppercase_versions = true`; corpus cleaning decides which characters get counted in every score.

## First-glance unknowns (do not answer here)

1. Does the correct model compose corpus `á` → keystrokes `´ a` (what `with_dead_key` does today), or decompose typed `´ a` → `á` (what the docstring example reads like)? The bug triage says the former; `TODO.md` was written believing the latter.
2. Should `CorpusConfig` gain a `dead_key` section, and what would its TOML shape be (list of composed chars? per-layout dead-key char?)?
3. Do dead keys interact with uppercase generation, or does the existing `uppercase_versions` flag on the `multiple` table own that (queue's own open question)?
4. Which published corpus configs actually need dead keys, and is any current count measurably wrong without them? (No verification done at intake.)
5. Is `custom shift and repeat char` (also in the TODO at line 225) already covered by the wired `shift_key`/`repeat_key` fields, making dead keys the only missing piece?
6. Does the REPL/Tauri GUI need to expose this, or is config-file-level support sufficient for v1?

## Next stages

- `/speckit.assess.research slug=dead-key-mapping` — evidence for/against, including the real-vs-perceived gap in item 1/4 above.
- `/speckit.assess.define` → `/speckit.assess.shape` → `/speckit.assess.decide` only if research survives.
