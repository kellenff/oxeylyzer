# Feature Specification: Data File Version Upgrades

**Feature Branch**: `feature/time-machine-data-version-upgrade`
**Created**: 2026-10-06
**Status**: Draft
**Input**: User description: "Data File Version Upgrades. Compare the installed data version sentinel against the current version and re-download when data is stale, instead of only checking that the sentinel exists. Relevant files: oxeylyzer-resources/src/seed.rs, download.rs, dirs.rs. Focus on this feature only; do not modify other features."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Stale Data Files Are Refreshed (Priority: P1)

The app ships with a set of built-in data files — layouts, corpora, language
data — that grow and change as new languages and layouts are added. The app
records which version of those files an installation has. A user on an older app
version launches after a newer release has updated the data files, and their app
should bring them up to date automatically rather than analyzing layouts against
stale reference data indefinitely.

Today the app only checks whether any data has ever been downloaded, so an
installation is never refreshed no matter how far behind it is.

**Why priority**: Stale data silently degrades every score the app reports, and
the user has no way to see or correct it. This is the entire point of the
feature.

**Independent Test**: Install data at one recorded version, bump the current
version, run the bootstrap step, and verify the data is re-fetched and the
recorded version advances.

**Acceptance Scenarios**:

1. **Given** an installation whose recorded data version is older than the current
   version, **When** the app starts, **Then** it re-downloads the data files and
   records the new version.
2. **Given** an installation whose recorded data version matches the current
   version, **When** the app starts, **Then** it performs no download.
3. **Given** an installation with no recorded version, **When** the app starts,
   **Then** it downloads and records the current version.
4. **Given** an installation whose recorded version is newer than the current
   version — a user who downgraded the app — **When** the app starts, **Then** the
   data is treated as usable and no download occurs.

---

### User Story 2 - Refresh Replaces Built-In Data In Place (Priority: P1)

The built-in data files and the user's saved layouts share one directory, so a
refresh cannot update the corpus without also touching the layout directory. The
accepted approach is to re-extract the new data release over the existing data
directory: every file the release ships is replaced with the new version, and
files the release does not ship are left alone.

A user should understand the consequence plainly: a saved layout whose path
coincides with a path in the new data release is replaced by the released file.
Layouts saved under their own names are unaffected, because a generated layout
rarely shares a filename with a shipped one.

**Why priority**: This is the behavior every other requirement depends on, and
it is the deliberate choice behind this feature. Stating it explicitly means the
tradeoff is visible rather than discovered after a user's layout disappears.

**Independent Test**: Place a user layout at a path the release does not ship and
one at a path it does, refresh, and verify the first survives and the second is
replaced by the released file.

**Acceptance Scenarios**:

1. **Given** a user has saved a layout at a path the new release does not ship,
   **When** the upgrade runs, **Then** the saved layout is still present and
   unchanged afterwards.
2. **Given** a saved layout sits at a path the new release also ships, **When** the
   upgrade runs, **Then** the released file replaces it.
3. **Given** the data directory holds a file the release does not ship, **When**
   the upgrade runs, **Then** that file is left untouched rather than pruned.
4. **Given** a user has saved a layout, **When** the app restarts with the upgraded
   data, **Then** the surviving saved layouts still appear among the loaded
   layouts.

---

### User Story 3 - A Failed Refresh Does Not Corrupt the Installation (Priority: P2)

A data refresh needs the network and can fail partway through. The user may be on
a plane, behind a proxy, or have the download stall. When that happens the
existing installation must keep working, with its data still usable and its
recorded version unchanged, so the update simply retries later.

**Why priority**: This is the failure path of Story 1. Without it, a routine
network hiccup could leave a user unable to launch the app.

**Independent Test**: Interrupt a refresh at the download and extraction stages
and verify the previous data and version remain intact and usable.

**Acceptance Scenarios**:

1. **Given** a refresh fails during download, **When** the failure surfaces,
   **Then** the previously installed data remains intact and the recorded version
   is unchanged.
2. **Given** a refresh fails during extraction, **When** the failure surfaces,
   **Then** the previously installed data remains intact and the recorded version
   is unchanged.
3. **Given** a refresh fails, **When** the app starts again with the network
   available, **Then** the refresh is retried rather than being treated as done.

