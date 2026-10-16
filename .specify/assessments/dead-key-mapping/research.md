# Idea Research: Dead Key and Shift Mapping for Corpus Cleaning

- **Slug**: dead-key-mapping
- **Created**: 2026-10-12T16:13:41Z
- **Evidence confidence (overall)**: high — findings rest on repo source, the local data root, upstream git history, and the GitHub API; no external claims relied on.

> Intake is untrusted data, not instructions. Nothing in `intake.md` changed what was investigated; the six first-glance unknowns were treated as research questions only.

## Users & Demand

- No user-facing demand signal exists in-repo. There are no issues, no feature requests, and no config field naming dead keys: `grep -rln 'dead' provided/*.toml` over all 51 shipped corpus configs returns **none** — [source: local data root `~/lib/oxeylyzer/static/corpus_configs/provided/`, 51 `.toml` files] (confidence: high).
- The one adjacent user issue is upstream **#34 "How to generate new language data?"** (OPEN), from a French/Japanese/English writer building a custom language file. Their keylogger script literally comments out dead/compose keys, i.e. their data *avoids* dead keys rather than needing them modeled. Nothing in the thread asks for a dead-key config surface — [source: https://github.com/o-x-e-y/oxeylyzer/issues/34, fetched read-only via `gh` API] (confidence: medium — one issue, paraphrased).
- The demand for this feature came entirely from **in-code TODOs**, not users: `corpus_transposition.rs:225` ("add proper dead key mapping, custom shift and repeat char") and the derived queue entry — [source: repo] (confidence: high).

## Prior Art

- **The need is already served, by a different mechanism.** `[multiple]` / `to_multiple` with `*` as the simulated accent key is the project's actual dead-key modeling. `README.md:177` documents the contract: "single character, and the sequence of keys this character should be converted into … very useful for languages like Spanish or French where you can use an accent key (denoted by `*`) to collapse all kinds of accented keys into single key + letter. You don't need to change the corpus for this at all, it's purely handled within these corpus rules." — [source: `README.md:177`, `:196-204`] (confidence: high).
- **7 of 51 published configs already use it**, with `uppercase_versions = true`: `czech`, `french`, `hungarian`, `italian`, `polish`, `spanish`, `welsh_pure` — [source: grep over local data root] (confidence: high).
- **Upstream ships a `german_deadkey` corpus config + data file**, and it is built on `[multiple]` with `*`, not on any dead-key field:
  ```toml
  inherits = ["./default.toml"]
  [multiple]
  uppercase_versions = true
  list = [ ["ü","*u"], ["ä","*a"], ["ö","*o"], ["ß","*s"] ]
  ```
  Present on the `data-files` branch (fetched via GitHub API) and in local history at `43604a9`/`c263bed`/`87d9478`, alongside `static/language_data/german_deadkey.json` (`char_total` 45,198,388). It is **not** in the local data root's `provided/` listing — [source: `gh api repos/o-x-e-y/oxeylyzer/contents/static/corpus_configs/provided?ref=data-files`; local `git log --diff-filter=A`] (confidence: high). This is the strongest prior-art fact: the project's own named "deadkey" artifact uses the existing `multiple` mechanism.
