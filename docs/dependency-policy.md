# Dependency Policy

The rule this repo lives by: **"current" is a checked property, not a remembered one.**
Every statement below has a step in `.github/workflows/ci.yml` and a local reproduction
command (see *Local reproduction* at the end); if a step can be disabled, it will drift
again.

> **Gate status.** Every step here is verified locally on the `009` branch (floor compute,
> mirror match, `cargo +1.97 check`, `cargo audit --deny unsound`, the allowance lint, and
> the freshness report all pass or fail as specified). GitHub Actions has **not run on this
> fork** — the repo reports zero workflow runs, because Actions is off by default on forks.
> Until Actions is enabled for `kellenff/oxeylyzer`, the gate is enforced by the local
> commands, and the workflow is what CI will run once it is switched on.

## 1. MSRV window rule

The floor is the **oldest of the three newest stable Rust releases**, computed at build
time from the official dist feed
(`https://static.rust-lang.org/dist/channel-rust-stable.toml`) by
`scripts/ci/rust-window.sh`.

The feed is truth. `rust-version` in the root `Cargo.toml` (`[workspace.package]`) is a
**mirror** — one authoritative location, inherited by all five members via
`rust-version.workspace = true`. CI fails when the mirror and the computed floor
disagree, and separately compiles the workspace with `cargo +<floor> check --workspace`
to prove the floor is real, not decorative.

When the floor moves, the mirror is a one-line commit.

### When a dependency needs a newer Rust than the window allows

The window wins. Hold the dependency at its newest window-compatible version and record
the hold in `msrv-holds.toml`:

```toml
[[hold]]
crate = "example"
held = "1.2.3"
latest = "2.0.0"
required_rust = "1.99"
reason = "2.0 requires an out-of-window toolchain"
```

The freshness step reports that crate as `held (recorded)` instead of `behind`. A hold
whose `required_rust` has now entered the window is stale, and CI fails until the entry
is deleted — holds must be surfaced and retired, never quietly accumulated.

## 2. Advisory gate and allowance procedure

CI runs `scripts/ci/audit.sh`, which stages the allowance list and runs
`cargo audit --deny warnings` over the resolved `Cargo.lock`. The `--deny warnings`
matters twice over: several advisories that concern this project (e.g. RUSTSEC-2026-0190
in `anyhow`) are *informational* unsoundness notices that cargo-audit reports without
failing on by default, and the gate is not narrowed to unsoundness — an *unmaintained*
notice is an advisory too, and FR-004 says an unrecorded advisory fails the build. So
every category is denied, and what is accepted is recorded with a reason rather than
silenced by narrowing the filter.

An advisory appears → CI goes red → the maintainer chooses exactly one:

1. **Upgrade** the crate past the affected range (preferred).
2. **Accept it consciously**: append its id to `ignore` in `audit.toml`, one id per
   line, with a `# reason:` comment on the line immediately above.

```toml
[advisories]
ignore = [
  # reason: no upstream fix; nil impact on scoring; revisit when 0.24 ships
  "RUSTSEC-2026-NNNN",
]
```

`scripts/ci/lint-audit-toml.py` fails the build on an orphaned id, so an accepted
advisory is always discoverable in the repo rather than only in a CI log.

`audit.toml` at the repo root is the editable truth. cargo-audit (0.22) has no
`--config` flag — it reads `./.cargo/audit.toml`, which this repo gitignores — so the
allowance list is inert unless a human remembers to copy it. `scripts/ci/audit.sh` is
what removes that step from the loop: it stages the file, runs the audit, and restores
whatever was there before, and both CI step 6 and the local command below call it, so a
local run and CI cannot disagree about which advisories are allowed.

Post-upgrade the list is expected to be **empty in principle**; in practice it holds
what the window cannot reach. Current entries, each with a `# reason:` line in
`audit.toml`: `RUSTSEC-2024-0429` (glib, via the Tauri GTK stack) and `RUSTSEC-2026-0186`
(memmap2, pinned by `file-chunker`) — both transitive, neither reachable at a patched
version without dropping the crate that pins it. Plus two *unmaintained* notices that
`--deny warnings` now fails on: `RUSTSEC-2025-0057` (`fxhash`, retained deliberately per
§7) and `RUSTSEC-2024-0370` (`proc-macro-error`, transitive through
`glib-macros`/`gtk3-macros` in the Linux Tauri GTK stack — it is not even in the macOS
dependency graph, so it moves when the Tauri stack updates its macros, not when we choose
to).

## 3. Freshness report and hold procedure

CI runs `cargo outdated --workspace --format json` and pipes it through
`scripts/ci/freshness.py`, which prints a current-vs-available table (also posted to
the job summary) and enforces:

