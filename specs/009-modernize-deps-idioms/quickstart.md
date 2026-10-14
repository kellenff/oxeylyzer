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
`rust-version` equals the oldest of the three newest releases in the feed; the six inherits resolve.

## 2. Advisory gate (US1-1/2, FR-003/4, SC-001)

```bash
cargo audit                       # exit 0, zero unignored advisories
cat audit.toml                    # allowance list; every ignore id has a "# reason:" line above it
```

**Negative test (the US1-1 check)**: pin `anyhow = "=1.0.102"` in the workspace manifest, run the CI audit
locally (`cargo generate-lockfile && cargo audit`) → pipeline/step must fail naming
`RUSTSEC-2026-0190` and `anyhow`. Add `RUSTSEC-2026-0190` to `audit.toml` with a reason → passes. Revert.

## 3. Rust-side dependency currency (US2-1/2, FR-006/7/8, SC-004)

```bash
grep -E '^(zip|ureq|ratatui|crossterm|rustyline|itertools|directories|toml|shlex|nanorand|fancy-regex) ' \
  oxeylyzer-*/Cargo.toml Cargo.toml oxeylyzer-tauri/src-tauri/Cargo.toml   # versions per research R5/R6

cargo test --workspace           # FR-008
cargo clippy --all-targets --all-features -- -D warnings   # FR-009
cargo fmt --all -- --check                                  # FR-009
```

**Expected**: `anyhow ≥ 1.0.103`, `rustls ≥ 0.23.45` in `Cargo.lock` (the two known advisories clear);
full suite green on a workspace with no nested lockfile.

## 4. Archive containment — refuse policy (US2-3/4, FR-013/14, SC-007)

```bash
cargo test -p oxeylyzer-resources download
```

**Expected, all named cases**:
- traversal/absolute/drive-prefixed/doubled-separator/sideways/padded-hostile entries are **rejected**
  (`traversal_escaping_root_is_rejected` … `doubled_separator_after_prefix_is_rejected`).
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

**Expected**: every vector accepted/rejected identically to xflags (same outcome; for the REPL a rejection
is a returned `Err` printed by the prompt loop — never a process exit; for bench the exit-on-error path is
unchanged); `--headless` markdown table still renders.

## 7. Frontend (FR-018) — last, and unmeasured until now

```bash
cd oxeylyzer-tauri && bun install && bun run build     # vite 8 / Rolldown
bunx tsc --noEmit                                      # typescript 7
bun run tauri dev                                      # manual desktop smoke
```

**Expected**: clean build + dev app loads and analyzes. If a plugin breaks under Rolldown, the
**recorded-hold fallback** applies (package.json pin + reason in `docs/dependency-policy.md` + surfaced in
the freshness step) — same shape as FR-002a. This is the plan's principal estimating risk; see research R11.

## 8. Identity + policy documents (FR-016)

```bash
head -20 README.md ; grep -n 'description\|repository' Cargo.toml oxeylyzer-core/Cargo.toml
cat docs/dependency-policy.md
```

**Expected**: no "abandoned upstream" framing; policy file covers MSRV, audit, freshness, lockfile,
containment, and retained-with-rationale deps (`fxhash`).
