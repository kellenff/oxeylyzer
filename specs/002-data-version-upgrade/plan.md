# Implementation Plan: Data File Version Upgrades

**Branch**: `feature/time-machine-data-version-upgrade` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/002-data-version-upgrade/spec.md`

## Summary

Decide whether an installation's data files need downloading by comparing the
version recorded in `.data-version` against the current version, instead of
checking only that the file exists.

**Technical approach**: parse the recorded sentinel as a dot-separated version,
order-compare it against the build's current version, and download when the
recorded one is absent, unreadable, or older. Expose the comparison as a
`DataCurrency` enum so callers stop inferring intent from a boolean.

**Naming change**: `OxeylyzerDirs::is_first_run()` becomes
`OxeylyzerDirs::needs_data()`. Under version semantics "first run" is a claim
about install history, but the real question is whether data must be fetched.
The Tauri app gates its download on this at `lib.rs:1608` and must be updated in
step.

## Technical Context

**Language/Version**: Rust, edition 2024, `stable` per `rust-toolchain.toml`
**Primary Dependencies**: none added — `std` only for version parsing
**Storage**: filesystem — the `.data-version` sentinel in the data root
**Testing**: `cargo test` — in-crate unit tests
**Target Platform**: Linux, macOS, Windows
**Project Type**: library crate (`oxeylyzer-resources`), consumed by `oxeylyzer-repl` and `oxeylyzer-tauri`
**Performance Goals**: no measurable cost — one file read and a numeric compare at startup
**Constraints**: no new dependencies; the extraction path and its containment guarantee are unchanged
**Scale/Scope**: three files in `oxeylyzer-resources`, plus one call site in `oxeylyzer-tauri`

## Design

### Version representation

Dot-separated unsigned integers, compared component-wise: longer is newer when a
prefix matches, otherwise the first differing component decides. This handles
`1.0.0` → `1.0.1` and `1.0` → `1.0.1` without a semver dependency.

Non-numeric or empty components make the recorded version unreadable, which
FR-006 maps to "absent" and therefore to downloading.

```rust
/// Orders `a` against `b`, returning `None` when either is not dot-separated numbers.
fn compare_versions(a: &str, b: &str) -> Option<Ordering>
```

`None` covers both the recorded and the current version being unparseable. The
current version is a build constant under our control, so if it ever fails to
parse the safest reading is to download rather than assume currency.

### Currency

```rust
pub enum DataCurrency {
    /// No version recorded — a fresh installation.
    Absent,
    /// Recorded version is older than the current one.
    Stale { installed: String },
    /// Recorded version matches or exceeds the current one.
    Current { installed: String },
}

impl DataCurrency {
    /// True when data must be downloaded.
    pub fn needs_download(&self) -> bool
}
```

`Absent` and `Stale` need a download; `Current` does not. A recorded version
newer than current lands in `Current`, satisfying FR-005's no-rollback rule
without a separate variant.

### Where each requirement lives

| Requirement | Change |
|---|---|
| FR-001 | `DATA_VERSION` const already written on successful extraction; now also read |
| FR-002, FR-005, FR-006 | `compare_versions` + `DataCurrency` in `dirs.rs` |
| FR-003, FR-004 | `ensure_data` in `seed.rs` branches on `needs_download()` |
| FR-007 | Extraction already overwrites; no change |
| FR-008 | Extraction never prunes; no change |
| FR-009, FR-010 | Sentinel written only after success — already true; failure propagates before the write |
| FR-011 | `OxeylyzerDirs::needs_data()` replaces `is_first_run()` |
| FR-013 | Existing `ResourceError` variants cover it |
| FR-014 | `ensure_data` stays idempotent |

FR-009 and FR-010 need no new work. The sentinel is written inside
`download_and_extract` after the extraction loop, so any failure propagates
before the write, and the next startup re-reads the old version and retries.

### Call sites

`oxeylyzer-tauri/src-tauri/src/lib.rs:1608`:

```rust
if dirs.is_first_run() {          // before
if dirs.needs_data() {            // after
```

No other caller. `oxeylyzer-repl` calls `ensure_data` unconditionally and relies
on the internal check, so it needs no change.

## Project Structure

```text
oxeylyzer-resources/
├── src/
│   ├── lib.rs      # unchanged
│   ├── dirs.rs     # DataCurrency, compare_versions, needs_data()
│   ├── download.rs # unchanged — DATA_VERSION written and read
│   └── seed.rs     # ensure_data branches on currency
oxeylyzer-tauri/
└── src-tauri/src/lib.rs   # one call site renamed
```

## Verification

```bash
cargo test -p oxeylyzer-resources
cargo clippy -p oxeylyzer-resources
cargo test -p oxeylyzer-tauri   # renamed call site still compiles
cargo test --workspace         # 178 existing tests still pass
```

**New tests** — all pure, no filesystem needed except where a temp dir is the
point. `compare_versions` first, since ordering rules are where this breaks:

| Test | Input | Expect |
|---|---|---|
| older patch | `1.0.0` vs `1.0.1` | `Less` |
| older minor | `1.0.0` vs `1.1.0` | `Less` |
| fewer components | `1.0` vs `1.0.1` | `Less` |
| equal | `1.0.0` vs `1.0.0` | `Equal` |
| newer recorded | `2.0.0` vs `1.0.0` | `Greater` — downgrade, no download |

Then the enum mapping: `Absent.needs_download()` is true, `Stale` is true,
`Current` is false.

Then `ensure_data` against a temp dir with `OXEYLYZER_DATA_DIR`-style override,
checking the sentinel content drives the next call's decision.

## Complexity Tracking

No constitution violations — `.specify/memory/constitution.md` is still the
unmodified template.

| Choice | Simpler alternative rejected because |
|---|---|
| `DataCurrency` enum | A bool loses the distinction between "never downloaded" and "downloaded but stale", which FR-011 requires and which the Tauri startup screen wants for messaging |
| Component-wise integer compare | A `semver` dependency for comparing two dot-separated triples; `std::cmp::Ordering` covers it |
| Hand-bumped `DATA_VERSION` | Deriving from the crate version re-downloads all data on every patch release |
| `needs_data()` over keeping `is_first_run()` | The old name asserts install history; under version semantics it answers a different question |