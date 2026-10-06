# Feature Specification: Layout Edit Consistency

**Feature Branch**: `feature/time-machine-edit-view-rename-delete`
**Created**: 2026-10-06
**Status**: Draft
**Input**: User description: "Layout Rename Consistency. Make editing a layout's name leave no orphaned file on disk, and remove the assumption that boards are always a five-plus-five row split."

## Background

Two defects deferred from an earlier GUI improvement pass. Both were re-verified
against the current code before this spec was written, and both confirmed real —
B6 considerably worse than originally recorded.

### B6: editing corrupts row formatting on every shipped layout

`applyCharAt` splits each row at a hardcoded five keys and rejoins the two halves
with a double space. The boards shipped with the data files are three rows of ten
keys — 809 of 815 are exactly 10/10/10, five are 10/11/10, one is 10/10/10/2 — so
the split falls in the middle of every row that gets edited.

Editing one character of `a b c d e f g h i j` produces
`a b c X e  f g h i j`. The double space is injected mid-row, which is not a
cosmetic preference: the row is then re-serialized, and every subsequent edit
re-splits it.

### B5: renaming leaves the disk out of step with the app

`save_layout_edit` derives the write path from the original name, then updates the
in-memory map under the new name. The file on disk keeps its old name while the app
believes the layout has been renamed. After a reload from disk the old name
returns.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Editing a Key Leaves the Layout Intact (Priority: P1)

A user opens a layout, changes one key, and saves. Every other key must be exactly
where it was, the row must keep its original shape, and re-opening the layout must
show the same layout with only that one key changed.

Today, changing a key also injects a gap into the middle of the row — on every
shipped layout, because they are all ten keys wide and the editor assumes five.

**Why priority**: Editing is the feature users reach for when a layout is close to
right, and it silently damages the file on every save. It is the whole of this
feature.

**Independent Test**: For a range of row shapes, edit one key at each position and
assert every other key is untouched and the row shape is preserved.

**Acceptance Scenarios**:

1. **Given** a three-row ten-key layout, **When** one key is edited, **Then** only
   that key changes and no spacing is introduced anywhere in the row.
2. **Given** a layout with rows of differing widths, **When** a key is edited, **Then**
   each row keeps its own width.
3. **Given** a layout with more than three rows, **When** a key is edited, **Then**
   the fourth row is handled like the others.
4. **Given** a key is edited and saved, **When** the layout is reloaded from disk,
   **Then** the change is the only difference from the original.
5. **Given** two keys are exchanged by dragging, **When** the exchange is applied,
   **Then** exactly those two keys change and nothing else does.
6. **Given** a layout whose rows contain irregular spacing, **When** a key is
   edited, **Then** the row's existing spacing convention is preserved rather than
   normalized into something else.

---

### User Story 2 - Renaming a Layout Renames the File (Priority: P1)

A user renames a layout in the editor and saves. They expect the layout to have a
new name — including after restarting the app. Instead the file keeps its old name,
so the rename appears to have worked, then silently reverts.

A rename must move the file, not just relabel it in memory. After saving, the old
name must be gone and the new name must survive a restart.

**Why priority**: A rename that reverts is worse than no rename — the user may act
on a false belief about which file holds their layout. This is the second half of
the feature.

**Independent Test**: Rename a saved layout, then read the layouts directory from
disk and assert the old file is gone and the new one holds the layout.

**Acceptance Scenarios**:

1. **Given** a saved layout is renamed, **When** the rename is saved, **Then** the
   file takes the new name and the old file no longer exists.
2. **Given** a rename is saved, **When** the app restarts, **Then** the layout is
   listed under its new name.
3. **Given** a layout is renamed to a name that already exists, **When** the rename
   is saved, **Then** the user is told rather than one layout silently overwriting
   the other.
4. **Given** a layout is edited but not renamed, **When** it is saved, **Then** it
   keeps its filename.
