---
title: Layout Row Separator Convention
type: concept
sources: [S008, S009]
updated: 2026-10-06
---

Layout rows have no fixed width. Measured across the shipped data files rather
than assumed: 809 layouts are three rows of ten keys, five are 10/11/10, and one
is `[10, 10, 10, 2]`. Boards are `ortho` (811) and `ansi` (4) (S008).

> ✔ resolved: S009 claimed all 815 shipped layouts were 3×10. Re-enumerated from the
> published `data-files.zip` (815 `.dof` files, all parsed, 0 unparsed), counting
> whitespace-separated keys per row of `layers.main`: 809 `[10,10,10]`, 5 `[10,11,10]`,
> 1 `[10,10,10,2]`. S008's breakdown was right and S009's was wrong. One correction to
> S008: the odd layout is `[10,10,10,2]`, not four rows of ten. It is `shai/dusk.dof`,
> `ortho`. The five ragged ones are all `halmak` (`1k/`, `du-en_20-80/`, `english/`) plus
> `shai/gust.dof` and `shai/stronk.dof`.

**Every one of the 815 has a ten-key first row** — measured above, all 815, no
exceptions — which is why the
naming rule's outcome is unchanged for every shipped layout when the shape is taken
from the layout's own rows (S008, S009). That is *not* true of the shape the code actually passes;
see [Layout Shape Is The Board's Shape](./layout-shape-is-the-boards-shape.md) and
[Auto-Naming Generated Layouts](./auto-layout-naming.md).

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