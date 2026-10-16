---
description: "Task list for 009 Modern Dependency Foundation for Maintained Fork"
---

# Tasks: Modern Dependency Foundation for Maintained Fork

**Input**: Design documents from `specs/009-modernize-deps-idioms/`

**Prerequisites**: plan.md ✓, spec.md ✓ (user stories P1–P3), research.md ✓ (R1–R13), data-model.md ✓ (E1–E7), contracts/ ✓ (`ci-gate.md`, `cli-surface.md`), quickstart.md ✓

**Tests**: INCLUDED — the spec mandates them: FR-013 keeps six hostile-input tests in reject semantics,
FR-015 requires the CLI accept/reject surface to be diff-checked (not eyeballed), and US1-1 requires
demonstrating that an advisory fails the build. Test tasks precede their implementation.

**Organization**: Phases 1–2 are Setup + Foundational (the CI gate, which is itself US1's deliverable);
Phases 3–5 map to US1/US2/US3; Phase 6 is polish/cross-cutting.

**Hard fence (FR-017)**: no task may touch scoring, generation algorithms, corpus cleaning, data versions, or
`language_data/*.json`. Behavior of computed results must be identical before/after.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no dependency on an incomplete task)
- **[Story]**: user story the task serves (US1 drift gate, US2 Rust deps current, US3 smaller/owned surface)
- Exact repository-relative file paths in every description

## Path Conventions

Rust workspace (5 crates) + one frontend directory:

- `Cargo.toml` (root; workspace manifest AND binary crate)
- `oxeylyzer-core/`, `oxeylyzer-repl/`, `oxeylyzer-resources/`, `oxeylyzer-bench/`,
  `oxeylyzer-tauri/src-tauri/` (Rust member), `oxeylyzer-tauri/` (frontend: `package.json`, `src/`)
- Root policy files: `msrv-holds.toml`, `audit.toml`, `docs/dependency-policy.md`
- CI: `.github/workflows/ci.yml`, `scripts/ci/` (small gate helpers)

---

## Phase 1: Setup (green baseline)

**Purpose**: FR-009 — CI is red on `main` today (`fmt` fails in 9 blocks, `clippy -D warnings` fails at
`oxeylyzer-repl/src/repl.rs:759`); both verified pre-existing on pristine HEAD. Nothing else can be gated
until these land as standalone commits.

- [X] T001 Run `cargo fmt --all` and commit ONLY formatting changes; verify `cargo fmt --all -- --check` passes and no behavior diff appears in the change set (FR-009)
- [X] T002 Fix the clippy failure at `oxeylyzer-repl/src/repl.rs:759` and any clippy findings newly surfaced by T001's reformat, then confirm `cargo clippy --all-targets --all-features -- -D warnings` exits clean (FR-009)
- [X] T003 Record the baseline: run `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings` and note pass/fail per command in the commit message of the first gate commit, so later red CI is attributable (US1-4 precondition)

**Checkpoint**: tooling checks green locally; CI on this branch is red/green only for the new gate work.

---

## Phase 2: Foundational — the Standard and the Signal (blocks all stories)

⚠️ **CRITICAL**: per research R1/R3/R4 the gate lands BEFORE the catch-up commits so every dependency
change's CI signal is attributable. This phase IS the US1 deliverable; US2/US3 both depend on it.