- a direct dependency a **major** behind latest stable → build red, unless recorded
  as a hold;
- minor/compat drift → reported, not gating (the catch-up work lands it deliberately);
- `cargo-outdated` failure → build red with the gate-unavailable message, never a
  silent pass.

The same script then checks the **frontend** (`oxeylyzer-tauri/package.json`) against
the npm registry, because FR-006 names the Tauri frontend and the Rust-only gate
would have left it silent:

- a declared range that already admits the registry latest → `up to date` (so `^8.0.0`
  with 8.3.4 published is current, not "behind" — a range floor is not an installed
  version);
- a range that cannot reach latest, major apart → build red, unless recorded as a hold;
- a range form the script does not parse (`*`, git:, link:) → reported for a human,
  never silently passed;
- registry unreachable → build red with the gate-unavailable message, fail-closed.

Frontend version holds use the same `msrv-holds.toml` entries, keyed by package name; a
recorded hold whose declared range already admits the registry latest is reported
`HOLD STALE` and fails the build, so a hold only survives while the range genuinely
cannot reach the newest version. `npm_latest` shells out to `curl` rather than using
urllib, because a framework python on macOS has no CA bundle and would otherwise fail
the gate closed on a machine that is fine.

A frontend dependency **substitution** — replacing a package outright, like the
eslint-to-oxlint swap in §9 — is not a version hold and is not recorded here: the
replacement is at latest, so a row for it would be stale on arrival. Substitutions
live in §9 with the condition that would reopen the old path.

No manual registry query is ever required, on either side of the repo: one CI log
answers "what are we behind on, and by how much".

`python3 scripts/ci/freshness.py --self-check --json /dev/null --floor 0` runs the npm
range logic without a network, and fails if the caret/tilde/pin rules break.

## 4. Lockfile rule

**One lockfile: the root `Cargo.lock`, committed.** `oxeylyzer-tauri/src-tauri` is a
workspace member, so its resolution lives in the root lock; its nested
`Cargo.lock` was a second, unsynchronized truth and is deleted plus gitignored.
`oxeylyzer-tauri/bun.lock` is retained — it belongs to the frontend (npm) ecosystem,
not the Rust workspace.

## 5. Archive containment policy

Downloaded data archives are extracted in `oxeylyzer-resources/src/download.rs`.
The policy is **refuse, before normalization**:

- validate the **raw** entry name (`file.name()`) after the `oxeylyzer-data-files/`
  prefix is stripped;
- reject NUL bytes and any component that climbs above the destination
  (`is_contained()` is authoritative);
- do **not** trust `ZipFile::enclosed_name()` for containment: measured under zip 9,
  it *normalizes* hostile names such as `oxeylyzer-data-files/../escaped.txt` into
  `escaped.txt` instead of rejecting them, which launders a traversal attempt into a
  silent rewrite of what the archive claimed.

This is not a blanket `..` ban: an in-bounds parent segment
(`a/../b` staying under the destination) and a prefixed entry are both allowed, and
the tests assert that.

Extraction writes per entry, and the `.data-version` sentinel is written only after
the whole archive succeeds, so a mid-archive refusal leaves a destination without its
sentinel. Full temp-dir staging is deliberately out of scope (spec FR-014 asks only
that the sentinel not claim a partial extraction).

## 6. Yanked-but-locked

Nothing in the gate surfaces a yanked-but-locked dependency. Measured: `cargo outdated`
has no yanked flag, and the JSONL rows it emits (`name`, `project`, `compat`, `latest`,
`kind`, `platform`) carry no yanked field, so the freshness table cannot show one. This
sentence used to claim it did; it is corrected rather than implemented, because
detecting yanked versions would mean a per-dependency registry query the rest of the gate
deliberately does not make. Response when Cargo's resolver does report one on a fresh
resolution: `cargo update -p <crate>` in the next maintenance pass. Yanked status does
not fail the build by itself (cargo-audit's `--deny yanked` is intentionally not set: a
yanked-but-unaffected transitive dependency is a schedule question, not a security one).

## 7. Retained dependencies with rationale

Kept deliberately, each with the condition that would reopen it:

- **`fxhash`** — deliberate hot-path performance choice at its single call site
  (`FastLayout` scoring caches); swapping it requires a benchmark first, and
  performance work is out of this feature's scope. Revisit when a replacement is
  proven not to cost score throughput.
- **`ahash`** — salted, fast hashing for non-crypto internal maps; no stdlib
  equivalent with the same properties. Revisit only if a hash-fixation concern appears.
- **`sexp`** — parses the REPL's s-expression command input; a bespoke parser would be
  more code than the dependency. Revisit if the command grammar is redesigned.