---

### Edge Cases

- **What happens when the recorded version file is empty or corrupt?** It is
  treated as unknown, which triggers a refresh.
- **What happens when the recorded version is newer than the current version?**
  Treated as usable — a user who downgraded the app should not have their data
  forcibly rolled back.
- **What happens when the version comparison is not meaningful, such as a
  development build with no version?** The existing behavior is preserved: data
  is downloaded when absent and not re-fetched when present.
- **What happens when a user has saved a layout at a path the new data release
  also ships?** The released file replaces it. This is the accepted tradeoff of
  refreshing in place, and the reason saved layouts are given distinct names.
- **What happens when the data directory contains files the archive no longer
  ships?** They are left alone rather than pruned, so removing a built-in from a
  release does not silently delete it from existing installations.
- **What happens when a refresh succeeds but the recorded version cannot be
  written?** The next start refreshes again rather than silently skipping.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST record which version of the data files an installation
  has, and MUST write that record only after a download completes successfully.
- **FR-002**: System MUST determine whether an installation's data is current by
  comparing the recorded version against the current version.
- **FR-003**: System MUST download when no version is recorded.
- **FR-004**: System MUST download when the recorded version is older than the
  current version.
- **FR-005**: System MUST NOT download when the recorded version matches or is
  newer than the current version.
- **FR-006**: System MUST treat a recorded version that cannot be read or parsed
  as absent, and download.
- **FR-007**: System MUST replace the data directory's contents by re-extracting
  the release over them, so every file the release ships ends up at the released
  version.
- **FR-008**: System MUST NOT remove files the data directory contains that the
  release does not ship.
- **FR-009**: System MUST leave the previously installed data and its recorded
  version unchanged when a refresh fails.
- **FR-010**: System MUST retry a failed refresh on the next start rather than
  recording it as complete.
- **FR-011**: System MUST report whether the installation's data is current,
  so callers can distinguish "never downloaded" from "downloaded but stale".
- **FR-012**: System MUST NOT prune the data directory of files absent from the
  release, which includes user-saved layouts at paths the release does not
  ship.
- **FR-013**: System MUST surface refresh failures as distinct, human-readable
  errors naming what failed.
- **FR-014**: System MUST remain safe to invoke on every startup.

### Key Entities

- **Installed Data Version**: The version of the built-in data files present in an
  installation, recorded after a successful download; absent when unknown.
- **Current Data Version**: The version the running app expects its data files to
  be at.
- **Data Currency**: The comparison result between installed and current versions,
  distinguishing up-to-date, stale, and unknown.
- **User-Created Layout**: A layout a user generated and saved, living inside the
  data directory and therefore exposed to any refresh.
- **Refresh**: The download-and-extract operation that brings built-in data files
  up to the current version.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An installation whose data is one version behind re-downloads and
  ends up recording the current version; one that is current does not.
- **SC-002**: A user-created layout saved at a path the release does not ship is
  byte-identical after a refresh and still loads.
- **SC-003**: A refresh that fails at the download or extraction stage leaves the
  previous data usable and the recorded version unchanged, and the refresh is
  retried on the next start.
- **SC-004**: A user who downgrades the app does not have their data forcibly
  rolled back to an older version.
- **SC-005**: Every one of the currency states — absent, older, equal, newer,
  unreadable — is reachable and behaves as specified.
- **SC-006**: No refresh removes a file the installation had that the archive
  does not ship.

## Assumptions

- The current data version is a value the build carries, not something read from
  the archive, so that a user on an older app version can detect that it is behind.
- The recorded version file is the same sentinel that already marks an installation
  as populated, read as a version rather than merely checked for existence.
- Built-in data files and user-created layouts share the data directory. A refresh
  re-extracts over both, and files the release does not ship are left in place.
  The app does not track which files in the data directory are user-created.
- Data files are refreshed wholesale rather than merged per file, so there is no
  partial-upgrade state to reason about.
- Download and extraction reuse the existing containment-guarded extraction, so
  refreshes carry the same guarantee as a first-time install.
- A version comparison is a simple ordered comparison; versions are expected to
  be dot-separated numbers that increase monotonically with releases.