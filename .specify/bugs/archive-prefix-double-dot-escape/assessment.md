# Bug Assessment: archive prefix lets one `..` escape the data directory

- **Slug**: archive-prefix-double-dot-escape
- **Created**: 2026-10-10
- **Source**: pasted text (same text recorded in `TODO.md:81-145`; also in
  `wiki/pages/archive-path-containment.md`)
- **Verdict**: valid
- **Severity**: high

No URL was present in the report, so the URL trust policy did not apply and
nothing was fetched.

## Report (summarized)

The report claims: `oxeylyzer-resources/src/download.rs:90-103` runs the
containment check and strips the archive prefix in the wrong order. `zip`
2.4.2's `enclosed_name()` counts path components against a depth counter
(`types.rs:542-551`), so the `oxeylyzer-data-files/` prefix contributes one
level of depth. An entry `oxeylyzer-data-files/../escaped.txt` therefore
validates at depth 1, decrements to 0, and is accepted; the prefix is then
stripped, leaving `../escaped.txt`, and `dest.join()` writes one level *above*
the destination.

Quoted code from the report:

```rust
let enclosed = file.enclosed_name().ok_or_else(|| ...)?;   // accepts "../escaped.txt"
let raw_name = enclosed.to_string_lossy();
let stripped = raw_name.strip_prefix(ZIP_INNER_PREFIX).unwrap_or(&raw_name);
let out_path = dest.join(stripped);                          // <data_root>/../escaped.txt
```

Supporting claims, all of which the assessment confirmed: the author added a
temporary single-`..` test to the real crate, it failed on `assert!(!ok)`, and
the file appeared outside the destination; every shipped traversal test uses
two or three `..` segments, so all 34 tests pass against vulnerable code; the
in-code comment at lines 85-89 states the correct rule while the code below it
does the opposite; `specs/001-archive-safety/plan.md` prescribes
strip-then-check but `tasks.md` T002 prescribes the opposite and the
implementation followed T002.

## Symptom

A zip entry named `oxeylyzer-data-files/../<rel>` is accepted by extraction and
written to `<data_root>/../<rel>` — outside the data directory — while
`extract_bytes` returns `Ok(())` and the `.data-version` sentinel is written as
though the install were clean. Expected: any entry that would land outside
`dest` is refused with `ResourceError::Extraction` and extraction aborts
(FR-001, FR-002, FR-003).

## Reproduction

1. Build an in-memory zip containing the GitHub-style root entry
   `oxeylyzer-data-files/` and one hostile entry
   `oxeylyzer-data-files/../escaped.txt` (contents arbitrary).
2. Call `extract_bytes(&body, dest)` with `dest` standing in for the data root
   (`<sandbox>/oxeylyzer`), then inspect the filesystem.
3. Observed: returns `Ok(())`, and `escaped.txt` exists at `<sandbox>/escaped.txt`
   — one level above `dest`. The same happens for
   `oxeylyzer-data-files/../someotherapp/evil/payload.txt` and for padding with
   harmless directories first, e.g. `oxeylyzer-data-files/a/b/../../../escaped.txt`.
4. Control: `oxeylyzer-data-files/../../escaped.txt` (two `..`) returns `Err` and
   writes nothing — see "Measured evidence" for why the boundary sits exactly
   there.

Triggering end to end requires `dirs.needs_data()` to be true, i.e. a fresh
install or a `DATA_VERSION` bump (`oxeylyzer-resources/src/dirs.rs:122-127`).
Unknown: nothing observed about a real crafted archive; the reproduction above
uses the real extraction code path with an in-memory archive, which is how the
shipped tests exercise it too.

## Suspected Code Paths

- `oxeylyzer-resources/src/download.rs:90-95` — `enclosed_name()` is called on
  the **unstripped** name; the prefix's `Normal` component banks the depth that
  the following `ParentDir` then spends, so the check passes.
- `oxeylyzer-resources/src/download.rs:97-103` — the prefix is stripped
  **after** the check, so `dest.join("..")` walks out. `File::create` at line 111
  succeeds because `<dest>/..` always exists; even the `create_dir_all` calls
  succeed.
