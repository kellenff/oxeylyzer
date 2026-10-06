# Feature Specification: Graceful Desktop Startup Failures

**Feature Branch**: `feature/time-machine-tauri-startup-graceful`
**Created**: 2026-10-06
**Status**: Draft
**Input**: User description: "Graceful Desktop Startup Failures. Report directory resolution, download, and config load failures as actionable startup errors instead of panicking, so the app can explain what went wrong."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Startup Failure Explains Itself (Priority: P1)

A user launches the desktop app and it fails to start. The built-in data files
have not downloaded yet, the network is down, the user's configuration file is
malformed, or their data directory cannot be written. In each case the app today
stops with a panic message aimed at a developer.

The user needs to learn what failed and what they can do about it. "failed to load
corpus" appearing in a terminal tells them nothing they can act on; being unable
to start the app at all, with no window and no explanation in the app, is worse.

**Why priority**: This is the difference between an app that fails and an app that
can be fixed. It is the entire scope of the feature.

**Independent Test**: Force each startup dependency to fail and confirm the app
reports the specific cause rather than terminating abruptly.

**Acceptance Scenarios**:

1. **Given** the data directory cannot be determined or created, **When** the app
   starts, **Then** it reports that directories could not be set up and names the
   reason, rather than terminating abruptly.
2. **Given** the initial data download fails, **When** the app starts, **Then** the
   user is told the download failed and why, and can retry.
3. **Given** the configuration file cannot be written, **When** the app starts,
   **Then** the user is told which configuration file could not be written.
4. **Given** the configuration file exists but cannot be parsed, **When** the app
   starts, **Then** the user is told which file is malformed and which setting is at
   fault.
5. **Given** the corpus file named by the configuration is missing or unreadable,
   **When** the app starts, **Then** the user is told which file is missing and that
   it comes from the configured language.

---

### User Story 2 - The Failure Is Actionable, Not Just Visible (Priority: P2)

A message that names a problem without saying what to do about it leaves the user
stuck. Telling them the configured corpus file is missing is only half the answer;
they need to know it comes from a language setting they chose, and where that
setting lives.

Every startup failure must point the user at the thing they can change. A failed
download points at the network. A missing corpus points at the language setting.
A malformed configuration points at the configuration file and the setting in it
that is wrong.

**Why priority**: Story 1 delivers the message; this makes it enough to act on.
A graceful failure the user cannot resolve is still a dead end, just a quieter one.

**Independent Test**: Force each failure and confirm the message identifies both
what failed and what the user can change about it.

**Acceptance Scenarios**:

1. **Given** the configured corpus file is missing, **When** the app reports it,
   **Then** it names the missing file and the language setting that chose it.
2. **Given** the configuration file is malformed, **When** the app reports it,
   **Then** it names the file and the setting that could not be parsed.
3. **Given** the initial download failed, **When** the app reports it, **Then** it
   makes clear the app could not fetch its data files and that retrying may help.
4. **Given** directories could not be created, **When** the app reports it, **Then**
   it names the directory it could not use.

---

### Edge Cases

- **What happens when several startup steps fail at once?** The user is told about
  the first blocking failure, and any later ones, without the errors being merged
  into something unreadable.
- **What happens when the failure is a permissions problem on the user's home
  directory?** The message names the directory that could not be used, not just the
  category of error.
- **What happens when the configuration parses but a path inside it does not
  exist?** The user is told which configured path is missing rather than that
  loading "failed".
- **What happens when the app is in development mode with a data directory
  override?** The same reporting applies, naming the overridden directory.
- **What happens when the failure happens before any window exists?** The report is
  still shown — to the user, not only to a terminal they may not have open.
- **What happens when a non-startup step fails later, such as a watcher-triggered
  reload?** Existing behavior continues unchanged.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST NOT terminate the process abruptly when a startup
  dependency fails.
- **FR-002**: System MUST identify which startup step failed.
- **FR-003**: System MUST report the specific underlying reason, not only a category.
- **FR-004**: System MUST name the file, directory, or path involved whenever the
  failure concerns a specific location.
- **FR-005**: System MUST report a failure to determine or create the data and
  configuration directories as a distinct, actionable condition.
- **FR-006**: System MUST report a failed initial data download as a distinct,
  actionable condition, and allow the user to retry it.
- **FR-007**: System MUST report a configuration that cannot be written and a
  configuration that cannot be parsed as distinct conditions, identifying the file.
- **FR-008**: System MUST report a missing or unreadable corpus as a condition
  naming the file and the configured language it came from.
- **FR-009**: System MUST open the app window before startup work begins, so that a
  failure at any point has somewhere to be reported.
- **FR-010**: System MUST report each failure inside the app window, not only to a
  terminal that may not be open.
- **FR-011**: System MUST identify what the user can change about a failure — the
  network, the language setting, or the configuration file — not only what broke.
- **FR-012**: System MUST name the specific file or directory involved in a
  failure, including which configured language a corpus path came from.
- **FR-013**: System MUST preserve the existing success path exactly, including the
  order in which startup steps depend on each other.
- **FR-014**: System MUST leave non-startup failure handling unchanged.

### Key Entities

- **Startup Step**: One of the independent operations startup performs — directory
  setup, initial data download, configuration creation, configuration load, corpus
  load. Each can fail on its own.
- **Startup Failure**: A failed startup step together with its reason and the
  location involved, as distinct from other failures.
- **Degraded Startup**: A successful launch in which some data could not be loaded,
  leaving features that need it unavailable while the rest remain usable.
- **Blocking Failure**: A startup failure after which the app cannot usefully
  continue and must explain itself instead of continuing.
- **Recoverable State**: The condition in which the user can act — retry a
  download, choose another language, or repair a configuration — without
  reinstalling.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Each of the five startup failures can be forced in turn, and in every
  case the app names the step and the specific reason instead of terminating.
- **SC-002**: Every startup failure is reported inside the app window rather than
  only to a terminal.
- **SC-003**: Every failure message identifies both what failed and what the user
  can change about it.
- **SC-004**: Every failure message names a concrete file, directory, or path where
  one applies, including the configured language for a corpus failure.
- **SC-005**: No failure during startup terminates the process abruptly.
- **SC-006**: A successful startup behaves exactly as before, and the workspace
  test suite is unchanged.

## Assumptions

- Startup failure reporting is delivered through the desktop window rather than the
  terminal, since a user double-clicking an application has no terminal open.
- The desktop app already has an in-window error display, currently used only for
  failures that happen after startup completes. Startup failures join that display.
- The desktop app already emits data-download progress as an application event; the
  frontend does not currently listen for it. Reporting startup progress is a
  natural companion to reporting startup failure, and shares the same channel.
- Configuration parsing errors can name the offending setting, since parsing is
  structured rather than free-text.
- Corpus loading is treated as blocking, as it is today. Every other part of the
  app analyzes through a corpus-derived engine, so opening without one would leave
  most of the interface non-functional. The message must therefore be actionable
  rather than the app remaining open.
- The command-line interface already reports errors on its own terms and needs no
  equivalent change; this feature concerns the desktop app.
- Startup order between the data-dependent steps is deliberate and preserved:
  directories, then data, then configuration, then corpus. Opening the window is
  independent of all of them and can precede them without disturbing this.