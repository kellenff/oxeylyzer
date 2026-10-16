---
title: A/B A Fix With Worktrees And A Scratch Binary
type: howto
sources: [S013]
updated: 2026-10-13
---

To prove a fix removes a symptom — rather than re-reading the diff — build the
reproducer twice, against the same commit, without touching the working tree
(S013).

## Procedure

1. Create two worktrees at the same base: `git worktree add /tmp/pre HEAD` and
   `git worktree add /tmp/post HEAD`, then apply the working-tree diff to `post`
   (S013).
2. Add the reproducer as a cargo `example` in each tree. An example reaches the
   crate's `pub` API, so the real tree stays unmodified (S013).
3. Run both and diff the output (S013).

## Proving a new test is pinned

Splice the added `#[test]` into the pre-fix tree and run it: it must **fail**
there (S013). A test that passes on both sides proves nothing (S013).

## Worked example: score-stretch-divergence

Pre-fix: 2/10 top-10 overlap and a ~27.1e9 score gap between the display and
search standards; post-fix: 10/10 and a gap of exactly 0 (S013). The regression
test failed pre-fix (`-19378971018` vs `-31833064819`) and passed post-fix
(S013).

## Limits

- Two full builds, so run the reproducer at `--release` for pools large enough
  to matter (S013).
- It reaches `pub` API only; a headless run cannot exercise the Tauri GUI (S013).

Related: [Probing Crate Behavior From Scratch Crate](./probing-crate-behavior-from-a-scratch-crate.md) ·
[One Scoring Standard](./one-scoring-standard.md)