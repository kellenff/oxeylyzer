# Dependency Policy

The rule this repo lives by: **"current" is a checked property, not a remembered one.**
Every statement below is enforced by a step in `.github/workflows/ci.yml`; if a step
can be disabled, it will drift again. Reproduce any of them locally — see
*Local reproduction* at the end.

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

CI runs `cargo audit --deny unsound` over the resolved `Cargo.lock`. `--deny unsound`
matters: several advisories that concern this project (e.g. RUSTSEC-2026-0190 in
`anyhow`) are *informational* unsoundness notices, which cargo-audit reports but does
not fail on by default.

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
`--config` flag — it reads `./.cargo/audit.toml` — so CI copies the file into place
before auditing.

Post-upgrade, this list is expected to be **empty**.

## 3. Freshness report and hold procedure

CI runs `cargo outdated --workspace --format json` and pipes it through
`scripts/ci/freshness.py`, which prints a current-vs-available table (also posted to
the job summary) and enforces:

- a direct dependency a **major** behind latest stable → build red, unless recorded
  as a hold;
- minor/compat drift → reported, not gating (the catch-up work lands it deliberately);
- `cargo-outdated` failure → build red with the gate-unavailable message, never a
  silent pass.

No manual registry query is ever required: one CI log answers "what are we behind on,
and by how much".

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

`cargo outdated` flags yanked versions; the freshness table surfaces them. Response:
report it, then `cargo update -p <crate>` in the next maintenance pass. Yanked status
does not fail the build by itself (cargo-audit's `--deny yanked` is intentionally not
set: a yanked-but-unaffected transitive dependency is a schedule question, not a
security one).

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

This repository is the **maintained fork** of oxeylyzer. The original upstream is
abandoned; do not read it as the source of truth for issues, releases or CI.
Correspondingly, personal git forks are not permitted in the build: `xflags` and
`ansi_rgb` (both `github.com/O-X-E-Y`) are replaced by clap v4 and an in-repo color
helper. Verify with `cargo tree` — no `O-X-E-Y` source may appear.

## Scope fence

This feature is dependency foundation only. It deliberately excludes: `fxhash`
replacement, general Rust idiom sweeps, corpus cleaning changes, data-version changes,
and any edit to scoring or generation algorithms. Computed results must be identical
before and after (FR-017).

## Local reproduction

Requires `cargo install cargo-audit cargo-outdated --locked`.

```bash
# step 3 — the floor CI enforces
./scripts/ci/rust-window.sh

# step 4 — mirror vs floor
grep -n 'rust-version' Cargo.toml

# step 5 — the floor really builds
cargo +"$(./scripts/ci/rust-window.sh)" check --workspace

# steps 6+7 — audit with the allowance list, then lint the allowance list
mkdir -p .cargo && cp audit.toml .cargo/audit.toml
cargo audit --deny unsound
python3 scripts/ci/lint-audit-toml.py audit.toml

# step 8 — freshness, gating on unrecorded majors
cargo outdated --workspace --format json > outdated.json
python3 scripts/ci/freshness.py --json outdated.json \
  --holds msrv-holds.toml --floor "$(./scripts/ci/rust-window.sh)"
rm -f outdated.json .cargo/audit.toml
```

Fail-closed behaviour is testable offline: run the two network steps with no
connectivity and confirm each exits non-zero with
`dependency gate unavailable (network); build not advisory-verified`.