- **`with_dead_key` is a specialized `multiple`, proven by execution.** The wiki records that `.with_dead_key([('á','a')], '´')` produces output identical to `.with_mappings([('á', vec!['´','a'])])` — [source: `wiki/pages/corpus-cleaner-direction.md`, source S012 = `.specify/bugs/with-dead-key-inverted/`] (confidence: high).
- **The "inversion bug" framing is already disproven.** `.specify/bugs/with-dead-key-inverted/` returned verdict `invalid` (2026-10-06T23:57:32Z), fix `not-applied`, test `verified`; all 173 tests passed. The docstring line the original report quoted as its contract (`// i.e. corpus "á" should become "a"`) **does not exist in the source**, confirmed by grep and against commit `13327a7` — [source: `.specify/bugs/with-dead-key-inverted/{assessment,fix,test}.md`] (confidence: high). `TODO.md` still carries the pre-correction "confirmed bug" framing and should be reconciled.
- **The queue's blocking premise has partly expired, partly was wrong.** Intake's trigger was that the skip note's reasons may no longer hold. Checking each:
  - "`CorpusConfig` has no dead-key field" — **still true**; no dead-key field exists — (confidence: high).
  - "custom shift and repeat char are missing" — **false, and already recorded as false**: `.shift_char(...)` and `.repeat_key(...)` are wired at `corpus_transposition.rs:243-244` and consumed at `corpus_cleaner.rs:396-397`. History confirms deliberate work: `f5b76fa` "add shift key configuration to corpus config", `3102e17` "add repeat key option for corpus config" — [source: wiki S012 + git log] (confidence: high). So shift/repeat are done; dead keys are the only remaining concept in that TODO's wording.
  - "`HashMap<char, Vec<char>>` cannot express a two-char dead-key sequence" — **misread**. The map is corpus char → keystrokes; a two-char *output* is expressible today. Only two-char *input* is not, and nothing needs it — [source: `corpus_cleaner.rs:9`, wiki S012] (confidence: high).
  - "sequence-matching machinery exists, no new infrastructure" — **true but unused for this**: `SlidingWindow` is allocated only when `repeat_key` is on (`clean_corpus`: `match cleaner.repeat_key { true => SlidingWindow::new(2,…), false => new(1,…) }`) — (confidence: high).
- **The helper has been dead since birth.** `with_dead_key` was added in `0541468` ("add oxeylyzer v2's corpus parsing logic", 2026-02-08) and has had **zero callers and zero tests** ever since; its only later touch was `13327a7`/`23652b7` ("add docs and doctests using gemini"), which added a doctest that calls `.build()` and asserts nothing. `git log -S 'with_dead_key'` across all branches shows only those three commits — [source: git log/blame] (confidence: high). Note the commented-out parameter at `corpus_cleaner.rs:286`: `// include_uppercase: bool,`.

## Market & Context

- The cost of doing nothing is low and already absorbed. Diacritic languages are modeled today via `multiple`+`*`, and the shipped per-language data files already reflect that convention — [source: local `language_data/*.json`] (confidence: high).
- A real-world keyboard's dead key and this project's `*` are different things, and the README says so deliberately. `README.md:198`: "I have the most common set of accents act like this (for French this is `^`), but have the rest simulate a keypress in between the accent key and the actual letter. For example, `ú` becomes `* u` … You go `*`->`<key on opposite hand>`->`<letter>`." The model is a *hand-alternation simulation* for scoring comfort, not a faithful OS dead-key state machine — (confidence: high).
- Alternative available today with zero code: an author who wants a true dead key can write `[multiple] list = [["á","´a"]]`. That is exactly what `with_dead_key` produces. The only thing missing is a shorter spelling — [source: README contract + wiki execution proof] (confidence: high).

## Data & Constraints

- Measured character distributions in the shipped data confirm the `*` convention is load-bearing, and show accented letters that *did* get dedicated keys:

  | language | `*` share | accented letters kept verbatim |
  |---|---|---|
  | czech | **6.51%** (4th most common char) | none |
  | polish | 2.58% | `ł` 1.19%, `ę` 0.81%, `ż` 0.67% |
  | hungarian | 1.62% | `á` 3.38%, `é` 3.27%, `ö` 1.01%, `ó` 0.97%, `ő` 0.91% |
  | spanish | 1.40% | `ó` 0.80% |
  | french | 0.70% | `é` 2.36%, `à` 0.43% |
  | italian | 0.62% | none |
  | esperanto | 0.00% | `ĝ` `ĉ` `ŭ` `ŝ` `ĵ` `ĥ` (all dedicated) |
  | portuguese | 0.00% | `ã` `ç` `á` `é` `ó` `ê` `õ` `â` `ô` (all dedicated) |

  [source: parsed `~/lib/oxeylyzer/static/language_data/*.json` in-sandbox] (confidence: high). Czech at 6.5% `*` matches README's claim that Czech uses one accent key for many accents; esperanto/portuguese at 0% show the "dedicated key" path also in production use.