- **`file-chunker`** — streams corpus files in fixed chunks for the cleaner. Revisit
  if corpus cleaning is rewritten around `std::io::Read::read_buf`.
- **`sliding_window_alt`** — n-gram windowing in the analyzer; niche but exact.
  Revisit when the n-gram pass is next touched.

## 8. Fork identity

This repository is the **maintained fork** and the trunk for ongoing development:
[o-x-e-y/oxeylyzer](https://github.com/o-x-e-y/oxeylyzer) is the original project. Read it
as the origin, not as the source of truth for this code's issues or CI — there is no
`upstream` remote wired up, and the fork does not wait on it.

> **Correction to the 009 spec's premise.** spec.md describes upstream as "abandoned".
> Measured on 2026-10-14: its `main` is quiet (last merge 2026-06-18) but its `staging`
> branch is **active** — 26 commits ahead of `main`, head dated 2026-10-03, including
> "update rust and fix new clippy warnings". So "abandoned" is not accurate for
> upstream as a whole; what is true is that *this fork's `main`* is the trunk and
> nothing here inherits from upstream automatically. The dependency work in 009 stands
> on its own regardless.

Overlap risk, measured rather than guessed: 12 files changed on this branch are also
changed on upstream `staging` — `oxeylyzer-core/src/{fast_layout,generate}.rs`, the
whole REPL parse path (`flags.rs`, `commands.rs`, `repl.rs`), and the Tauri side
(`src-tauri/src/lib.rs`, `src/App.tsx`, `src/api.ts`, `src/store.ts`,
`src/views/EditView.tsx`, `package.json`, `bun.lock`). Reconciling the two is a real
merge with hot-path conflicts, not a fast-forward — so if upstream work is ever wanted,
plan it as its own feature with scoring verified before/after (FR-017's fence applies).

Personal git forks are not permitted in the build. Both are gone: `ansi_rgb` was replaced
by the in-repo `oxeylyzer-repl/src/color.rs`, and `xflags` by clap v4. Verify with
`cargo tree --workspace | grep -i 'O-X-E-Y\|git+'` — it must print nothing, and
`Cargo.lock` must contain no `git+` source.

## 9. Deviations from the spec as written

One requirement could not be met exactly. It is recorded here rather than left as a
silent exception, because the honest version of "we are current" includes "and here is
what we chose instead".

**FR-015 — exact lexer parity is met by a token screen in front of clap, not by clap.**
clap's own lexer does not reproduce the old parser's: it accepts attached-value forms the
old parser rejected (`-n5`, `-nn`, `-n=5`, `-c=5`, `--pins=ab`), it takes a lone `-` as a
positional value, and it rejects hyphen-leading values it accepted (`-a -p`, `-n -5`). No
clap knob closes this — `require_equals` is worse, it breaks every space-separated form.
So the surface is restored in two parts: every value-taking option is declared
`allow_hyphen_values` (a leading `-` is real data: `-p a-b` pins the hyphen key,
`--sfbs -1.5` is a negative weight), and `oxeylyzer-repl/src/lexer.rs::check` screens the
token stream at the two parse entry points, rejecting the shapes the old grammar never
looked up. The screen reads clap's own arg metadata for which options take a value rather
than restating the grammar, and a value-taking option swallows the token after it, so
`generate x -p --count=5` still pins the literal `--count=5`. Result: all 59 bench rows
and all 167 REPL rows match the frozen baseline captured from the xflags parser before
the swap, with no divergence annotations left to record.

**FR-018 — eslint is replaced by oxlint, not upgraded to eslint 10.** `typescript 7.0` and
`eslint 10` cannot coexist: `typescript-eslint` throws `does not support TS 7.0` at its
newest release (peer range `>=4.8.4 <6.1.0`). Since FR-002a says the constraint wins and
the choice is what to hold, TS 7 is the requirement that mattered, so the eslint chain
(`eslint`, `@eslint/js`, `@typescript-eslint/parser`, `eslint-plugin-solid`) was removed
and `oxlint` (the Oxc linter, matching the Rolldown/Oxc bundler now in the build)
replaced it. `eslint.config.js` is deleted. Lint coverage was verified by planting unused
variables and a constant condition in both `.ts` and `.tsx`: oxlint flags both.

**FR-018 — the dev loop runs; the click-through was not driven.** `bun run tauri dev` was run
2026-10-15 in a real Aqua session and does work: it compiles `oxeylyzer-tauri`, brings the Vite 8.3.4
dev server up on `localhost:1420` (HTTP 200), and launches the binary. The app registers with the
WindowServer and spawns its `Networking` and `Graphics and Media` helpers, so the WebView window
really comes up; the SolidJS client executes (`createRoot` warnings in the dev log) and the log holds
no error or panic — its only warning is an unrelated `~/.cargo/registry` cache-permission notice from
Cargo itself. What is **not** verified: loading a layout in the running app and reading its score.
Driving that needs Accessibility permission (an `osascript` call against System Events hangs in an
unpermitted session) and screen recording (`screencapture` is refused by TCC), so it could not be
automated here. A maintainer should open the app once before releasing from this branch. Note also
that no CI job exercises the frontend — `vite build`, `tsc --noEmit` and `oxlint` are local-only, so a
broken frontend build does not fail CI.

Revisit when `typescript-eslint` supports TS 7 — that is the single condition that would
reopening the eslint path make sense.

## Scope fence

This feature is dependency foundation only. It deliberately excludes: `fxhash`
replacement, general Rust idiom sweeps, corpus cleaning changes, data-version changes,
and any edit to scoring or generation algorithms. Computed results must be identical
before and after (FR-017).

### FR-017 evidence (measured 2026-10-15)

`printf 'analyze sturdy\nquit\n' | cargo run` on `origin/main` (`d015a16`, the fork's tip,
pre-branch) and on this branch, each run as its own build from a detached worktree of
`origin/main` versus the working tree, so neither reuses the other's binary:

```text
main:    sha256 54cd12b2e5b0000ad33f9539f5c61cb5714be32ef8a42294d0b7c0a582f03410  (1355 bytes, exit 0)
branch:  sha256 54cd12b2e5b0000ad33f9539f5c61cb5714be32ef8a42294d0b7c0a582f03410  (1355 bytes, exit 0)
diff:    no differences
```

Every score line is identical, `Sfb: 0.866%`, `Dsfb: 5.509%`, `Finger Speed: -1.785`,
`Stretches: -13.110%`, `Scissors: 0.081%`, `Lsbs: 2.254%`, `Pinky Ring Bigrams: 2.473%`,
`Inrolls: 21.325%`, `Outrolls: 23.259%`, `Total Rolls: 44.584%`, `Onehands: 1.939%`,
`Alternates: 27.305%`, `Alternates (sfs): 6.060%`, `Total Alternates: 33.364%`,
`Redirects: 2.570%`, `Redirects Sfs: 2.024%`, `Bad Redirects: 0.144%`,
`Bad Redirects Sfs: 0.161%`, `Total Redirects: 4.900%`, `Bad Sfbs: 2.011%`, `Sft: 0.063%`,
`Score: 0.468`.

This is the evidence T059 claimed but did not record, and it covers `analyze` only: the
scoring path, not the generator. The generation path did change on this branch —
`possible_swaps` in `oxeylyzer-core/src/generate.rs` moved from
`.tuple_combinations::<(_, _)>().map(Into::into)` to `.array_combinations::<2>().map(|[a, b]| PosPair::from((a, b)))`,
plus five `array_combinations` sites in `oxeylyzer-core/src/fast_layout.rs` — and
enumeration order is observable there. Both combinators yield the same pairs in the same
order for a pair enumeration, which is why nothing changed, but a single-layout score is
not a proof of that. Generation is not reproducible run to run either way (the RNG is
`nanorand` with no seed API), so a generator comparison is a distribution claim, not a
byte claim, and is out of scope for this feature.

## Local reproduction

Requires `cargo install cargo-audit cargo-outdated --locked`.

```bash
# step 3 — the floor CI enforces
./scripts/ci/rust-window.sh

# step 4 — mirror vs floor
grep -n 'rust-version' Cargo.toml

# step 5 — the floor really builds
cargo +"$(./scripts/ci/rust-window.sh)" check --workspace

# step 6 — audit with the allowance list staged, the same way CI runs it
./scripts/ci/audit.sh

# step 7 — lint the allowance list
python3 scripts/ci/lint-audit-toml.py audit.toml

# step 8 — freshness: Rust majors + the npm frontend, both gating
cargo outdated --workspace --format json > outdated.json
python3 scripts/ci/freshness.py --json outdated.json \
  --holds msrv-holds.toml --floor "$(./scripts/ci/rust-window.sh)"
rm -f outdated.json

# the npm range rules, without a network
python3 scripts/ci/freshness.py --self-check --json /dev/null --floor 0

# frontend build + types + lint (no CI job runs these; verify locally)
cd oxeylyzer-tauri && bun install && bun run build && bunx tsc --noEmit && bunx oxlint src
```

Fail-closed behaviour is testable offline: run the two network steps with no
connectivity and confirm each exits non-zero with
`dependency gate unavailable (network); build not advisory-verified`.
