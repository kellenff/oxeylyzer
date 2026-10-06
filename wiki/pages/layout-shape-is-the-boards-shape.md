---
title: Layout Shape Is The Board's Shape
type: component
sources: [S009]
updated: 2026-10-06
---

`Layout.shape` and `FastLayout.shape` are **the form factor's fingering-table row
widths**, taken from `board.shape()` during `From<Dof> for Layout`
(`oxeylyzer-core/src/layout.rs`). They are *not* the row widths of the layout's own
`main` layer (S009).

Measured against libdof 0.25.0, the two tables the app can actually produce (S009):

| Board | `board.shape()` | Layer rows in shipped layouts |
|---|---|---|
| `ortho` | `[10, 10, 10, 6]` | `[10,10,10]` in 808 of 811 |
| `ansi` | `[14, 14, 13, 12, 8]` | `[10,10,10]` or `[10,11,10]` in all 4 |

The gap is not small. For an `ansi` board `shape.inner()[0]` is **14**, while the
layout's own first row is **10** keys wide. Anything that treats `shape` as a
description of the layout's own rows is wrong on every `ansi` layout — verified by
running the real `libdof` conversion path, not by reading the table (S009).

## Why it matters: the shape is not the layout's row count either

`board.shape()` describes the whole physical board, including rows the layout does not
occupy — `ortho` declares four rows, and the 808 three-row layouts fill only the first
three. So `shape.inner().iter().sum()` (36 for `ortho`) overstates the key count of a
30-key layout. Anything equating the two is wrong on `ortho` too (S009).

Only the first element is right often enough to be tempting, and it is exactly right for
`ortho` — which is why an error here survives a test suite written against
`ortho`-shaped fixtures (S009).

## Why the shipped tests missed it

Every `auto_name` test builds its shape by hand —
`Shape::from(vec![10,10,10])`, `[10,10,10,10]`, `[13,12,12]`, `[10,2]` (S009). **No real
board produces any of those.** `board.shape()` yields `[10,10,10,6]` for `ortho` and
`[14,14,13,12,8]` for `ansi`. The test shapes are synthetic arrangements, so the tests
can pass while every real board behaves differently (S009). A test that parses a `.dof`
and uses the `shape` the parser returns would not have this blind spot.

Consumed by: [Auto-Naming Generated Layouts](./auto-layout-naming.md) ·
[Layout File Lifecycle](./layout-file-lifecycle.md)