# Phase 1 Data Model: Modernize Dependency Foundation (009)

This feature is a repository-maintenance pass; its "entities" are the small, machine-checked artifacts that
carry the dependency policy, plus the code-level contracts that change. There is no runtime data store.

## E1. Workspace MSRV declaration

**Carrier**: `[workspace.package] rust-version` in root `Cargo.toml`; every member inherits
`rust-version.workspace = true`.

| Field | Type | Rule |
|---|---|---|
| `rust-version` | string, `"MAJOR.MINOR"` | MUST equal the CI-computed floor = oldest of the 3 latest stable Rust releases (FR-001). Literal is a mirror; the dist feed is truth. |
| inheritance | `rust-version.workspace = true` | In `oxeylyzer-core`, `-repl`, `-resources`, `-bench`, `oxeylyzer-tauri/src-tauri` (FR-002, SC-002: exactly 1 authoritative location). |

**State transition**: none. CI check: computed floor ≠ declared → build fails with both values printed.
At planning time declared = `1.97` (stable 1.99 window).

## E2. Advisory allowance list

**Carrier**: `audit.toml` (`[advisories] ignore = [...]`), repo root, read by `cargo audit` in CI.

| Field | Type | Rule |
|---|---|---|
| `ignore[]` | RUSTSEC id (`RUSTSEC-YYYY-NNNN`) | MUST have a `# reason: <one line>` comment immediately above it (FR-004); CI lints the file for orphaned ids. |
| reason | comment text | names why accepted (e.g. `no upstream fix; score impact nil; revisit when 0.24 ships`). |

**Invariant**: every advisory present in `cargo audit` not listed in `ignore[]` fails the build (FR-003,
SC-001). Empty list is valid (the post-upgrade expectation is empty for `anyhow`/`rustls` — FR-007).
**Transition**: `advisory appears → CI red → maintainer either patches (preferred) or adds id+reason → CI green`.

## E3. MSRV hold record

**Carrier**: `msrv-holds.toml`, repo root.

| Field | Type | Rule |
|---|---|---|
| `[[hold]] crate` | string | dependency held below latest |
| `held` | semver | version actually resolved |
| `latest` | semver | newest stable |
| `required_rust` | string | `rust-version` of `latest` |
| `reason` | string | why window wins (FR-002a) |

**Invariants**: freshness step treats a listed crate as "held (recorded)", not "behind"; an entry whose
`latest.required_rust` now fits the window is **stale** and fails CI until removed. Not expected to be
non-empty at planning time (all current upgrades fit 1.97), but the file and checks exist per FR-002a.

## E4. Dependency policy document

**Carrier**: `docs/dependency-policy.md` (FR-016, SC-005 "recorded rationale" for retained deps).

Sections → requirement mapping:

| Section | Covers |
|---|---|
| MSRV window rule | FR-001, FR-002, FR-002a |
| Advisory gate + allowance procedure | FR-003, FR-004 |
| Freshness report + hold procedure | FR-005, FR-006 |
| Lockfile rule | FR-012 |
| Archive containment policy | FR-013, FR-014 |
| Retained dependencies with rationale (e.g. `fxhash`) | SC-005 |
| Fork identity statement | FR-016 |

## E5. Containment policy (code contract in `oxeylyzer-resources::download`)

Current signature surface is preserved; only the *decision function* changes.

| Element | Before | After |
|---|---|---|
| containment input | `enclosed_name()` (zip-normalized) | raw `file.name()` after `ZIP_INNER_PREFIX` strip |
| authority | `is_contained(rel)` | `is_contained(rel)` — unchanged, now fed the raw name |
| reject set | abs, drive, NUL, climb-above-dest | same + names zip 9 would normalize in-bounds (rejected pre-normalization) |
| on hostile | abort archive, no `.data-version` | identical (FR-014) |

**Invariant**: no path outside `dest` is ever written; `.data-version` written iff every entry honored
(FR-014). The six hostile-input tests keep reject semantics (FR-013); `parent_segment_within_bounds_is_allowed`
and the prefix-strip tests must keep passing (the validator is not a blanket `..` ban).

## E6. CLI surface contract

**Carrier**: `contracts/cli-surface.md` + new tests in the two `flags.rs` modules.

| Binary | Current grammar source | Replacement |
|---|---|---|
| `oxeylyzer` (repl) | `oxeylyzer-repl/src/flags.rs` `xflags!` (≈20 subcommands, aliases, repeated args, optional flags) | clap v4 builder/derive, same tree |
| `oxeylyzer-bench` | `oxeylyzer-bench/src/flags.rs` (1 subcommand, 7 options) | clap v4, same flat options |

**Invariant (FR-015)**: for every argument vector, new parser's accept/reject decision and resulting parsed
values equal the old parser's. Encoded as table-driven tests (see contract).

## Relationships

```
Cargo.toml (E1) ── read by ──> CI msrv job ──> dist feed (external)
audit.toml (E2) ── read by ──> CI audit job ──> Cargo.lock (workspace, E7)
msrv-holds.toml (E3) ── read by ──> CI freshness job (dedupes "behind" vs "held")
docs/dependency-policy.md (E4) ── documents ──> E1,E2,E3,lockfile,containment,retained deps
download.rs containment (E5) ── enforced on ──> archive bytes -> filesystem
flags.rs (E6) ── defines ──> argv -> command structs
```

## E7. Lockfile policy

**Carrier**: root `Cargo.lock` (committed, single). Nested `oxeylyzer-tauri/src-tauri/Cargo.lock` deleted and
gitignored. `oxeylyzer-tauri/bun.lock` retained (frontend ecosystem) and named in the policy doc (FR-012).