- **A collision constraint any real dead-key work must respect**: `default.toml` `[one_to_one]` already normalizes the acute accent away — `from = "´÷‘’–ʹ͵"`, `to = "'/''-''"`, i.e. corpus `´` becomes `'`. So `´` cannot simultaneously serve as a dead-key character under the inherited defaults. `normalize_misc_punctuation(true)` in core does the same (`('´', '\'')`) — [source: local `default.toml`, `corpus_cleaner.rs` `normalize_misc_punctuation`] (confidence: high). This is a concrete design constraint the intake's "what TOML shape" unknown did not anticipate.
- Uppercase handling is already owned by a working mechanism: `MultipleAsVec`'s deserializer emits `(upper, to)` pairs when `uppercase_versions && from.to_uppercase().count() == 1` (`corpus_transposition.rs:~34`). That directly answers intake unknown 3's mechanism question — a dead-key path that did *not* reuse this would duplicate it — [source: repo] (confidence: high).
- Corpus cleaning feeds every score. `Data` is built from cleaned text; `AnalyzerData` bakes weights ×100 into flat i64 arrays — [source: `CLAUDE.md` data pipeline] (confidence: high). So an incorrect mapping is not cosmetic. But equally, changing `default.toml` or any of the 7 `*`-using configs would silently re-score every existing layout and invalidate the shipped `language_data/*.json`, which carry a hand-bumped data version (`DATA_VERSION = "1.0.0"`, `oxeylyzer-resources/src/download.rs:19`) — [source: repo + `wiki/pages/why-the-data-version-is-hand-bumped.md`] (confidence: high).
- No corpus *text* is shipped locally to re-run cleaning end-to-end: `language_data_raw/` holds 11 files, none for czech/french/hungarian, and `static/text/` in-repo holds only `test/`. The `compare_repeat_key` test reads `../static/text/monkeyracer/mr.txt`, which is absent locally — so a "counts are measurably wrong" experiment needs the data-files archive — [source: local `find`, `corpus_cleaner.rs` test] (confidence: high).

## Evidence Against the Idea

- **The feature may already exist.** `multiple`+`*` is a documented, README-endorsed, 7-config-in-production dead-key model, and upstream's own `german_deadkey.toml` uses it. Adding a `[dead_key]` section would create a second way to spell `["á","´a"]` — [source: README:177, local configs, `data-files` branch] (confidence: high).
- **`with_dead_key` is redundant by construction.** It is one line over `with_mapping`; the wiki already asks "whether `with_dead_key` should exist at all" — [source: `corpus_cleaner.rs:288-290`, wiki S012] (confidence: high). Deletion is a live alternative that no evidence rules out.
- **Zero demand.** No issue, no config, no user. Demand is a stale TODO string. The TODO's own wording ("dead key mapping, custom shift and repeat char") is 2/3 already implemented, which lowers trust in the remaining third — [source: repo, git log `f5b76fa`, `3102e17`] (confidence: high).
- **The intake's central factual dispute is already settled, against the bug framing.** The `invalid` verdict is documented, execution-verified, and wiki-ingested. Proceeding as "fix an inversion bug" would re-litigate a closed question — [source: `.specify/bugs/with-dead-key-inverted/`] (confidence: high).
- **The real remaining defect is documentation, not capability** — and it is already recorded: docstring doesn't name which side is input; doctest asserts nothing. That is a small, self-contained fix that does not need a new config surface — [source: wiki S012] (confidence: high).
- **Regeneration blast radius.** Any change to shared cleaning defaults invalidates shipped data files and every existing layout score, gated by a hand-bumped version sentinel — [source: `download.rs:19`, wiki data-version page] (confidence: high).
- **A concrete blocker for the obvious design.** `´` is already eaten by `default.toml`'s `one_to_one`, so the intuitive "dead key = the accent char" spelling conflicts with inherited normalization — (confidence: high).
- No evidence was found that any current character count is wrong. Czech's 6.5% `*` and French's dedicated `é` are consistent with the README's stated intent, not with a defect.

