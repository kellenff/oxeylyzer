# Contract: CI Dependency Gate

**Object**: `.github/workflows/ci.yml` (extended) — the gate that stops drift recurring (US1).
This is the acceptance contract for FR-001..FR-005, FR-009, SC-001..SC-003, SC-006.

## Preconditions

- `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings` pass on `main`
  (FR-009 landed first; otherwise the gate is meaningless — US1-4).

## New/changed CI steps (order matters)

| # | Step | Command | Pass criterion | On failure |
|---|---|---|---|---|
| 1 | fmt | `cargo fmt --all -- --check` | clean | build red (pre-existing step) |
| 2 | clippy | `cargo clippy --all-targets --all-features -- -D warnings` | clean | build red (pre-existing step) |
| 3 | MSRV floor compute | parse `https://static.rust-lang.org/dist/channel-rust-stable.toml` | floor = oldest of latest 3 stable | network failure = explicit error, build red (fail-closed) |
| 4 | MSRV declare-match | `rust-version` in root `[workspace.package]` == floor from 3 | string equal, both values printed on mismatch | build red |
| 5 | MSRV build-check | `cargo +<floor> check --workspace` | compiles on floor toolchain | build red |
| 6 | Audit | `cargo audit` with `audit.toml` allowance list | zero unignored advisories | build red; id + crate named (US1-1) |
| 7 | Allowance lint | every `ignore` id in `audit.toml` has a `# reason:` comment | file conforms | build red (FR-004) |
| 8 | Freshness | `cargo outdated --workspace` diffed against `msrv-holds.toml` | no direct dep behind except recorded holds | report in log; unrecorded major gap = build red (SC-003) |

## Acceptance scenarios (from US1)

1. Commit touching a lockfile to add a dependency version covered by an unrecorded RustSec advisory →
   pipeline fails at step 6 naming RUSTSEC id and crate.
2. Same advisory appended to `audit.toml` with reason comment → step 6 passes; step 7 passes; reason
   discoverable in repo (file diff).
3. Fresh checkout: step 8 log alone answers "what are we behind on and by how much" with no manual
   registry query (US1-3).
4. Removing a recorded hold that has become stale (new latest fits window) → step 8 fails until file
   updated (FR-002a surfaceable).

## Degradation contract

- No network: steps 3/6/8 abort with a message stating `dependency gate unavailable (network); build not
  advisory-verified` and exit non-zero. They MUST NOT skip silently (`::notice::` + success is a contract
  violation: recreates undetected-drift).