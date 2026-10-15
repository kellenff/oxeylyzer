# Quickstart & Validation Guide: 009 Modernize Dependency Foundation

**Purpose**: how a maintainer (or CI) proves this feature works end-to-end, without hand-querying any
registry. Everything below is runnable today on the branch; ordering matches task phase order.

## Prerequisites

- Rust stable ≥ the computed window floor (see step 1); local toolchain at planning time: 1.99.
- `bun` for the Tauri frontend (already required by `oxeylyzer-tauri`).
- Network access for steps 1, 2, 5, 6 (fail-closed: each prints an explicit "gate unavailable (network)"
  error if denied).
- `cargo install cargo-audit cargo-outdated --locked` (CI installs the same; local mirror only for the
  walkthrough below).

## 1. Drift is now visible without touching crates.io by hand (US1-3, FR-005, SC-003)

```bash
# freshness: what are we behind on, and by how much
cargo outdated --workspace

# recorded MSRV holds (window wins), expected: {} at planning time
cat msrv-holds.toml

# the floor CI enforces, recomputed live
curl -s https://static.rust-lang.org/dist/channel-rust-stable.toml | grep -A2 '^\[pkg.rust\]' | head
grep -n 'rust-version' Cargo.toml            # declared mirror, exactly one location
grep -rn 'rust-version.workspace' */Cargo.toml oxeylyzer-tauri/src-tauri/Cargo.toml  # 5 inherit sites
```

**Expected**: the outdated table names current vs available for every direct dependency; the declared
`rust-version` equals the oldest of the three newest releases in the feed; the five member inherits
resolve (root `Cargo.toml` declares, the five crates inherit — "six" in the original text counted the
declaration itself).

Add `python3 scripts/ci/freshness.py --json <(cargo outdated --workspace --format json) --holds
msrv-holds.toml --floor "$(./scripts/ci/rust-window.sh)"` to see the same answer as one gate, including
the npm frontend half (§3 of `docs/dependency-policy.md`); `--self-check` verifies its range logic with
no network.

## 2. Advisory gate (US1-1/2, FR-003/4, SC-001)

```bash
mkdir -p .cargo && cp audit.toml .cargo/audit.toml   # cargo-audit 0.22 has no --config flag
cargo audit --deny unsound          # exit 0; --deny unsound is what makes it bite
cat audit.toml                      # allowance list; every ignore id has a "# reason:" line above it
```

**Corrected while walking this on 2026-10-14/15:** two things the text above got wrong.
(1) Plain `cargo audit` exits 0 even with an advisory present — RUSTSEC-2026-0190 is an
*unsound* informational, so the gate command is `cargo audit --deny unsound` (which is
what `ci.yml` runs). (2) The allowance list is **not** empty: it carries two reasoned
transitive entries (`RUSTSEC-2024-0429` glib via the Tauri GTK stack, `RUSTSEC-2026-0186`
memmap2 pinned by `file-chunker`), each with its `# reason:` line. `fxhash` and
`proc-macro-error` additionally raise *unmaintained* warnings that `--deny unsound`
does not fail on — see `docs/dependency-policy.md` §2 and §7.

**Negative test (the US1-1 check)**: pin `anyhow = "=1.0.102"` in the workspace manifest,
run `cargo generate-lockfile && cargo audit --deny unsound` → must fail (exit 1) naming
`RUSTSEC-2026-0190` and `anyhow`. Add `RUSTSEC-2026-0190` to `audit.toml` with a reason →
`cargo audit --deny unsound` and `python3 scripts/ci/lint-audit-toml.py audit.toml` both
pass. Revert. Verified this way on the branch.

## 3. Rust-side dependency currency (US2-1/2, FR-006/7/8, SC-004)

```bash
grep -E '^(zip|ureq|ratatui|crossterm|rustyline|itertools|directories|toml|shlex|nanorand|fancy-regex) ' \
  oxeylyzer-*/Cargo.toml Cargo.toml oxeylyzer-tauri/src-tauri/Cargo.toml   # versions per research R5/R6

cargo test --workspace           # FR-008
cargo clippy --all-targets --all-features -- -D warnings   # FR-009
cargo fmt --all -- --check                                  # FR-009
```

