# Feature Specification: Safe Data Archive Extraction

**Feature Branch**: `feature/time-machine-archive-safety`
**Created**: 2026-10-05
**Status**: Draft
**Input**: User description: "Safe Data Archive Extraction. Refuse archive entries that would write outside the data directory, and stream the download instead of buffering the whole archive in memory. Relevant files: oxeylyzer-resources/src/download.rs. Focus on this feature only; do not modify other features."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Hostile Archive Cannot Escape the Data Directory (Priority: P1)

The app downloads its data files as a zip archive from a remote repository on
first run. That archive is untrusted input: it arrives over the network and
nobody verified its contents. If a crafted archive could write a file outside
the data directory — overwriting a shell profile, planting a config the app
loads, or clobbering the user's own documents — then running the app once
would be enough to compromise the machine.

The user must be able to install and update the app without that risk, even if
the archive is hostile or malformed.

**Why priority**: This is the only path in the app where untrusted remote data
reaches the filesystem, and a successful escape is arbitrary file write with the
user's own privileges. It is the whole scope of this feature.

**Independent Test**: Extract a set of hostile archives — entries named with
`..` segments, absolute paths, Windows drive prefixes, and paths that traverse
out of the data root — into a temp directory, and assert that no file appears
outside it.

**Acceptance Scenarios**:

1. **Given** an archive entry whose path escapes the destination directory,
   **When** extraction runs, **Then** the entry is refused and extraction fails
   with an error naming the offending entry.
2. **Given** an archive entry that is an absolute path, **When** extraction runs,
   **Then** the entry is refused rather than written to that absolute location.
3. **Given** a well-formed archive containing a single top-level folder, **When**
   extraction runs, **Then** that folder is stripped and all entries land inside
   the destination directory.
4. **Given** a well-formed archive whose entries sit at the root with no top-level
   folder, **When** extraction runs, **Then** entries land inside the destination
   directory.

---

### Edge Cases

- **What happens when the archive contains an entry with `..` segments that
  still land inside the destination?** The entry is treated as normal; only
  entries that actually escape are refused.
- **What happens when the archive contains a mix of valid and hostile entries?**
  Refusal is not silent — the operation fails and reports which entry was
  rejected, so a partially applied hostile archive is never mistaken for a
  successful install.
- **What happens when the archive layout changes** (a different top-level folder
  name, or files at the root)? Entries are still extracted correctly regardless
  of whether the expected prefix is present.
- **What happens when the archive is truncated or corrupt?** Extraction fails
  with an error and the data directory is left in its previous state rather than
  partially populated.
- **What happens when the download is interrupted mid-transfer?** No sentinel is
  written, so the next startup retries.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST derive each archive entry's destination path from a
  form of the entry name that is already guaranteed to resolve inside the
  destination directory, rather than from the raw entry name.
  the result is guaranteed to be inside the destination directory.
- **FR-002**: System MUST refuse any entry whose resolved path would fall
  outside the destination directory, including entries containing `..` segments,
  absolute paths, and platform-specific root or drive prefixes.
- **FR-003**: System MUST treat refusal as a hard failure that aborts extraction
  and surfaces an error identifying the rejected entry, rather than skipping the
  entry and reporting success.
- **FR-004**: System MUST strip the archive's single top-level folder from entry
  paths when present, and MUST extract correctly when no such folder exists.
- **FR-005**: System MUST create parent directories for entries that need them.
- **FR-006**: System MUST report progress covering connecting, byte-level download
  progress, extraction, and completion.
- **FR-007**: System MUST record the data version sentinel only after extraction
  completes successfully.
- **FR-008**: System MUST surface download, extraction, and I/O failures as distinct, human-readable errors naming what failed.

### Key Entities

- **Archive Entry**: One item in the downloaded zip, with a declared path, a
  file-or-directory kind, and contents.
- **Resolved Destination Path**: The filesystem location an entry will be
  written to, guaranteed to be inside the destination directory or rejected.
- **Extraction Refusal**: The decision to abort because an entry's path is not
  safe, carrying enough detail to identify the offending entry.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every hostile archive tested — `..` traversal, absolute paths,
  drive-letter paths, mixed valid-and-hostile — no file is created outside the
  destination directory, and extraction fails with the offending entry named.
- **SC-002**: A well-formed archive with a top-level folder and the same archive
  without one both extract to identical content inside the destination directory.
- **SC-003**: A failed extraction never leaves a state that a later startup
  mistakes for a complete install.
- **SC-004**: Every distinct failure mode (network, corrupt archive, refused
  entry, unwritable directory, unknown platform directory) produces an error
  naming the failing step.

## Assumptions

- The data archive is published as a zip at a stable URL under the project's
  repository, and its contents are trusted in the sense of being intended data,
  but not verified — the archive itself is untrusted input for containment
  purposes.
- The archive may contain a single top-level folder that gets stripped during
  extraction, and may also contain entries at the root.
- Download and extraction failures are recoverable: the user can retry, and the
  app remains usable for functionality that does not require data files.