# Feature Specification: Layout Auto-Naming Accuracy

**Feature Branch**: `feature/time-machine-layout-auto-naming`
**Created**: 2026-10-06
**Status**: Draft
**Input**: User description: "Layout Auto-Naming Accuracy. Derive a generated layout's name from its actual board shape and anchor keys rather than a fixed positional offset that misnames non-standard boards."

## Background

Generated layouts get their name from a fixed positional offset: skip the first ten
keys, take the next four. On a three-row board of ten keys that lands exactly on the
first four keys of the home row, which is the conventional way a layout is named —
"Colemak" from `c o l e`, "Workman" from `w o r k`.

Measured against the layouts actually shipped with the data files, that offset is
correct for **all 815 of them**. Every one is three rows of ten keys, so index 10 is
always the start of the second row. The heuristic is not currently wrong.

It is wrong for boards of any other shape:

| Board | `skip(10).take(4)` picks | Should pick |
|---|---|---|
| 3×10 (all shipped layouts) | first 4 of row 1 | first 4 of row 1 ✓ |
| 4 rows of 10 | first 4 of row 0 (number row) | first 4 of row 1 |
| 10/9/8 stagger | first 4 of row 1 | first 4 of row 1 ✓ |
| 2 rows | first 4 of row 1 | first 4 of row 1 ✓ |

The 4-row case is the real defect: a generated layout gets named after its number
row, producing names like `1 2 3 4` instead of `k l m n`.

The offset is also written three times, in three crates, with the reasoning recorded
only as a TODO in one of them.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Generated Layouts Are Named From the Home Row (Priority: P1)

A user generates layouts and saves them. Each saved layout is named after the first
four keys of its home row, which is how keyboard layouts are conventionally named
and what makes a list of generated layouts recognizable rather than a column of
meaningless identifiers.

On a board where the home row is not the fourth row, the name must still come from
the home row — never from the number row or another row.

**Why priority**: The name is what a user sees and refers to. A layout named after
its number row is not a cosmetic problem — it identifies the wrong keys.

**Independent Test**: For boards of several shapes, generate a name and assert it
comes from the second row's first four keys.

**Acceptance Scenarios**:

1. **Given** a three-row board of ten keys, **When** a layout is named, **Then** the
   name is the first four keys of the second row.
2. **Given** a four-row board, **When** a layout is named, **Then** the name comes
   from the second row rather than the first.
3. **Given** a board whose rows are not all the same width, **When** a layout is
   named, **Then** the name still comes from the second row.
4. **Given** a board whose second row has fewer than four keys, **When** a layout is
   named, **Then** a name is still produced, using whatever keys are available.
5. **Given** a board with a single row, **When** a layout is named, **Then** a name
   is still produced from that row.
6. **Given** a layout that already has a name, **When** it is saved, **Then** the
   existing name is kept rather than regenerated.

---

### User Story 2 - One Rule, Used Everywhere (Priority: P2)

A generated layout can be named from the command line, from the desktop app's save
flow, or when converted internally. All three must agree, and the rule that produces
a name must exist in one place.

If the rule lives in three copies, it will drift, and a fix applied to one will leave
the others wrong. This has already happened: the reasoning behind the offset is
recorded as a TODO in only one of the three copies.

**Why priority**: A generated name that differs between the CLI and the desktop app
is a subtle bug — the same layout saved from two places gets two names.

**Independent Test**: Compare names produced through each of the three paths for the
same layout and assert they are identical.

**Acceptance Scenarios**:

1. **Given** the same layout, **When** it is named through the command line, the
   desktop app, or internal conversion, **Then** all three produce the same name.
2. **Given** the naming rule, **When** a maintainer looks for it, **Then** it exists
   once, not as a repeated offset in three crates.
3. **Given** the shipped three-row layouts, **When** naming is compared before and
   after the change, **Then** every name is unchanged.

---

### Edge Cases

- **What happens when the second row has fewer than four keys?** Naming continues
  into the following row rather than producing a short or empty name, unless the
  board has no further keys.
- **What happens when the board has exactly one row?** The available keys are used.
- **What happens when the board is empty?** No name is produced; nothing panics and
  no index is read out of range.
- **What happens when a row's declared width disagrees with the number of keys
  present?** The keys actually present are authoritative.
- **What happens when the layout already has a name?** It is kept. Regenerating a
  name a user may have customised would discard their intent.
- **What happens when a key maps to a placeholder character?** It is used as-is; this
  feature is about which keys are chosen, not what they become.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST derive a generated name from the second row of the
  layout's physical shape, not from a fixed index.
- **FR-002**: System MUST take the first four keys of that row.
- **FR-003**: System MUST produce the same name for the same layout regardless of
  which interface performs the naming.
- **FR-004**: System MUST exist as a single shared rule rather than repeated in
  separate crates.
- **FR-005**: System MUST preserve names currently produced for boards of three rows
  of ten keys.
- **FR-006**: System MUST preserve an existing name rather than regenerating it.
- **FR-007**: System MUST produce a name for boards with fewer than four keys in the
  second row, using available keys.
- **FR-008**: System MUST produce a name for boards with fewer than two rows, using
  available keys.
- **FR-009**: System MUST NOT panic or read out of range for any board shape,
  including an empty one.
- **FR-010**: System MUST rely on the keys actually present in a row rather than a
  declared width that disagrees.
- **FR-011**: System MUST NOT change the naming of layouts that carry a name.

### Key Entities

- **Generated Name**: The name assigned to a layout that has none, derived from its
  own keys.
- **Layout Shape**: The ordered row widths of a layout's physical board, which
  determine where its home row starts.
- **Home Row**: The second row of the board, the conventional source of a layout's
  name.
- **Naming Rule**: The single shared definition of how a generated name is derived.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For boards of three, four, and two rows, and for rows of differing
  widths, the generated name equals the first four keys of the second row.
- **SC-002**: Every one of the layouts shipped with the data files produces the same
  name before and after the change.
- **SC-003**: Boards with fewer than four keys in the second row, fewer than two
  rows, and no keys at all all yield a name without panicking.
- **SC-004**: Naming through the command line, the desktop app, and internal
  conversion yields identical names for the same layout.
- **SC-005**: The naming rule appears once in the codebase.
- **SC-006**: A layout that already has a name keeps it.

## Assumptions

- The second row is the home row for the purposes of naming, which is the convention
  the existing offset encodes for three-row boards.
- The layout's physical shape — its row widths — is available where naming happens,
  so no new data has to be plumbed into the naming call.
- The row widths are a hint: where a declared width disagrees with the keys present,
  the keys present are authoritative.
- Naming is a presentation concern with no effect on analysis, so changing it cannot
  alter any score.
- An empty board has nothing to name, and an empty name is the correct result rather
  than a synthesised one.
- Names already stored on disk are user-visible and are not rewritten by this change.