---
title: Declare "Current" As A CI-Computed Window
type: decision
sources: [S014]
updated: 2026-10-13
---

This fork is the trunk of an abandoned project, so nothing upstream will ever
tell the repo it is behind. Feature 009 makes "current" a checked property
rather than a remembered one, and two choices in it are worth keeping after the
feature's commit lands (S014).

## The MSRV window is computed in CI, never pinned in a manifest

The policy is "the three most recent stable Rust releases". Evaluating that
against a pinned literal would rot the moment the feed moved, so CI fetches the
Rust dist channel feed at build time and derives the floor as the oldest of the
latest three. The manifest's `rust-version` is a **mirror**, not the truth: CI
fails when the computed floor and the declared value diverge, and additionally
builds the workspace on the floor toolchain so the declaration is proven rather
than asserted (S014).

**Rejected alternative:** pin the floor and bump it by hand on a schedule
(S014). The repo's own history is the argument against it — `19a2b91`,
`ddd62b2`, and `03c4a9c` were all ad-hoc cleanups that produced no policy and
were followed by more drift (S014).

## The window wins collisions with dependency recency

When a dependency's inherited `rust-version` falls outside the window, the
window wins: the dependency is held at its newest compatible version and the
hold is recorded in an explicit file with its reason, then surfaced by the
freshness report instead of staying silent (S014). The freshness step therefore
reports two different things — "behind" and "held (recorded)" — so a deliberate
hold cannot be mistaken for neglect, and a hold that becomes stale (its latest
now fits the window) fails until removed (S014).

## Fail closed, never skip silently

The audit and freshness steps exit non-zero when the network is unavailable,
printing that the build is not advisory-verified (S014). A gate that goes
quietly green without running is the exact failure this feature exists to
prevent: a stale nested `Cargo.lock` sat undetected in the tree for months
because nothing ever checked (S014).

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Zip 9 Normalizes Paths That Zip 2 Refused](./zip9-normalizes-paths-that-zip2-refused.md) ·
[Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md)