**Expected**: `anyhow ≥ 1.0.103` and `rustls ≥ 0.23.46` in `Cargo.lock`; full suite green on a
workspace with no nested lockfile.

**Corrections from walking this on 2026-10-14/15:**
- `rustls` is **≥ 0.23.46, not 0.23.45**. The advisory database moved on 2026-10-14 and
  published RUSTSEC-2026-0346/0347/0348/0349 against rustls < 0.23.46, so the version
  this spec was written against became vulnerable the day after it was resolved. The
  gate caught it; `cargo update -p rustls` clears it. This is FR-003 working, and it is
  the argument for auditing the *lockfile* on every build rather than once per feature.
- `shlex` is at **2.0.1, which is latest stable** — there is no shlex 3 (tasks.md T033
  assumed one). FR-006's requirement ("latest stable") is met at 2.0.1.

## 4. Archive containment — refuse policy (US2-3/4, FR-013/14, SC-007)

```bash
cargo test -p oxeylyzer-resources download
```

**Expected, all named cases**:
- traversal/absolute/doubled-separator/sideways/padded/single-parent-hostile entries are **rejected**
  (`traversal_escaping_root_is_rejected` … `doubled_separator_after_prefix_is_rejected`), and
  `files_outside_reports_everything_outside_the_destination` covers the report path.

**Correction on the drive-prefixed case.** The original text listed "drive-prefixed" among the named
hostile inputs, but no test exists for it and none can be portable: `Component::Prefix` — the arm
`is_contained` uses to refuse `C:/…` — only exists when compiling for Windows. On Unix the name
decomposes into ordinary components and lands *inside* the destination under a literal `C:` directory, so
the entry is contained rather than rejected. Both outcomes are safe, which is why the suite asserts the
invariant ("nothing lands outside the destination") instead of a per-platform verdict. Verified on
macOS: `data/C:/Windows/system32/evil.txt` and `data/D:/evil.txt`, both inside `dest`.
- `parent_segment_within_bounds_is_allowed`, `prefixed_entry_has_prefix_stripped`,
  `unprefixed_entry_extracts_at_root`, `archive_root_entry_is_skipped` still pass (not a blanket `..` ban).
- `mixed_valid_and_hostile_fails_without_partial_files`: abort, nothing written, and —
  FR-014 — `dest/.data-version` absent.
- The zip-9 normalization case (`oxeylyzer-data-files/../escaped.txt`) is rejected because validation runs
  on the raw name before any normalization.

## 5. Dependency-surface deletions (US3-1/2/3, FR-010/11/12/15, SC-005)

```bash
grep -rn 'num_cpus\|once_cell\|xflags\|ansi_rgb' --include='*.rs' --include=Cargo.toml . | grep -v target
  # expected: no hits
test ! -e oxeylyzer-tauri/src-tauri/Cargo.lock && echo "single lockfile: ok"
git check-ignore -v oxeylyzer-tauri/src-tauri/Cargo.lock   # nested lock gitignored, never returns
```

## 6. CLI parity — replacement must be invisible (US3-4, FR-015)

The REPL binary parses **no argv** — its grammar runs per interactive line (`shlex` → `Repl::from_vec` in
`oxeylyzer-repl/src/commands.rs`). Parity is proven by the table-driven tests, not by `cargo run -- <cmd>`:

```bash
cargo test -p oxeylyzer-repl flags_parity     # from_vec-driven accept/reject vectors
cargo test -p oxeylyzer-bench                 # bench parity vectors (argv path)
# interactive smoke (the REPL path):
printf 'analyze sturdy\nquit\n' | cargo run
printf 'rm foo -y\nbadcmd --nope\nquit\n' | cargo run   # rejection prints an error, REPL keeps running
# bench is a plain argv CLI:
cargo run --release -p oxeylyzer-bench -- -n 5 -a hill,sa --headless
```

