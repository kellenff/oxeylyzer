---
title: Exact-Integer Scoring Is An Invariant
type: concept
sources: [S010, S013]
updated: 2026-10-13
---

Every score comparison in the app — which layout is better, whether a swap is an
improvement, when generation has converged — is an exact integer comparison
(S010).

So **"identical scores" means bit-for-bit equal, not approximately equal**
(S010). A performance change that shifts a score by one unit changes which
layouts users are shown and which get saved: a different product, delivered
silently (S010).

Four requirements exist only to hold this line — two functional requirements and two
success criteria (S010):

| ID | Requires |
|---|---|
| FR-001 | identical scores before and after the change, for every layout swap |
| FR-008 | no score's sign, magnitude, or ordering changes |
| SC-001 | cached score equals uncached score exactly, on a layout swap |
| SC-005 | generated layouts unchanged for a fixed seed and budget |

The invariant is not enough on its own: two exact-integer standards can both be
"correct" and still disagree, in which case the top-10 a user sees is not the
one the optimizer produced. Every surface that ranks layouts must sort by the
same standard the search maximizes (S013) — see
[One Scoring Standard](./one-scoring-standard.md).

## A performance win is not a performance change

The reference implementation exists purely to make this checkable —
[Why obsolete.rs Stays](./why-obsolete-rs-stays.md). Without an independent way
to compute the same answer, "identical" is an assertion with nothing behind it
(S010).

Related: [Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[Finger Usage Is Summation Plus Penalty](./finger-usage-is-summation-plus-penalty.md) ·
[One Scoring Standard](./one-scoring-standard.md)