# Implementation Plan: Layout Edit Consistency

**Branch**: `feature/time-machine-edit-view-rename-delete` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/006-edit-view-rename-delete/spec.md`

## Summary

Two defects in layout editing. Editing a key rewrites its row through a hardcoded
five-key split, which mangles every shipped layout. Saving a renamed layout writes to
the old filename, so the rename reverts on restart.

**Technical approach**: make row editing replace a single character in place and
leave the row's spacing untouched; make the save detect a name change and move the
file, refusing collisions the way the REPL already does.

## Technical Context

**Language/Version**: TypeScript/SolidJS (editor), Rust edition 2024 (`src-tauri`)
**Primary Dependencies**: none added
**Storage**: filesystem — per-language layout directories
**Testing**: existing frontend has no test runner; row logic is pure and extracted for direct verification. `cargo test --workspace` for regressions.
**Target Platform**: desktop
**Performance Goals**: not applicable
**Constraints**: no new dependencies; FR-014 keeps same-name saves behaviorally identical
**Scale/Scope**: one editor component, one Rust command, one extracted helper

## Measurements that shape the approach

Taken from the published data files rather than assumed:

| Row shape | Count |
|---|---|
| 3 rows × 10 keys | 809 |
| 3 rows, 10/11/10 | 5 |
| 4 rows × 10 | 1 |

Boards are `ortho` (811) and `ansi` (4). Every shipped layout is at least ten keys
wide, so the five-key split lands mid-row on effectively all of them.

The observed defect, on a real row:

```
a b c d e f g h i j     edit key 3 to X
a b c X e  f g h i j    double space injected after the fifth key
```

The row is then serialized, so the gap persists and the next edit re-splits again.

## Frontend changes

### Row editing

`applyCharAt` currently: split the row on whitespace, replace one entry, then
rejoin as `slice(0,5) + "  " + slice(5)`. That join is the entire bug — the split
position is meaningless and the double space is the corruption.

Replace with an in-place character replacement that reconstructs the row from its own
separator:

```ts
/** Replaces the key at `pos` in `rows`, preserving the row's existing spacing. */
function setKeyAt(rows: string[], rowIndex: number, posInRow: number, char: string): void
```

The row's separator is whatever the row currently uses between its keys, so
hand-alternated layouts using double spaces keep them. FR-002 and FR-004.

`updateKeyAt` and `handleSwap` both route through this helper, so FR-001 and FR-005
follow from it.

Row and column indices come from walking the rows and accumulating offsets — already
what the function does. Only the write-back changes.

## Backend changes

### Rename on save

`save_layout_edit` takes the whole layout plus the original name. It currently
derives the path from the original name, writes, then updates the map under the new
name. The fix is three steps:

1. Compute the new path from the new name, the old path from the original name.
2. If they differ, this is a rename: refuse if the new path already exists and the
   layout is not the one being saved (FR-010).
3. Write to the new path, remove the old, then update the map removing the original
   key and inserting the new one (FR-007, FR-008, FR-012).

Order matters for FR-011. Write-then-delete means a failure to write leaves the
original intact; a failure between write and delete leaves both, which the next save
resolves rather than losing the layout. Delete-then-write would risk losing it.

Collision check follows the REPL's existing precedent at `repl.rs:412`, which refuses
a colliding name rather than clobbering. FR-010 is satisfied by refusing with a
message naming the conflicting layout.

### Missing original

If the old path does not exist, report it rather than writing a second copy
(FR-013). Today the write would succeed and the user would end up with two files
for one layout.

## Watcher interaction

A rename is a create plus a delete on disk. The existing watcher refreshes the
layout list on file events with a debounce, which will fire for both. That is
acceptable: it re-reads the directory, so the old name disappears and the new one
appears. The save path's own map update happens first and the watcher overwrites it
with the same result.

No watcher change needed. Recorded in the plan because it looks like it might need
one, and because a debounce that fires mid-rename could otherwise resurrect the old
name — it re-reads from disk, so it cannot.

## Project Structure

```text
oxeylyzer-tauri/
├── src/views/EditView.tsx     # row editing
└── src-tauri/src/lib.rs       # save_layout_edit rename handling
```

## Verification

```bash
cargo test --workspace          # 205 must still pass
cargo clippy -p oxeylyzer-tauri
bunx tsc --noEmit
bunx eslint src
bun run build
```

The row helper is pure and takes a position and a character, so it is verified
directly against the shipped row shapes: 10/10/10, 10/11/10, and 4×10, at every
position, asserting the row is byte-identical apart from the replaced key.

Rename is verified against a real temp layouts directory: rename, then assert the
old path is gone and the new path holds the contents; rename to a colliding name,
assert refusal and both files intact; simulate a write failure, assert the original
survives.

## Complexity Tracking

No constitution violations.

| Choice | Simpler alternative rejected because |
|---|---|
| Replace in place, no rejoin | Rejoining is what injects the gap; any rejoin has to guess the separator |
| Preserve each row's own separator | Normalizing to single spaces destroys deliberate hand-alternation, which uses double spaces |
| Write-then-delete on rename | Delete-then-write risks losing the layout if the write fails |
| Reuse the REPL's refuse-on-collision rule | Overwriting silently is a worse outcome than the bug being fixed, and the codebase already has the safer precedent |