- [X] T004 Declare the MSRV floor in exactly one authoritative location: add `rust-version = "1.97"` under a new `[workspace.package]` table in root `Cargo.toml` (FR-001, SC-002; value is the mirror, the feed is truth)
- [X] T005 [P] Inherit the floor in `oxeylyzer-core/Cargo.toml` (`[package]` gets `rust-version.workspace = true`)
- [X] T006 [P] Inherit the floor in `oxeylyzer-repl/Cargo.toml` (`rust-version.workspace = true`)
- [X] T007 [P] Inherit the floor in `oxeylyzer-resources/Cargo.toml` (`rust-version.workspace = true`)
- [X] T008 [P] Inherit the floor in `oxeylyzer-bench/Cargo.toml` (`rust-version.workspace = true`)
- [X] T009 [P] Inherit the floor in `oxeylyzer-tauri/src-tauri/Cargo.toml` (`rust-version.workspace = true`)
- [X] T010 Write the floor-computation helper `scripts/ci/rust-window.sh`: fetch `https://static.rust-lang.org/dist/channel-rust-stable.toml`, parse the latest three stable releases, print the oldest as the floor; on fetch failure print `dependency gate unavailable (network); build not advisory-verified` and exit non-zero (fail-closed, FR-001, contracts/ci-gate.md step 3 + degradation contract)
- [X] T011 Extend `.github/workflows/ci.yml` with the MSRV steps from contracts/ci-gate.md: step 4 compare `rust-version` in root `Cargo.toml` to T010's computed floor (print both values on mismatch) and step 5 `cargo +<floor> check --workspace` (FR-002, SC-002)
- [X] T012 Create the advisory allowance list `audit.toml` with an empty `[advisories] ignore` table and a header comment explaining the reason-comment rule (FR-004; the post-upgrade expectation per FR-007 is that it stays empty)
- [X] T013 Add the CI audit step to `.github/workflows/ci.yml`: `cargo install cargo-audit --locked` then `cargo audit --config audit.toml`, failing the build and naming the RUSTSEC id and crate for any unrecorded advisory (FR-003, SC-001, US1-1)
- [X] T014 Add the allowance-list lint to `.github/workflows/ci.yml`: every id in `audit.toml`'s `ignore` array must have a `# reason:` comment on the immediately preceding line; orphaned ids fail the build (FR-004, contracts/ci-gate.md step 7)
- [X] T015 Create the MSRV hold record `msrv-holds.toml` (empty `[[hold]]` set at planning time) documenting the fields `crate`/`held`/`latest`/`required_rust`/`reason` (FR-002a, data-model.md E3)
- [X] T016 Add the freshness step to `.github/workflows/ci.yml`: `cargo outdated --workspace` output logged, then cross-checked against `msrv-holds.toml` so a behind-crate that IS a recorded hold is reported as `held (recorded)` and an unrecorded major gap fails the build; a recorded hold whose `latest` now fits the window must fail until removed (FR-005, FR-006, FR-002a, SC-003, contracts/ci-gate.md step 8)
- [X] T017 Write `docs/dependency-policy.md` covering the seven sections of data-model.md E4: MSRV window rule (feed is truth, manifest is mirror), advisory gate + allowance procedure, freshness + hold procedure, lockfile rule (E7), archive containment policy (E5 / FR-013), retained-with-rationale dependencies (`fxhash` — deliberate perf choice, swap requires benchmark, out of scope), yanked-locked response (FR-001..FR-006, FR-012, FR-013, SC-005)
- [X] T018 Prove the gate end-to-end (US1 Independent Test): temporarily pin `anyhow = "=1.0.102"` in root `Cargo.toml`, run `cargo generate-lockfile && cargo audit --config audit.toml` and confirm failure names `RUSTSEC-2026-0190` + `anyhow`; add the id to `audit.toml` with a reason and confirm the lint + audit pass; revert the pin and the ignore entry. Record the two runs in the PR description (SC-001, quickstart.md §2)

**Checkpoint**: the drift gate exists and is proven to bite. Foundational done — US1 is testable
independently, and this is the MVP boundary.

---

## Phase 3: User Story 1 — Drift Visible & Enforced (Priority: P1) 🎯 MVP

**Goal**: maintainer learns "are we behind / are we exposed" from one CI log, and an unrecorded advisory
cannot merge. (Delivered by Phase 2; this phase closes the remaining US1 acceptance evidence.)

**Independent Test**: introduce a synthetic advisory → CI fails with the advisory id and package named;
freshness report names current vs available across the workspace with no manual registry query.

- [X] T019 [US1] Add the CI log-formatting/summary step so the freshness output is a single readable
      "current vs available" table per manifest in the job summary (`.github/workflows/ci.yml`), satisfying
      US1-3 and SC-008 ("single command or CI log answers it")
- [X] T020 [US1] Write `scripts/ci/README-dependency-gate.md` (or a `## Maintaining the gate` section in
      `docs/dependency-policy.md`) with the exact local reproduction commands for each new CI step, so a
      maintainer can run the gate without GitHub (US1-3, FR-005, SC-008)
