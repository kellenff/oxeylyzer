# Preserve existing layouts when saving generated results

Issue: https://github.com/kellenff/oxeylyzer/issues/2

## Intent and scope

Saving a generated layout must never replace an existing destination. A name
conflict must return an error, preserve the existing bytes, and let the user
rename the result and retry. New destinations must still produce loadable DOF
files and enter the loaded-layout collection only after a successful save.

## Approaches

- **Atomic creation (selected):** serialize first, then open with
  `OpenOptions::create_new(true)` and write all bytes. The filesystem refuses
  existing destinations, including a destination created by another writer.
- **Existence check followed by writing (rejected):** another writer can create
  the destination between the check and the write, permitting data loss.

The user approved atomic creation on November 6, 2026.

## Design and constraints

Keep the change in the Tauri backend's generated-layout save path. Preserve the
existing name-to-filename normalization and command signature. Report an
`AlreadyExists` error with the conflicting name and guidance to choose another
name; propagate other I/O failures as save errors. Reuse the Generate screen's
existing error display and retry controls. Do not add dependencies.

Use the existing serialization before touching the destination, then create it
exclusively and use `write_all`. Add a small private persistence helper only if
needed to exercise the real write path without desktop initialization. This
test seam is an implementation choice, not a new public API.

## Verification

Use isolated temporary files to prove that a conflicting save reports an error
and leaves the original bytes unchanged, and that a new save writes all expected
bytes. Check that repeat saves refuse replacement. Run the relevant Rust tests
and the canonical Rust formatter.

## Limits

Automatic renaming, overwrite confirmation, filename validation changes, and
changes to other save commands are outside this issue. Atomic creation prevents
replacement; it does not promise crash-safe publication of the newly written
file. Native desktop testing is separate from backend filesystem evidence.