5. **Given** a rename is saved, **When** the layout list refreshes, **Then** the
   layout appears once, under the new name, not both names.

---

### Edge Cases

- **What happens when a rename would collide with a layout in another language?**
  Layouts are stored per language, so names may legitimately repeat across
  languages; only collisions within the same language are a problem.
- **What happens when the new name contains characters a filename cannot hold?**
  Names are already normalized when written to disk, and the same normalization
  applies on rename, so the file is always writable.
- **What happens when the original file no longer exists — deleted by hand, or
  renamed in the file manager?** The save reports it rather than writing a second
  copy.
- **What happens when a row is shorter than the assumed split?** No gap is inserted,
  since there is nothing to separate.
- **What happens when the layout is edited in the browser and not saved?** Nothing on
  disk changes.
- **What happens when a rename fails after the old file was already removed?** The
  layout must not be lost; the save reports failure and the file is restored.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST change only the requested key when a key is edited.
- **FR-002**: System MUST NOT introduce any spacing, padding, or separator into a
  row when a key is edited.
- **FR-003**: System MUST NOT assume a fixed number of keys per row or a fixed split
  position; row widths may differ per row and rows may differ in count.
- **FR-004**: System MUST preserve the separator convention already used within a
  row when editing it.
- **FR-005**: System MUST apply a drag exchange by changing exactly the two keys
  involved.
- **FR-006**: System MUST persist a key edit such that reloading from disk shows the
  edit and nothing else.
- **FR-007**: System MUST rename the backing file when a saved layout's name changes.
- **FR-008**: System MUST NOT leave the previous file behind after a rename.
- **FR-009**: System MUST NOT change the filename when a layout is edited without
  being renamed.
- **FR-010**: System MUST report a rename that would overwrite an existing layout,
  and MUST NOT perform it.
- **FR-011**: System MUST NOT lose a layout if a rename fails partway.
- **FR-012**: System MUST keep the in-memory layout list consistent with disk, so a
  layout appears once under one name.
- **FR-013**: System MUST report a save whose original file is missing rather than
  creating a duplicate.
- **FR-014**: System MUST preserve existing behavior for layouts whose name is
  unchanged.

## Key Entities

- **Layout Edit**: An in-progress change to one or two keys, held before saving.
- **Layout File**: The per-language file on disk holding one layout's definition.
- **Layout Name**: The name a layout carries, which is also the basis of its file
  name after normalization.
- **Rename**: A save in which the layout's name differs from the original, requiring
  the file to move.
- **Row**: One line of a layout's key definitions, whose width and internal spacing
  are properties of that layout rather than fixed by the editor.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Editing one key in a layout of any row shape changes that key and no
  other, and introduces no new spacing anywhere.
- **SC-002**: Every layout shipped with the data files round-trips through an edit
  with exactly one key changed.
- **SC-003**: A rename results in the old file being absent and the new file present
  with the layout's contents, verified by reading the directory.
- **SC-004**: A renamed layout is listed under its new name after a full restart.
- **SC-005**: A rename targeting an existing name in the same language is refused
  with a message and both layouts remain intact.
- **SC-006**: A failed rename leaves the layout present under its original name.
- **SC-007**: Editing without renaming leaves the filename unchanged.

## Assumptions

- Layout files are stored per language, one directory per language, and names are
  unique within a language rather than globally.
- Filenames are derived from layout names by normalizing case and spaces, the same
  way saves already work.
- A layout file's name is the only thing that has to move on rename; its contents
  are independent of the name.
- Editing is a client-side operation that saves the whole layout, so fixing the
  editor's row handling is sufficient to make edits non-destructive.
- Rename is a save where the name changed, not a separate operation, so both paths
  run through the same save.
- Whether the file watcher should react to a rename as a create plus a delete, and
  how that interacts with refreshing the layout list, is an implementation detail
  rather than a user-visible decision.