**Expected**: the tables pass, and every row where the new parser's outcome differs from the old one is
annotated in place with `// DIVERGENT:` naming the old outcome (11 of 59 bench rows, 5 of 167 REPL rows).

**This expectation was corrected while implementing T048/T049.** FR-015's "accept and reject exactly what
they do today" is not achievable with clap: clap's lexer accepts attached-value forms the old parser
rejected (`-n5`, `-n=5`, `--pins=ab`) and rejects hyphen-leading values it accepted (`-p -a`), and no clap
knob closes that (`require_equals` is worse — it breaks every space-separated form). The decision taken was
**idiomatic clap with the divergences recorded**, not exotic opt-ins to mimic a dead parser. Real behavior
is preserved where the old acceptance was load-bearing: `allow_hyphen_values` on `--pins` and on the
`config` f64 weight flags, so `-p a-b` (pin the hyphen key) and `--sfbs -1.5` (negative weight) still work.
`docs/dependency-policy.md` §9 carries the full record.

Unchanged and still asserted here: for the REPL a rejection is a returned `Err` printed by the prompt loop
— never a process exit (`try_get_matches_from`, not `parse`); for bench the exit-on-error path is identical
(`--help` → 0, unknown flag → 2, bad value → 2, measured against `main`); `--headless` renders the markdown
table.

## 7. Frontend (FR-018) — was the plan's unmeasured risk; now measured

```bash
cd oxeylyzer-tauri && bun install && bun run build     # vite 8 / Rolldown
bunx tsc --noEmit                                      # typescript 7
bunx oxlint                                            # replaces eslint
bun run tauri dev                                      # manual desktop smoke (NOT run)
```

**Measured result**: all three automated steps pass with **no config edits at all** —
`vite build` compiles 39 modules and emits the wasm chunk (~460ms), `tsc --noEmit` is clean
under TypeScript 7.0.2, and oxlint reports 0 warnings on the 22 source files. The Rolldown
migration did **not** break `vite-plugin-solid`, `vite-plugin-wasm` or `@tailwindcss/vite`,
so T056 recorded **no plugin hold**; the freshness gate's frontend half is what would
surface one.

**eslint was replaced rather than upgraded.** FR-018 asked for `eslint 9→10`, and eslint 10
is unobtainable next to TypeScript 7: `typescript-eslint` throws `does not support TS 7.0`
even at its newest release (peer range `>=4.8.4 <6.1.0`). Given the choice between TS 7 +
oxlint and eslint 10 + TS ≤ 6, TS 7 was kept — the same "constraint wins, record it" shape as
FR-002a. `eslint.config.js` is deleted; oxlint's coverage was proven by planting unused
variables and a constant condition in both `.ts` and `.tsx` files, which it flags.

**Not verified**: `bun run tauri dev` needs a GUI session, so the desktop smoke test is out
standing. No CI job runs the frontend either, so CI is unaffected; a maintainer should load
the app once before releasing from this branch.

## 8. Identity + policy documents (FR-016)

```bash
head -20 README.md ; grep -n 'description\|repository' Cargo.toml oxeylyzer-core/Cargo.toml
cat docs/dependency-policy.md
```

**Expected**: the README names this repo as the maintained fork/trunk and no longer badges or links
`o-x-e-y/oxeylyzer` as if it were this code's home; the policy file covers MSRV, audit, freshness,
lockfile, containment, retained-with-rationale deps (`fxhash` and friends), and — new — §9's record of the
two places the implementation departs from the spec.

**Verified on this branch**: the CI badge was removed rather than repointed — it targeted
`github.com/oxey/oxeylyzer`, which 404s, and this fork's GitHub Actions has never run (zero workflow
runs, because Actions is off by default on forks), so any badge would assert a status that does not exist.
Note also that the spec's premise that upstream is "abandoned" is **not accurate**: upstream `main` is
quiet (2026-06-18) but its `staging` branch is active (26 commits, head 2026-10-03). What is true, and what
the README now says, is that this repo is the trunk, there is no `upstream` remote, and nothing inherits
from it automatically. See `docs/dependency-policy.md` §8.
