# Specification Quality Checklist: Resource Crate Test Coverage

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-06
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

**Rescoped 2026-10-06.** The original queue entry asked for broad coverage of
`oxeylyzer-resources` "path resolution under platform and override roots,
sentinel-gated download behavior, default config generation, and extraction
results." Measurement showed most of that already exists:

| Originally queued | Status |
|---|---|
| Extraction results and containment | Covered — 7 tests in `download.rs` |
| Sentinel-gated download behavior | Covered — version and currency tests in `dirs.rs`, decision tests in `seed.rs` |
| Path resolution under override root | Covered — `every_path_resumes_under_the_data_root` |
| Default config generation | **Partially** — file existence only, contents never verified |

Two genuine gaps remained and became this feature's scope. The existing
`ensure_data_writes_a_default_config` asserts only that `config.toml` exists;
nothing loads it through the app's own config loader, so a malformed or
path-broken default config would pass every test in the crate.

**A suspected bug was investigated and dismissed.** `default_config_toml`
generates a layout path of `static/layouts/english/*.dof`. The published data
archive was downloaded and inspected: it ships an `english` layout directory
containing 146 `.dof` files, plus `static/language_data/english.json` and 58
corpus config `.toml` files. All three generated paths resolve against the real
release. No defect here — worth recording because the directory name looked
absent from a partial listing and would otherwise have become a phantom bug in
this spec.

**Sentinel-gated download is now doubly covered** and is explicitly listed as
already-covered rather than queued again.
