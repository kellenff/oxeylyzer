# Feature Specification: Resource Crate Test Coverage

**Feature Branch**: `feature/time-machine-resources-test-coverage`
**Created**: 2026-10-06
**Status**: Draft
**Input**: User description: "Cover the two remaining untested behaviors in oxeylyzer-resources: that the generated default config round-trips through the app config loader with its three data paths intact, and that all generated paths resolve under the platform root rather than only the override root."

## Background

This feature was originally scoped as full test coverage for `oxeylyzer-resources`
back when the crate had none. Two shipped features changed that:

- **Safe Data Archive Extraction** added 7 tests covering archive containment,
  prefix stripping, and refusal of hostile entries.
- **Data File Version Upgrades** added 18 tests covering version comparison, data
  currency, and the refresh decision.

The crate went from 0 tests to 25. Rather than re-specify work that is done, this
feature covers what measurement showed to still be untested.

## What Already Has Coverage

Recorded here so the gap is not re-litigated, and so a future change knows what
these tests are protecting:

- Archive entry containment, hostile-entry refusal, prefix stripping, root-entry
  handling, and no-prune-on-failure (`download.rs`)
- Version ordering across all comparison cases, and every data-currency state
  (`dirs.rs`)
- Paths resolving under the override root (`dirs.rs`)
- The refresh decision short-circuiting for current data, and the default config
  being written and not overwritten (`seed.rs`)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Freshly Generated Configuration Actually Works (Priority: P1)

A user installs the app on a clean machine. The app generates a configuration
file for them, pointing at the built-in corpus, the built-in layouts, and the
corpus configurations it just downloaded. If that generated file is malformed, or
if any of those three paths is wrong, the app cannot start or silently analyzes
nothing — the user sees an error about a config file the app itself wrote, which
is a baffling first-run experience.

The generated configuration must be known to load through the same path the app
uses at startup, with its data paths intact.

**Why priority**: This is the first thing a new user depends on, and it is the
only generated artifact the app produces. A broken default config makes the app
unusable on first run, with no workaround short of hand-writing the file.

**Independent Test**: Generate a configuration into a temp directory, load it
back through the app's config loader, and verify the corpus, layout, and corpus
config paths come back unchanged and point inside the data root.

**Acceptance Scenarios**:

1. **Given** a freshly generated configuration, **When** it is loaded through the
   app's config loader, **Then** it loads without error.
2. **Given** a freshly generated configuration, **When** it is loaded back, **Then**
   the corpus path, layout path, and corpus config path all point inside the
   resolved data root.
3. **Given** a freshly generated configuration, **When** it is loaded back, **Then**
   the values other than those three paths — analysis weights and precision
   settings — come back as the app's defaults.
4. **Given** a generated configuration written with the layout path expressed as a
   single path rather than a list, **When** it is loaded back, **Then** it loads,
   since the app accepts either form.

---

### User Story 2 - Path Resolution Holds On Real Installations (Priority: P2)

All path resolution is currently tested against a test-supplied root. What is
untested is the path the app actually takes on a user's machine, where
configuration and data live under the platform's own directories.

When the app resolves its directories, every path it hands out must sit under the
resolved root. If real resolution ever put a path outside its root, the app would
read and write locations the user did not intend, which no test using a supplied
root could catch.

**Why priority**: The override path is a test convenience. Verifying the same
containment property against what the platform actually reports closes the gap
without needing a real installation.

**Independent Test**: Ask the platform which directories it would use, assert every
path derived from that answer lies within it, without creating anything on disk.

**Acceptance Scenarios**:

1. **Given** the platform's reported directories, **When** every known path is
   derived, **Then** each sits under the reported configuration or data root.
2. **Given** the platform's reported directories, **When** the same path is
   requested repeatedly, **Then** each request returns an identical location.
3. **Given** the platform's reported directories, **When** data currency is
   checked against a root with no recorded version, **Then** data is reported as
   needing a download.

---

### Edge Cases

- **What happens if a generated path contains characters needing TOML escaping?**
  The configuration still loads and the path comes back byte-identical.
- **What happens when the data root path is a relative path?** Resolution still
  produces paths anchored at the root rather than the process working directory.
- **What happens when the configuration root and data root are the same
  directory?** Every derived path still resolves without collision.
- **What happens when a path is requested repeatedly?** Each call returns the
  same location, so callers do not disagree about where files live.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST generate a default configuration that loads
  successfully through the same loader the app uses at startup.
- **FR-002**: System MUST generate a default configuration whose corpus path,
  layout path, and corpus config path all lie within the resolved data root.
- **FR-003**: System MUST generate a default configuration that preserves the
  app's default analysis weights and precision settings through a load.
- **FR-004**: System MUST generate a layout path in a form the loader accepts
  whether the underlying setting holds one path or several.
- **FR-005**: System MUST survive path values requiring escaping in the generated
  configuration, and load them back unchanged.
- **FR-006**: System MUST derive every path from the directories the platform
  reports for this application, so that each derived path lies within the
  reported configuration or data root.
- **FR-007**: System MUST allow those platform-reported directories to be
  obtained without creating them, so that the containment property can be
  verified without writing to the user's filesystem.
- **FR-008**: System MUST return a consistent answer for the same path request
  across repeated calls.

### Key Entities

- **Generated Configuration**: The configuration file written on first run,
  pointing at the data files the app just fetched.
- **Resolved Directories**: The configuration root and data root for one
  installation, after platform resolution and creation.
- **Derived Path**: One of the fixed set of locations the app resolves relative to
  the configuration or data root.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A generated configuration loads through the app's own loader with
  no error, and its three data paths resolve inside the data root.
- **SC-002**: The layout path survives generation and loading as a list of
  exactly one entry, not an empty list or a single unwrapped string.
- **SC-003**: Analysis weights after a load equal the app's defaults, so a
  generated configuration is equivalent to no configuration.
- **SC-004**: A path requiring escaping is written and read back without
  alteration.
- **SC-005**: Every path derived from the platform's own reported directories lies
  within those directories, verified without creating anything on disk.
- **SC-006**: Repeated requests for the same path return identical locations.

## Assumptions

- The app's own configuration loader is the reference for whether a generated
  configuration is valid; tests use it rather than a hand-rolled parser.
- Platform directory resolution is delegated to the cross-platform directory
  library already in use, so "correct" means what that library reports for the
  running platform.
- Verifying containment against platform-reported directories does not create
  them, so the tests write nothing to the user's real configuration or data
  locations.
- Data paths are generated by joining the resolved data root with known relative
  locations, so they are expected to lie under that root by construction; the
  tests verify that rather than trusting it.
- Path escaping matters on platforms where a data directory can contain spaces or
  non-ASCII characters.
- The layout path setting accepts either a single path or a list of paths, and
  both forms are exercised.