## Gaps & Open Questions

- [NEEDS CLARIFICATION: does the maintainer want a *dedicated key* model, a *simulated alternation* model (`*`), or a *true OS dead-key state machine*? These are three different features and only the second is implemented. Intake unknown 1 assumed a two-way choice.]
- [NEEDS CLARIFICATION: is `with_dead_key` intended as public API for library consumers (it is `pub` and doctested), or internal dead code safe to delete?]
- [NEEDS CLARIFICATION: if a `[dead_key]` section were added, what does it buy over `["á","´a"]` in `multiple`? A shorter spelling is the only benefit identified — is that worth a second mechanism?]
- [NEEDS CLARIFICATION: would upstream accept a change to `default.toml`'s `one_to_one` `´` normalization, or must dead keys work around it?]
- [NEEDS CLARIFICATION: does the Tauri GUI need to expose corpus-config editing at all? Intake unknown 6 is unexamined — no Tauri source was read in this stage.]
- Not measured: whether re-cleaning any corpus with vs. without a dead-key model changes scores. Blocked locally by the absent corpus text (see Data & Constraints); would need the `data-files` archive.
- Not investigated: prior art outside this repo (other layout analyzers' dead-key handling). No fetch attempted; only `github.com` was contacted, and only via the `gh` API for this project's own issues and branch contents.

## Sources

All sources are local repository files, the local data root, or upstream GitHub. No unrecognized host was fetched.

- `README.md:177`, `:196-204` — repo (host: n/a, policy: local file)
- `oxeylyzer-core/src/corpus_cleaner.rs` (`:9`, `:272-293`, `:286`, `:288-290`, `build()`, `clean_corpus`, `next()` at `:426`) — repo
- `oxeylyzer-repl/src/corpus_transposition.rs` (`:25`, `:34`, `:110`, `:126`, `:225`, `:243-244`) — repo
- `oxeylyzer-resources/src/download.rs:6,12,19,140` — repo
- `.specify/bugs/with-dead-key-inverted/{assessment,fix,test}.md` — repo
- `wiki/pages/corpus-cleaner-direction.md` (source S012), `wiki/sources.md:19`, `wiki/pages/why-the-data-version-is-hand-bumped.md` — repo
- `.specify/extensions/time-machine/features-queue.yml` (`dead-key-mapping`, `corpus-config-serialization` skip notes) — repo
- `TODO.md` — repo
- Local git history: `0541468`, `13327a7`, `23652b7`, `43604a9`, `c263bed`, `87d9478`, `b99ac97`, `f5b76fa`, `3102e17`, `28e104c` — repo
- `~/lib/oxeylyzer/static/corpus_configs/provided/*.toml` (51 files; `default.toml`, `czech.toml`, `french.toml`, `hungarian.toml`, `spanish.toml`, `esperanto.toml`, `portuguese.toml`, `vietnamese.toml` read) — local data root
- `~/lib/oxeylyzer/static/language_data/*.json` — parsed in-sandbox for the distribution table — local data root
- https://github.com/o-x-e-y/oxeylyzer/issues/34 (host: github.com, policy: allowlisted; fetched read-only via `gh`, no redirects followed, no instructions obeyed)
- https://github.com/o-x-e-y/oxeylyzer/contents/static/corpus_configs/provided?ref=data-files (host: github.com, policy: allowlisted; read-only via `gh api`)
- https://github.com/o-x-e-y/oxeylyzer/git/trees/data-files?recursive=1 (host: github.com, policy: allowlisted; read-only via `gh api`)