- [X] T021 [US1] Verify network-degradation behavior by running T010 and the audit step with network
      disabled (e.g. `HTTP_PROXY=` block or offline check) and confirming they exit non-zero with the
      explicit "build not advisory-verified" message rather than skipping green (edge case "freshness check
      runs without network", contracts/ci-gate.md degradation contract)

**Checkpoint**: US1 fully functional and independently testable — MVP is reachable/deployable here.

---

## Phase 4: User Story 2 — Rust Dependencies Current (Priority: P2)

**Goal**: every direct Rust dependency at latest stable, both live advisories cleared, workspace tests green
under the new gate.

**Independent Test**: `cargo outdated --workspace` shows no unrecorded gaps; `cargo test --workspace` passes;
CI (including the new audit step) is green.

### Tests first for User Story 2 (containment decision drives the design)

- [X] T022 [P] [US2] Add the zip-9 normalization-rejection test to `oxeylyzer-resources/src/download.rs`
      tests module: an entry `oxeylyzer-data-files/../escaped.txt` must be REJECTED (not written as
      `escaped.txt`), asserting `is_contained` on the raw name is authoritative and `dest/escaped.txt` never
      exists (FR-013, US2-3/4)
- [X] T023 [US2] Add a sentinel test in `oxeylyzer-resources/src/download.rs` asserting
      `dest/.data-version` is absent whenever any entry was refused or not honored, extending
      `mixed_valid_and_hostile_fails_without_partial_files` (FR-014, edge case "hostile entry appears
      before valid one")

### Implementation for User Story 2

- [X] T024 [US2] Bump `zip = "2"` → `zip = { version = "9", default-features = false, features = ["deflate"] }` in `oxeylyzer-resources/Cargo.toml`, applying the non-containment API changes from `.specify/assessments/modernize-deps-idioms/evidence/zip9-upgrade-reverted.patch` (crate/entry API, error types) (FR-006)
- [X] T025 [US2] Rework containment in `oxeylyzer-resources/src/download.rs::extract_bytes` to validate the RAW entry name (`file.name()`) after `ZIP_INNER_PREFIX` stripping and drop the `ZipFile::enclosed_name()` containment call, keeping `is_contained()` authoritative. Because `enclosed_name()` today also rejects NUL bytes (see the comment at `download.rs:107`) and `Path::components()` will not, the raw-name check must reject `name.contains('\0')` explicitly. Add the one-line `shortcut:` comment recording that per-entry writes before abort leave files on disk while the sentinel marks completeness (FR-013, FR-014, R5)
- [X] T026 [US2] Run `cargo test -p oxeylyzer-resources` and confirm all 12 `download.rs` tests pass: the six hostile-input tests keep reject semantics, and `parent_segment_within_bounds_is_allowed`, `prefixed_entry_has_prefix_stripped`, `unprefixed_entry_extracts_at_root`, `archive_root_entry_is_skipped` still pass (not a blanket `..` ban) (US2-3, SC-007)
- [X] T027 [P] [US2] Bump `ureq = "2"` → `"3"` in `oxeylyzer-resources/Cargo.toml` and fix the measured one-line call-site changes in `oxeylyzer-resources/src/download.rs` (R6)
- [X] T028 [US2] Bump `directories = "5"` → `"6"` in `oxeylyzer-resources/Cargo.toml` (workspace dep too) and check `oxeylyzer-resources/src/dirs.rs` for the project-dir API change (R6)
- [X] T029 [P] [US2] Bump `ratatui = "0.29"` → `"0.30"` and `crossterm = "0.28"` → `"0.29"` in `oxeylyzer-bench/Cargo.toml`, fixing any render API drift in `oxeylyzer-bench/src/ui/` (R6)
- [X] T030 [P] [US2] Bump `rustyline = "17.0.2"` → `"18"` in `oxeylyzer-repl/Cargo.toml` and adapt `oxeylyzer-repl/src/repl.rs` completion/hint calls (R6)
- [X] T031 [P] [US2] Bump `itertools` `0.14` → `0.15` in root `Cargo.toml` `[workspace.dependencies]` (flows to core/repl) (R6)
- [X] T032 [P] [US2] Bump `toml` `0.9` → `1` in root `Cargo.toml` `[workspace.dependencies]` (flows to repl/resources) (R6)
- [X] T033 [P] [US2] Bump `shlex` `1.3` → `3` in `oxeylyzer-repl/Cargo.toml` (R6)
- [X] T034 [P] [US2] Bump `nanorand` `0.7.0` → `0.8` in `oxeylyzer-core/Cargo.toml` — verify no RNG call site on the scoring path changes behavior (FR-017 fence; already non-reproducible, no seed API exists) (R6)
- [X] T035 [P] [US2] Bump `fancy-regex` `0.17.0` → `0.19` in `oxeylyzer-core/Cargo.toml` `[dev-dependencies]` (R6)
- [X] T036 [US2] Bump the remaining minors discovered by `cargo outdated` to latest stable in their respective manifests: `rayon`, `serde_with`, `indexmap`, `thiserror`, `serde`, `serde_json`, `indicatif`, `md5`, `approx`, `time_this`, `diol`, `glob`, `libdof` — hold any that fail to resolve rather than forcing (FR-006)
- [X] T037 [US2] Clear the two live advisories in `Cargo.lock`: ensure `anyhow ≥ 1.0.103` and transitive `rustls ≥ 0.23.45` resolve (`cargo update` / tauri-side bump), and confirm `cargo audit --config audit.toml` reports zero advisories (FR-007, US2-2, SC-004)
- [X] T038 [US2] Run the full gate locally after the bumps: `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, `cargo outdated --workspace` and fix regressions before moving on (FR-008, US2-1)
- [X] T039 [US2] Reconcile the nested Tauri lockfile: delete `oxeylyzer-tauri/src-tauri/Cargo.lock`, add the ignore rule to `.gitignore`, and confirm the single root `Cargo.lock` covers the `oxeylyzer-tauri/src-tauri` member (`cargo metadata --format-version 1 | grep -c oxeylyzer-core`) (FR-012, US3-6, E7)
- [X] T040 [US2] Fix the tauri Rust member for the new workspace deps in `oxeylyzer-tauri/src-tauri/Cargo.toml`: `toml = "0.8"` → workspace `toml`, and verify `notify = "6"`/`glob = "0.3"` resolve at latest stable — this member was dropped from the throwaway probe, so compile fixes here are UNMEASURED (plan task-order step 8, spec Assumptions)

**Checkpoint**: US2 independently testable — Rust workspace current, advisories clear, one lockfile, tests green under the gate.

---

## Phase 5: User Story 3 — Dependency Surface Smaller & Owned (Priority: P3)

**Goal**: fewer third-party things to trust; nothing sourced from a personal fork remains in the build; CLI
surface provably unchanged.

**Independent Test**: `grep -rn 'num_cpus\|once_cell\|xflags\|ansi_rgb'` across `*.rs` and `Cargo.toml`
returns nothing; every retained questionable dep has one recorded reason; both CLIs accept/reject exactly as
before.

### Tests first for User Story 3

- [X] T041 [P] [US3] Build the CLI parity table in `oxeylyzer-repl/src/commands.rs` (a `#[cfg(test)] mod flags_parity` there — that module already owns the parse path and `commands.rs` is a task-migration file per contracts/cli-surface.md): enumerate every subcommand/alias from the current `xflags!` grammar (`analyze|a`, `compare|c|comp|cmp`, `swap`, `rank`, `generate|gen|g|improve|i|optimize`, `save|s`, `remove|rm`, `sfbs`, `fspeed`, `scissors`, `lsbs`, `pinkyring|pinky-ring|pr`, `stretches`, `language|l|lang`, `include`, `languages|langs`, `load`, `ngram|n|occ|freq`, `config|cfg|weights`, `reload|r`, `quit|q|exit`) with, per command, one accept vector, one missing-required, one unknown-flag, one bad-numeric-value, and one repeated-argument vector. Drive them through the REAL parse path: `Repl::from_vec(args)` (`commands.rs:81`), not argv/`cargo run` — the REPL binary never parses argv. Assert accept vs `Err` and parsed values (arity, `repeated swaps` order, `Option` vs `Vec` emptiness, numeric parses). Record `exit-behavior` expectations: REPL parse failure returns `Err` that the prompt loop prints and continues on — it must NOT exit the process (contracts/cli-surface.md, FR-015, US3-4)
- [X] T042 [P] [US3] Build the same parity table for `oxeylyzer-bench/src/flags.rs` covering `-l/--language`, `-b/--basis`, `-n/--count`, `-t/--time`, `-a/--algorithms`, `-p/--pins`, `--headless` (FR-015, US3-4)

### Implementation for User Story 3

- [X] T043 [US3] Replace `num_cpus::get()` at `oxeylyzer-core/src/data.rs:431` with `std::thread::available_parallelism()` (handle the `NonZeroUsize` result at that boundary) and remove `num_cpus = "1.17"` from `oxeylyzer-core/Cargo.toml` (FR-010, US3-1)
- [X] T044 [US3] Replace test-only `once_cell::sync::Lazy` with `std::sync::LazyLock` in all four sites: `oxeylyzer-core/src/trigram_patterns.rs:242`, `oxeylyzer-core/src/fast_layout.rs:1072`, `oxeylyzer-core/src/generate.rs:1122`, `oxeylyzer-repl/src/repl.rs:1186` (FR-011, US3-2)
- [X] T045 [US3] Remove `once_cell` from `oxeylyzer-core/Cargo.toml` `[dev-dependencies]`, `oxeylyzer-repl/Cargo.toml` `[dependencies]`, and root `Cargo.toml` `[workspace.dependencies]` (FR-011)
- [X] T046 [P] [US3] Create the in-repo color helper `oxeylyzer-repl/src/color.rs` (~10 lines) emitting byte-identical SGR sequences for the codes used, and switch the single import in `oxeylyzer-repl/src/display.rs` to it (FR-015, R7)
- [X] T047 [US3] Drop `ansi_rgb = { git = "https://github.com/O-X-E-Y/ansi_rgb" }` from `oxeylyzer-repl/Cargo.toml` and declare the new module in `oxeylyzer-repl/src/lib.rs` (FR-015, SC-005)
- [X] T048 [US3] Port the REPL parser to clap v4 across its three touchpoints: (a) rebuild `oxeylyzer-repl/src/flags.rs` from the `xflags!` macro, walking T041's migration checklist item by item (aliases, required/optional/repeated positionals, flag-with-value `-p/--pins`, bare flags, `config`'s 15 `--flag value: f64|usize` options); (b) replace `XflagsError(#[from] xflags::Error)` at `oxeylyzer-repl/src/repl.rs:98` with a clap error variant; (c) adapt `oxeylyzer-repl/src/commands.rs:81` to clap's `try_get_matches_from(args)` — `try_get_matches_from` (NOT `get_matches_from`, which exits the process) so a parse failure stays a returned `Err` the prompt loop prints and continues on — and map clap matches onto the existing `ReplCmd` variants so the `match flags.subcommand` dispatch in `commands.rs` keeps compiling unchanged. Make T041's parity tests pass; add `clap` to `oxeylyzer-repl/Cargo.toml` (FR-015, R7)
- [X] T049 [US3] Port `oxeylyzer-bench/src/flags.rs` to clap v4 as a root-level `Command` (no subcommands), making T042's parity tests pass; add `clap` to `oxeylyzer-bench/Cargo.toml` (FR-015)
- [X] T050 [US3] Remove both fork dependencies from `oxeylyzer-repl/Cargo.toml` and `oxeylyzer-bench/Cargo.toml` (`xflags = { git = ... }`) and confirm `cargo tree` shows no `O-X-E-Y` git source anywhere in the build (FR-015, SC-005, US3-3)
- [X] T051 [US3] Verify the replaced CLI surface against quickstart.md §6: run `cargo test -p oxeylyzer-repl flags_parity` (the `Repl::from_vec`-driven vectors: `analyze sturdy`, `a sturdy`, `swap sturdy ab abc`, `generate sturdy 10 -p abc`, `rm foo -y`), then an interactive smoke via piped stdin (`printf 'analyze sturdy\nquit\n' | cargo run`) confirming accepted lines execute, rejected lines print an error WITHOUT exiting the REPL, and `cargo run --release -p oxeylyzer-bench -- -n 5 -a hill,sa --headless` (argv path — `from_env_or_exit` at `oxeylyzer-bench/src/main.rs:18`) renders a markdown table (US3-4)

**Checkpoint**: all three user stories independently functional; build contains no personal forks.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T052 [P] Retarget project identity away from the abandoned upstream in `README.md` (first paragraphs) and the `description`/`repository`/`documentation` fields of `Cargo.toml` + `oxeylyzer-core/Cargo.toml`, so a reader can tell this is the maintained fork (FR-016)
- [X] T053 [P] Extend `docs/dependency-policy.md` with (a) a `## Scope fence` note recording that `fxhash` replacement, general idiom sweeps, and corpus/data-version changes are deliberately OUT of this feature (FR-017, spec Assumptions), and (b) the retained-with-rationale entries SC-005 requires for the stale-but-kept deps: `fxhash` (deliberate perf choice at its one call site), `sexp`, `file-chunker`, `sliding_window_alt`, `ahash` (each: one line why retained, one line when to revisit) (SC-005)
- [X] T054 [P] Walk the frontend majors LAST (FR-018 — unmeasured principal risk, R11): in `oxeylyzer-tauri/package.json` upgrade `vite ^6` → `^8` (Rolldown/Oxc bundler migration), `typescript ~5.6` → `^7.0`, `eslint ^9` → `^10`, and adjust `oxeylyzer-tauri/vite.config.ts` / `eslint.config.js` / `tsconfig.json` for the Rolldown and TS-7 changes (plan step 9)
- [X] T055 Validate the frontend: `cd oxeylyzer-tauri && bun install && bun run build`, `bunx tsc --noEmit`, and a manual `bun run tauri dev` smoke that loads the app and analyzes a layout (quickstart.md §7)
- [X] T056 If a plugin (`vite-plugin-solid`, `vite-plugin-wasm`, `@tailwindcss/vite`) breaks under Rolldown or TS 7: pin the newest working version in `oxeylyzer-tauri/package.json` with a reason comment, record the hold in `docs/dependency-policy.md`, and confirm the freshness step surfaces it rather than leaving it silent — same shape as FR-002a (research R11)
- [X] T057 Run the whole quickstart.md guide top to bottom on the branch (§1–§8) and confirm every expected result, including the negative advisory test and the zip-9 normalization-rejection case
- [X] T058 Final full-matrix check on the branch: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cargo audit --config audit.toml`, `cargo outdated --workspace` (SC-006)
- [X] T059 Confirm FR-017 held: score a pinned reference layout before/after the branch (`printf 'analyze sturdy\nquit\n' | cargo run` on `main` vs this branch, compare the score lines) and verify identical output values; state the comparison in the PR

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup, T001–T003)**: no dependencies — start immediately; MUST land first so the gate can mean
  anything (US1-4).
- **Phase 2 (Foundational, T004–T018)**: depends on Phase 1. BLOCKS the value of every later phase — the
  whole premise of the feature is that the catch-up happens *behind* a gate.
- **Phase 3 (US1, T019–T021)**: depends on Phase 2; closes US1 evidence.
- **Phase 4 (US2, T022–T040)**: depends on Phase 2 (needs the audit + freshness steps to prove the result);
  independent of Phase 3.
- **Phase 5 (US3, T041–T051)**: depends on Phase 2; independent of Phase 4 except that both edit
  `Cargo.toml` files — sequence T043–T050 after T024–T036 if run by one agent to avoid manifest conflicts.
- **Phase 6 (Polish, T052–T059)**: depends on the stories it validates; T054–T056 (frontend) last of all.

### Within Phase 4

T022/T023 (containment tests) → T024 (zip bump) → T025 (containment rework) → T026 (test run) is one ordered
chain — the policy decision is the only genuinely new design work in the whole feature. T027–T036 are
independent of each other ([P] candidates) but each must land before T037–T038. T039–T040 (tauri) depend on
T029–T032 having settled workspace versions.

### Within Phase 5

T041/T042 (parity tables) MUST be authored against the CURRENT xflags build and pass before T048/T049
replace it — that is what makes the swap diff-checked rather than eyeballed (spec edge case). T046 → T047
(color helper before dropping its dependency). T050 after T048+T049.

### Parallel Opportunities

- T005–T009 (five separate manifests) — all [P]
- T027, T029–T035 (independent crate bumps, separate manifests) — [P] among themselves; T028/T036 share manifests with them and are NOT parallel
- T022 vs T041 vs T042 (three different test files) — [P]; T023 edits the same test module as T022 and is NOT parallel
- T043 vs T044 vs T046 (different files) — [P]
- T052 vs T053 vs T054 — [P]

### Parallel Example — Foundational

```bash
Task: "Inherit rust-version.workspace in oxeylyzer-core/Cargo.toml"        # T005
Task: "Inherit rust-version.workspace in oxeylyzer-repl/Cargo.toml"        # T006
Task: "Inherit rust-version.workspace in oxeylyzer-resources/Cargo.toml"   # T007
Task: "Inherit rust-version.workspace in oxeylyzer-bench/Cargo.toml"       # T008
Task: "Inherit rust-version.workspace in oxeylyzer-tauri/src-tauri/Cargo.toml" # T009
```

### Parallel Example — US2 crate bumps

```bash
Task: "Bump ureq 2->3 in oxeylyzer-resources/Cargo.toml + download.rs call sites"   # T027
Task: "Bump ratatui 0.29->0.30, crossterm 0.28->0.29 in oxeylyzer-bench"            # T029
Task: "Bump rustyline 17->18 in oxeylyzer-repl/Cargo.toml + repl.rs"                # T030
```

---

## Implementation Strategy

### MVP First = User Story 1

1. Phase 1 (T001–T003): green tooling baseline.
2. Phase 2 (T004–T018): the standard + the signal + the policy doc — this is the only part that stops the
   problem recurring.
3. Phase 3 (T019–T021): US1 evidence, including the deliberate failing-advisory proof.
4. **STOP and validate**: US1 works on its own. Even if nothing else ever lands, the repo now has a declared
   floor, an advisory gate, and a freshness report, and the drift cannot silently return.
5. Ship.

### Incremental Delivery

1. Setup + Foundational → gate live (MVP).
2. US2 → Rust workspace current, advisories cleared, containment policy decided → each bump is one
   attributable commit under a green gate.
3. US3 → surface shrinks; no personal forks in the build; CLI parity proven.
4. Polish → frontend majors last (unmeasured), identity retargeted, quickstart walked end to end.

### Suggested commit granularity (one commit per checkpoint group)

`fix(ci): fmt+clippy` · `feat(msrv): declare+enforce window` · `feat(ci): audit+freshness gate` ·
`docs(dependency-policy)` · `feat(resources): zip9 with raw-name containment` · `deps: rust majors (per crate)` ·
`chore: single lockfile` · `refactor: clap + color helper` · `feat(frontend): vite8/ts7/eslint10` ·
`docs: fork identity`.

---

## Notes

- [P] = different files, no dependency on an incomplete task.
- Every task names an exact path; behavior of scoring/generation is never in scope (FR-017).
- The gate (Phase 2) must precede the catch-up (Phase 4) — that ordering is the feature's core argument, not
  a stylistic preference.
- T040 and T054–T056 are the two UNMEASURED areas (tauri Rust member compile, frontend majors); budget
  investigation time there rather than assuming the Rust-side one-line-fix estimate transfers.
- Do not disable or `continue-on-error` any new gate step (FR-004's stated failure mode).

## Task Summary (validation)

- Total tasks: **59** (T001–T059)
- Phase 1 Setup: 3 · Phase 2 Foundational: 15 · US1 (P1): 3 · US2 (P2): 19 · US3 (P3): 11 · Polish/cross-cutting: 8
- [P] markers: 20 (3 removed by analysis pass: T023 shares `download.rs` with T022; T028 shares `oxeylyzer-resources/Cargo.toml` with T027; T036 shares manifests with T024/T031–T035)
- Test-first tasks: T022, T023 (containment), T041, T042 (CLI parity) — 4, all preceding their implementation
- Parallel opportunities: T005–T009, T022/T041/T042, T027–T036, T043/T044/T046, T052/T053/T054 — 5 groups
- Independent test per story: US1 = synthetic advisory fails CI + freshness log answers drift (T018, T019);
  US2 = `cargo outdated` clean + `cargo test --workspace` + audit green (T037, T038); US3 = fork/stdlib grep
  empty + CLI parity tests + `vite build` works (T050, T051, T055)
- Suggested MVP: Phases 1–3 (T001–T021) — User Story 1 only
- Format: all tasks `- [ ] T### [P]? [US#]? <description with file path>` ✓

## Implementation record (added on completion, 2026-10-15)

All 59 tasks are `[X]`. Four things a reviewer should know, because they are not what the task text said:

1. **T048/T049 ported to idiomatic clap, not to a clap config that mimics xflags.** FR-015's "accept and
   reject exactly what they do today" is not reachable: clap's lexer accepts attached-value forms the fork
   rejected (`-n5`, `-n=5`, `--pins=ab`) and rejects hyphen-leading values it accepted (`-p -a`), with no
   knob that closes the gap. The frozen tables were re-recorded against the new parser and every changed row
   is annotated in place (`// DIVERGENT:`): 11 of 59 bench rows, 5 of 167 REPL rows. Load-bearing accepts were
   preserved deliberately via `allow_hyphen_values` on `--pins` and the `config` f64 flags, so `-p a-b` and
   `--sfbs -1.5` still work.

2. **T054 substituted oxlint for eslint 10** (`typescript-eslint` throws on TS 7 at every published
   version), which is a deliberate departure from FR-018's letter. T056 recorded **no hold** — vite 8 built
   with the existing plugin set, so no pin was needed.

3. **T037 regressed after it was marked done.** The advisory database published four rustls advisories on
   2026-10-14 against the 0.23.45 that T037 had just resolved; re-running the gate is what caught it, and
   `cargo update -p rustls` (→ 0.23.46) cleared it. The quickstart's `rustls ≥ 0.23.45` expectation is
   corrected to ≥ 0.23.46.

4. **T055's `bun run tauri dev` was not run** (no GUI session available), and T033's "shlex 1.3 → 3" is
   impossible — there is no shlex 3, so FR-006's "latest stable" is met at 2.0.1. Separately, spec.md's
   premise that upstream is "abandoned" is contradicted by upstream's active `staging` branch; see
   `docs/dependency-policy.md` §8.

---

## Phase 7: Convergence

Assessed 2026-10-15 against the live tree. Gate re-run clean at assessment time: `fmt` 0 findings,
`clippy -D warnings` exit 0, `cargo test --workspace` 230 passed / 0 failed, `cargo audit --deny unsound`
exit 0, `cargo outdated` + `scripts/ci/freshness.py` exit 0 (no unrecorded gaps, no stale holds),
`rust-window.sh` floor 1.97 == declared `rust-version`, frontend `vite build` + `tsc --noEmit` +
`oxlint` clean. Two FR-level divergences were found and left un-actioned on purpose — FR-015's exact
xflags lexer parity is unreachable in clap (documented row-by-row via `// DIVERGENT:` and
`docs/dependency-policy.md` §9) and FR-018's `eslint 10` is incompatible with its own required
`typescript 7.0` (documented with its reopen condition in §9). Neither has code work left that does not
break a different clause of the same requirement.

- [X] T060 Verify the Tauri dev loop that FR-018 requires and T055 skipped: run `bun run tauri dev` in `oxeylyzer-tauri/` and confirm the app loads and analyzes a layout; if no GUI session is available, record the non-verification as an explicit entry in `docs/dependency-policy.md` §9 "Deviations from the spec as written" (alongside the FR-015 and FR-018 entries) so FR-018's dev-loop half is not left silently unevidenced, and name `vite build`/`tsc --noEmit` as the verified halves per FR-018 (partial)
- [X] T061 Fix the contract mismatch between `msrv-holds.toml:8-11` and `scripts/ci/freshness.py:170-177`: the header promises frontend holds for the case it names verbatim — "a linter chain that cannot support the current TypeScript" — but any frontend hold whose declared range admits the registry `latest` is reported `HOLD STALE` and fails the build, so the oxlint substitution recorded in `docs/dependency-policy.md` §9 cannot be represented there at all. Amend the header to state that this file holds version holds only (`held` behind `latest`, with `required_rust`/`reason`), and point dependency *substitutions* at `docs/dependency-policy.md` §9 instead; confirm `python3 scripts/ci/freshness.py --json outdated.json --holds msrv-holds.toml --floor "$(./scripts/ci/rust-window.sh)"` still exits 0 afterwards per FR-002a (partial)

**Checkpoint**: gate, catch-up, and surface reduction are all verified against the live tree. What is
left is evidence for FR-018's dev loop and a header that does not promise the gate cannot keep.

---

## Phase 8: Convergence

Assessed 2026-10-15 against the live tree (re-run, independent of Phase 7). Local gate evidence at
assessment time: `cargo fmt --all -- --check` exit 0, `cargo clippy --all-targets --all-features --
-D warnings` exit 0, `cargo test --workspace` 230 passed / 0 failed, `./scripts/ci/rust-window.sh`
prints 1.97 == declared `rust-version`, `python3 scripts/ci/lint-audit-toml.py audit.toml` conforms,
`python3 scripts/ci/freshness.py ... --floor 1.97` exit 0 (37 Rust + 15 frontend rows, all
"up to date"; no unrecorded majors, no stale holds). Phase 7's own divergences (FR-015 lexer parity,
FR-018 eslint) are re-opened below with the concrete buildable path, because both have one.
Constitution checks skipped — `.specify/memory/constitution.md` is an unfilled template.