- `oxeylyzer-resources/src/download.rs:116` — `.data-version` is written after
  the loop, so a successful escape is recorded as a complete, current install
  and is never retried or flagged.
- `oxeylyzer-resources/src/download.rs:85-89` — the comment describes the correct
  ordering and the correct hazard, but the code below it does the opposite, and
  the comment's own example (`../../etc/passwd`, two `..`) is rejected under
  either ordering, so it does not demonstrate anything.
- `oxeylyzer-resources/src/download.rs:186-195` — `traversal_escaping_root_is_rejected`
  uses three `..`, `:268-280` (`mixed_valid_and_hostile_fails_without_partial_files`)
  uses two. Both are rejected by the shipped order regardless, so neither pins
  the ordering.
- `oxeylyzer-resources/src/download.rs:164-183` — `files_under`/`walk` are
  **dead code**: compiled with "function is never used" warnings, because
  `traversal_escaping_root_is_rejected` asserts on one hard-coded path instead.
  `specs/001-archive-safety/tasks.md` T007 says the test must "walk parent dir,
  do not just check return value"; the walk helper was written but never wired
  up.
- `oxeylyzer-resources/src/seed.rs:8-18` → `oxeylyzer-tauri/src-tauri/src/lib.rs:1669-1672`
  and `oxeylyzer-repl/src/repl.rs:220` — `ensure_data` is the only caller, on
  first launch. Confirms the "first run" trigger.
- `specs/001-archive-safety/tasks.md:11-13` (T002) — prescribes check-then-strip,
  contradicting `plan.md:20-25,64,152`. The implementation followed T002.

## Root Cause Hypothesis

The implementation validates the complete entry name and only then removes the
archive's root-folder prefix, but the prefix itself is a `Normal` component for
the library's depth counter (`zip` 2.4.2 `types.rs:542-551`). That single banked
level is exactly what one `..` needs, so `oxeylyzer-data-files/../X` passes the
containment check and the subsequent strip produces a name that starts with
`..`, which `dest.join()` faithfully applies. Confidence: **high** — the
mechanism is visible in the source of both the caller and the library, and it
was reproduced against the real crate version.

**Correction to the report.** The escape is bounded, not arbitrary: exactly one
level above `dest` is reachable per entry (`<dest>/../<rel>`), and only one
direct-ancestor component is reachable per entry because a second `..` is
rejected. From there, however, *unbounded depth downward* is reachable, because
the stripped remainder is a legal relative name:
`oxeylyzer-data-files/../someotherapp/evil/payload.txt` extracts `Ok` to
`<dest>/../someotherapp/evil/payload.txt`. So "writes anywhere the user can
write" is wrong as stated (the grandparent is unreachable), but on macOS and
Windows the parent of the data root is `~/Library/Application Support` and
`%APPDATA%` (Roaming) respectively, and *everything below those* — other apps'
support files, and on Windows the roaming Startup folder — is writable. The
severity conclusion survives the correction; the mechanism description does not.

**Also corrected.** The report's fix step 1 says hand-rolling the component walk
is unavoidable. The library's validator is not reachable for this: `zip` 2.4.2's
`src/path.rs` is a private module (`mod path;` in `lib.rs`), and the only public
entry points are `enclosed_name()`/`sanitized_name()`, neither of which accepts a
subpath argument. So the report is right that the library cannot express the
check — but not "either a second `ZipFile` over the same entry or an inline
component walk": keeping the existing `enclosed_name()` call *and* adding a
depth check on the stripped remainder is smaller and keeps the library's
NUL/absolute/drive rejection intact.

## Measured Evidence

Verified against the locked crate version (`zip` 2.4.2, checksum
`fabe6324e908f85a1c52063ce7aa26b68dcb7eb6dbc83a2d148403c9bc3eba50`) in a scratch
crate outside the repository, using a verbatim port of the `extract_bytes` loop
and the real `enclosed_name()`. The depth rule being tested, extracted from
`zip` 2.4.2 `types.rs:542-551` for comparison:

