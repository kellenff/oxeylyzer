---
title: Layout Row Separator Convention
type: concept
sources: [S008]
updated: 2026-10-06
---

Layout rows have no fixed width. Measured across the shipped data files rather
than assumed: 809 layouts are three rows of ten keys, five are 10/11/10, and one
is four rows of ten. Boards are `ortho` (811) and `ansi` (4) (S008).

Some layouts are hand-alternated, separating the two hands with **double
spaces** (S008). So the whitespace inside a row is a property of the layout, not
of the editor (S008).

## The defect this replaced

`applyCharAt` split the row on whitespace, replaced one entry, and rejoined as
`slice(0,5) + " " + slice(5)`. On a ten-key row that injects a double space
mid-row: editing one character in `a b c d e f g h i j` produced
`a b c X e f g h i j` (S008).

Not cosmetic — the row is re-serialized, so every later edit re-splits it
(S008). And because the split fell in the middle row on essentially every
shipped layout, it fired on nearly any layout a user opened (S008).

The fix replaces a token positionally and rebuilds the row from **its own
gaps**, so each gap is preserved individually (S008). A single hardcoded
separator would have destroyed hand alternation just as thoroughly (S008).

## The assertion is what makes the test real

The test asserts the edited row equals the original with exactly one token
swapped, and that non-edited rows are byte-identical (S008). A test checking
only that the target key changed would have passed against the broken code
(S008).

Mutation-checked both directions: 126 assertions, 0 failures against the fix,
116 failing against the original 5+5 split (S008). Both corrected and original
were caught, which is the check `archive-path-containment.md` argues for.

Related: [Layout File Lifecycle](./layout-file-lifecycle.md) ·
[Shipped Data Files Layout](./shipped-data-files-layout.md)