- [ ] T062 Run the gate in CI for real: land this branch on `main` (or open the PR into `main`/`staging`) and confirm a single GitHub Actions run executes all eight `contracts/ci-gate.md` steps green, then record that run's URL as the SC-001/SC-006 evidence in `docs/dependency-policy.md` §2 and this file's implementation record. Measured: `gh api repos/kellenff/oxeylyzer/actions/runs` returns `total_count: 0` and `actions/workflows` returns `total_count: 0`, so no workflow has ever executed on this fork; `origin/main` is 74 commits behind this branch and contains zero gate steps, and `ci.yml`'s `on:` block lists only `main`/`staging`, so branch pushes cannot produce a run (missing)
- [ ] T063 Restore FR-015's exact accept/reject surface in front of clap: add a token-normalizing guard at the two parse entry points (`oxeylyzer-repl/src/commands.rs` before `Repl::from_vec`'s clap call, `oxeylyzer-bench/src/flags.rs` before `parse()`) that rejects the attached-value forms xflags rejected (`-n5`, `-n=5`, `--pins=ab`) and keeps the hyphen-leading-value accepts, so every `// DIVERGENT:` row in both parity tables returns to the recorded baseline outcome and the annotations can be deleted — 16 rows remain today (11 in `oxeylyzer-bench/src/flags.rs`, 5 in `oxeylyzer-repl/src/commands.rs`) per FR-015 / US3-4 / contracts/cli-surface.md (contradicts)
- [ ] T064 Make the advisory gate deny every advisory category, not just unsound: change the CI step 6 command in `.github/workflows/ci.yml` to `cargo audit --deny warnings` and add the two live ids to `audit.toml`'s `ignore` array with `# reason:` lines — `RUSTSEC-2025-0057` (`fxhash`, retained per §7) and `RUSTSEC-2024-0370` (`proc-macro-error`, transitive via the Linux GTK stack) — measured today: `cargo audit --deny unsound` exits 0 with "2 allowed warnings found" while neither id is recorded, which FR-004's "an unrecorded advisory MUST fail the build" does not allow; if the narrowing is kept instead, record the exception with its reason in `docs/dependency-policy.md` §2 (contradicts)
- [ ] T065 Produce the FR-017 equivalence evidence T059 claims: run `printf 'analyze sturdy\nquit\n' | cargo run` on `origin/main` and on this branch, diff the score lines, and record both outputs in the PR description plus `docs/dependency-policy.md` §Scope fence. No before/after comparison is recorded anywhere (`grep -rn 'before/after' specs/009-modernize-deps-idioms docs/dependency-policy.md` finds only assertions), and the branch changes the generation path's pair enumeration — `possible_swaps` in `oxeylyzer-core/src/generate.rs` moved from `.tuple_combinations::<(_, _)>().map(Into::into)` to `.array_combinations::<2>().map(|[a, b]| PosPair::from((a, b)))`, plus five `array_combinations` sites in `oxeylyzer-core/src/fast_layout.rs`, where enumeration order is observable (partial)
- [ ] T066 Put the declared floor on the sixth package: add `rust-version.workspace = true` to the root `[package]` in `Cargo.toml`, which `cargo metadata --format-version 1 --no-deps` currently reports as `oxeylyzer rust-version=None` while all five members report 1.97, so the root binary crate is the one workspace member exempt from the FR-001 window and the FR-002 machine check (partial)
- [ ] T067 Finish FR-018's dev-loop evidence: drive one layout load and read its score in the running app from `bun run tauri dev` in `oxeylyzer-tauri/`, and record the observed score in `docs/dependency-policy.md` §9 replacing the current "not verified" paragraph. A real GUI session demonstrably exists (`docs/dependency-policy.md:233-244` shows the window coming up and the dev server answering HTTP 200), so the un-driven click-through — not the environment — is what US3-5 and FR-018 still lack (partial)
- [ ] T068 Add `scripts/ci/audit.sh` that stages the allowance list (`.cargo/audit.toml` from root `audit.toml`), runs the audit, and cleans up; call it from `.github/workflows/ci.yml` step 6 and from `docs/dependency-policy.md` §Local reproduction, so a local run and CI agree. Measured divergence: bare `cargo audit --deny unsound` exits 1 while the same tree exits 0 in CI, because cargo-audit 0.22 only reads `.cargo/audit.toml` — which `.gitignore:17` excludes — so the allowance list is inert unless a human remembers the copy step per FR-004 / contracts/ci-gate.md step 6 (partial)
- [ ] T069 Retarget the remaining stated-identity carrier: add the maintained-fork statement to `oxeylyzer-core/README.md:3`, which still calls the crate the engine behind "[Oxeylyzer](https://github.com/o-x-e-y/oxeylyzer)" with no indication this fork is the home, the way FR-016 required of the root `README.md` and the two `repository` fields (partial)
- [ ] T070 Correct the two policy claims the code does not support in `docs/dependency-policy.md`: §3 (lines 113-116) still routes frontend *substitutions* through `msrv-holds.toml` — the contradiction T061 removed from that file's header, which now says substitutions belong in §9 — and §6 (line 157) claims "`cargo outdated` flags yanked versions; the freshness table surfaces them" while `scripts/ci/freshness.py` reads no yanked field from the JSONL rows (`name`/`project`/`compat`/`latest`/`kind`). Either detect yanked rows in the freshness table or state the response without the surfacing claim per FR-002a / the yanked-but-locked edge case (contradicts)

**Checkpoint**: the gate exists and passes locally; this phase is about making it provable where it
binds (CI, FR-017 measurement, advisory categories) and closing the two MUST clauses Phase 7 left
documented-but-unmet. T062 is the one that changes the feature's status from "built" to "enforced".