```rust
let mut depth = 0usize;
for c in Path::new(rel).components() {
    match c {
        Component::Prefix(_) | Component::RootDir => return false,
        Component::ParentDir => match depth.checked_sub(1) { Some(d) => depth = d, None => return false },
        Component::Normal(_) => depth += 1,
        Component::CurDir => {}
    }
}
```

Filesystem result of the shipped ordering (`dest` = sandbox root + `/oxeylyzer`,
"escaped" lists files landing outside `dest`, relative to the sandbox root):

| Entry | Result | Escaped |
|---|---|---|
| `oxeylyzer-data-files/../escaped.txt` | `Ok` | `escaped.txt` |
| `oxeylyzer-data-files/a/b/../../../escaped.txt` | `Ok` | `escaped.txt` |
| `oxeylyzer-data-files/../someotherapp/evil/payload.txt` | `Ok` | `someotherapp/evil/payload.txt` |
| `oxeylyzer-data-files/../../escaped.txt` | `Err` | — |
| `oxeylyzer-data-files/../../../escaped.txt` | `Err` | — |
| `oxeylyzer-data-files/a/b/c/../../../../../../x.txt` | `Err` | — |
| `../escaped.txt` (no prefix) | `Err` | — |
| `oxeylyzer-data-files/static/text/en.txt` | `Ok` | — (correct) |
| `oxeylyzer-data-files/a/../b.txt` | `Ok` | — (correct) |

Ordering comparison on identical names (whole-name check as shipped vs
check-after-strip):

| Entry | whole-name accepts | stripped accepts | diverges |
|---|---|---|---|
| `oxeylyzer-data-files/../escaped.txt` | yes | no | **yes** |
| `oxeylyzer-data-files/a/b/../../../escaped.txt` | yes | no | **yes** |
| `oxeylyzer-data-files/../../escaped.txt` | no | no | no |
| `oxeylyzer-data-files/a/../b.txt` | yes | yes | no |

Only the single-`..`-after-prefix shape separates the two orders, matching the
report. Escape is bounded to one level above `dest` per entry: `a/b` padded by
two `..` (`oxeylyzer-data-files/a/b/../../../../x.txt`) is rejected, because the
second `..` arrives at depth 0. Padding with harmless directories *inside* the
dest-relative remainder (`a/b/../../../escaped.txt`) does escape — the padding
supplies the depth that one extra `..` consumes. The shipped test suite (34
tests) passes against this code, confirmed by running it.

## Proposed Remediation

**Preferred**: validate the exact string that is joined. Keep the
`enclosed_name()` call on the raw name (it is what rejects NUL bytes, absolute
paths, and drive prefixes), keep stripping `ZIP_INNER_PREFIX`, and add a short
depth check over the *stripped remainder* before `dest.join()`, using the same
component rule quoted above. Roughly six lines, no new dependency, and the
existing `a/../b.txt` → `dest/b.txt` case stays legal because the remainder's
depth never goes negative. The in-code comment at lines 85-89 is currently the
correct description printed above code that contradicts it; after the fix it
should describe the two-step shape (library rejects the unroutable names, the
remainder is then checked against its own depth) and its example must use a
**single** `..`, or it will keep failing to demonstrate the hazard.

This keeps the semantics `plan.md` asked for — nothing is joined before it is
validated — while not re-implementing NUL/drive handling, which is the cost
`plan.md:148-152` explicitly rejected.

**Alternatives**:
- *Strip first, then validate the remainder with a standalone walk* — matches
  the literal wording of `plan.md:20-25,64` (and contradicts T002), but the
  standalone walk now has to reproduce the library's NUL/absolute/drive
  rejection too; dropping those is a silent regression for the absolute-path
  case.
- *Join, normalize lexically, and require the result to be prefixed by `dest`* —
  conceptually simplest, but `dest` may not exist yet on first run so it cannot
  be canonicalized reliably, and per-entry filesystem resolution is the kind of
  thing that behaves differently on Windows.
