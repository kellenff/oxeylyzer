---
title: Layout File Lifecycle
type: component
sources: [S008, S009]
updated: 2026-10-06
---

`save_layout_edit` in `oxeylyzer-tauri/src-tauri/src/lib.rs` takes the whole
layout plus the **original** name, and both paths come from `layout_path` in the
current language's directory (S008). Names are unique within a language, not
globally — layouts are stored per language, so a name may legitimately repeat
across languages (S008).

It previously derived its write path from the original name while updating the
in-memory map under the new one, so the file kept its old name and the rename
silently reverted on restart (S008). Renaming is now part of save, not a
separate operation (S008).

## Order is the safety property

Rename writes the new path, then removes the old one (S008).

**Rejected: delete-then-write.** A failed write would lose the layout
outright (S008). Write-then-delete is weaker than atomic — a failure between the
two leaves both files, which the next save resolves — but the failure mode
degrades to a duplicate rather than data loss (S008).

## Three refusals, all verified against a real directory

| Case | Behavior |
|---|---|
| target name already exists in the same language | refused, naming the conflicting layout; both files intact (S008) |
| original file missing from disk | reported, no stray second copy written (S008) |
| same normalized path (case-only change, `Ergo` → `ergo`) | **not** a rename; same path, no move (S008) |

The collision rule was reused from the REPL, which already refuses rather than
overwrites — silently destroying another layout is a worse outcome than the bug
being fixed (S008).

The case-only case is worth noting because it looks like a rename and is not:
`Ergo` and `ergo` normalize to the same path, so `original_path != new_path` is
false and nothing moves (S008). It was checked rather than assumed (S008).

The in-memory map is updated under the lowercased name, removing the original
key, so a layout appears once under one name (S008).

## The file watcher needs no change

A rename is a create plus a delete on disk, and the watcher already refreshes the
layout list on debounced file events (S008). It re-reads the directory, so it
converges on the same result the save path produces — it cannot resurrect the
old name (S008). Recorded because it looks like it might need a change.

Same-name saves are deliberately left behaviorally identical (FR-014) (S008).

## Generated layouts bypass this path

Not part of this lifecycle, and easy to assume otherwise: `save_generated` in
`lib.rs` does its own `layouts_dir()` join and `std::fs::write`. It does **not** call
`save_layout_edit`, so it does not get that function's write-then-delete ordering, its
collision refusal, or its case-only no-op check. It picks the name first
(`name.unwrap_or_else(…)` → `auto_name` per S009), then lowercases it and replaces
spaces with underscores to get the filename. Read from `lib.rs:1076-1082` while
ingesting S009 — not stated in the spec, so verify before relying on it. Note the
divergent keying too: the file is written under the underscore form, but the in-memory
map is keyed under the plain lowercase name.

Related: [Auto-Naming Generated Layouts](./auto-layout-naming.md) ·
[Why One Shared Naming Rule](./why-one-shared-naming-rule.md) ·
[Layout Row Separator Convention](./layout-row-separator-convention.md) ·
[Config Root And Data Root](./config-root-and-data-root.md)