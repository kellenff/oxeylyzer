---
title: Auto-Naming Generated Layouts
type: component
sources: [S009]
updated: 2026-10-06
---

A generated layout with no name is named from the first four keys of its second row —
the convention that names "Colemak" from `c o l e` (S009).

`auto_name(keys, shape, mapping)` lives in `oxeylyzer-core/src/fast_layout.rs`. The
rule: `skip` is the first row's declared width, used only when the board has at least two
rows, clamped to `keys.len()`; otherwise `skip` is 0 (S009). It replaced a fixed
`skip(10).take(4)` duplicated in three crates — see
[Why One Shared Naming Rule](./why-one-shared-naming-rule.md).

| Site | Behavior |
|---|---|
| `layout.rs` `From<FastLayout> for Layout` | names on conversion; **short-circuits on `Some(name)`** (S009) |
| `repl.rs` `placeholder_name` | names, then chains the collision counter's digits, replaces `*` with `-` (S009) |
| `lib.rs` `save_generated` | `name.unwrap_or_else(…)` — a chosen name wins (S009) |

## ⚠ It reads the board's shape, not the layout's

`shape` is `board.shape()` — the form factor's fingering-table widths, **not** the
layout's own row widths. `ansi` declares a 14-key first row for layouts whose first row
is 10 keys (S009). So four shipped `ansi` layouts get a different name than before:

| Layout | board.shape() | old `skip(10)` | new |
|---|---|---|---|
| `english/halmak.dof` | `[14,14,13,12,8]` | `shnt` | `,.ae` |
| `shai/gust.dof` | `[14,14,13,12,8]` | `eiac` | `ydht` |
| `shai/stand.dof` | `[14,14,13,12,8]` | `stnd` | `yxra` |
| `shai/stronk.dof` | `[14,14,13,12,8]` | `strn` | `kyma` |

Measured by running the real `libdof` parse path (`Dof` → `board.shape()` → the shipped
`auto_name` body) over the published archive (S009). **SC-002 is violated** — it required
all 815 shipped layouts to keep their names, and 4 do not. The `.min(keys.len())` clamp
does not help: the keys are present, just the wrong ones. The rule is correct only for
`ortho`, where the board's first row coincidentally matches the layout's.

Full derivation and the test blind spot:
[Layout Shape Is The Board's Shape](./layout-shape-is-the-boards-shape.md) (S009).

## ⚠ The stated motivating defect does not reproduce

> ✔ resolved: S009 argued the real defect was a four-row board, where `skip(10)` lands
> on row 0 and names the layout `1 2 3 4`. Running the old rule on `[10,10,10,10]` gives
> `klmn` — index 10 starts row **1**, not row 0. The
> `auto_name_uses_the_second_row_on_a_four_row_board` test asserts `klmn`, so it passes
> against the pre-change code and guards no regression. Settled by running both rules;
> the genuine divergence is a board-supplied first row ≠ the layout's, which is what the
> `ansi` case above exposes.

The remaining divergence cases, checked by running both rules: the two differ exactly
when the given shape's first row is not ten keys wide, or the shape has fewer than two
rows. A narrower first row made the old offset reach one key past the home row's start; a
wider one started it on the wrong row (S009).

Related: [Layout Row Separator Convention](./layout-row-separator-convention.md) ·
[Layout Shape Is The Board's Shape](./layout-shape-is-the-boards-shape.md) ·
[Layout File Lifecycle](./layout-file-lifecycle.md) ·
[Why One Shared Naming Rule](./why-one-shared-naming-rule.md)