- *Reject any `..` anywhere in the name* — smallest diff, but it breaks the
  in-bounds case the spec explicitly allows (`spec.md` edge cases, existing test
  `parent_segment_within_bounds_is_allowed`), so it is a behavior regression.

**Files likely to change**:
- `oxeylyzer-resources/src/download.rs` — extraction loop plus comment
- `oxeylyzer-resources/src/download.rs` test module — new tests, and wire up the
  currently-dead `files_under`/`walk` helpers
- `specs/001-archive-safety/tasks.md` T002 — wrong ordering prescription;
  correcting spec artifacts is a separate, deliberate change
- `TODO.md` — the report's fix plan lives there; update once landed
- `wiki/pages/archive-path-containment.md` — will re-record the breach if
  re-ingested before the fix lands (noted in the report)

**Tests to add or update**:
- New: `oxeylyzer-data-files/../escaped.txt` → `Err`, and assert on the
  filesystem that **`dest.parent()`** contains no `escaped.txt`. Note the trap:
  the existing `traversal_escaping_root_is_rejected` asserts on
  `root.parent()` (the temp dir), which is where the **three**-`..` entry would
  land; the single-`..` entry lands at `dest.parent()` = `root`. A copy of the
  existing assertion shape would be green before the fix and prove nothing.
- New: padded single-`..` (`oxeylyzer-data-files/a/b/../../../escaped.txt`) and
  sideways (`oxeylyzer-data-files/../someotherapp/evil/payload.txt`) → `Err`,
  no files outside `dest`.
- Update: `traversal_escaping_root_is_rejected` to use `files_under(&root)` and
  assert every returned path starts with `data/`, which both wires the dead
  helper and turns the test into the general "nothing appeared outside the
  destination" check T007 asked for.
- Keep: `parent_segment_within_bounds_is_allowed` as the positive control that
  the fix has not over-rejected.
- Optional: a NUL-byte entry test to pin the library behavior a hand-rolled
  validator would lose.

## Risks & Considerations

- **Over-correction**: rejecting every `..` breaks the in-bounds case the spec
  allows and the existing passing test; the fix must preserve
  `a/../b.txt` → `dest/b.txt`.
- **Regression if the library check is replaced rather than supplemented**: a
  hand-rolled validator must also keep NUL-byte and drive-prefix rejection;
  there is no test for NUL bytes today.
- **Vacuous new test**: asserting at the wrong directory level (temp dir instead
  of `dest.parent()`) passes both before and after the fix; assert on the walk
  from `root` instead.
- **Silent-success property**: because the sentinel is written after a loop that
  never inspects the resolved path, an escaping archive is recorded as a clean,
  current install and is never retried. The fix should keep abort-on-refusal
  (already the behavior for `None`), so FR-003 stays true.
- **Reachability is bounded but real**: the write is capped at one directory
  above the data root, but that directory is `~/Library/Application Support` on
  macOS and `%APPDATA%` (Roaming, which contains the Startup folder) on Windows,
  with unbounded depth below it. Severity **high**, not critical: there is no
  unauthenticated remote path to a crafted archive — the URL is a fixed HTTPS
  GitHub endpoint fetched with ureq 2.12.1 over rustls/webpki-roots, and
  exploitation needs write access to the data-files branch or a CA-level MITM.
  No checksum or signature verification exists anywhere in the download path,
  so the archive is trusted by transport and repo alone.
- **Process**: T002 in `tasks.md` is wrong and will keep re-seeding this bug in
  wiki ingest until corrected; the report argues for a separate change, which
  the correct/incorrect framing above supports.

## Open Questions

- [NEEDS CLARIFICATION: correct `specs/001-archive-safety/tasks.md` T002 in the
  same change as the code fix, or as a separate documentation change as the
  report proposes?]
- [NEEDS CLARIFICATION: add a NUL-byte entry test now, to pin the library
  behavior that the hand-rolled alternative would otherwise silently drop?]
