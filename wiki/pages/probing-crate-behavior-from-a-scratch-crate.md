---
title: Probing Crate Behavior From A Scratch Crate
type: howto
sources: [S012, S013]
updated: 2026-10-13
---

When a question is "what does this API *actually* do" rather than "what does the code
read like", build a throwaway crate that depends on the real one by path and run it.
Nothing in the repo is created or modified (S012).

## Procedure

1. Scaffold a throwaway crate outside the repo, depending on the crate by path — this
   is what lets the probe leave no repo file created or modified (S012):

   ```toml
   [dependencies]
   oxeylyzer-core = { path = "/abs/path/to/oxeylyzer/oxeylyzer-core" }
   ```

2. Write a `main.rs` that prints observed output, one probe per claim under test (S012).
3. Run it, then remove the scratch crate. The probe crate used here was `deadkey-probe`,
   built against `oxeylyzer-core` by path from outside the repo (S012).

## What to probe, and why it beat reading

The corpus-cleaner question was a *direction* dispute: does `CorpusCleaner` map corpus
char → keystrokes, or keystrokes → corpus char (S012)? Reading the source had already
produced both answers in succession — an assessment verdict of `valid` followed by a fix
run that disproved it — so another read was not the tiebreaker (S012). Running it settled
the matter in one line each (S012):

- the disputed mapping's own documented example;
- a **second mapping in the same crate** believed to be correct — if it agrees with the
  disputed one, the disputed one is not the outlier;
- the report's *expected* behavior. This is the decisive probe: the claim was corpus
  `"´a"` should collapse to `"á"`, and it returned two `U+FFFD` replacement characters,
  so the wanted behavior demonstrably does not exist rather than merely differing in
  direction (S012);
- a control case that should be trivially identity, to prove the machinery is being read
  correctly rather than misconfigured (S012).

Prefer probing the *expected* behavior over re-confirming the *observed* one: observed
behavior is what both sides already agree on (S012).

## Limits

- Needs a `pub` entry point; crate-internal private items are unreachable this way (S012).
- Proves mechanism, not intent. It cannot establish what a caller *meant* — only that
  some behavior is absent, or present and correct (S012).
- A path dependency into a workspace member works from anywhere: `oxeylyzer-core` uses
  `{ workspace = true }` dependencies, and Cargo resolves them by finding that member's
  own workspace root from the given path, so the scratch crate does not need to be a
  workspace member itself (S012).
- Where the crate is already tested, read those tests first — the shipped assertion is
  cheaper than a probe and is what CI actually enforces (S012).

Where the crate is already tested, read those tests first (S012). When the question is
off the probe path — does a *fix* remove the symptom — go one step further and diff the
pre-fix and post-fix outputs; see
[A/B A Fix With Worktrees And A Scratch Binary](./ab-fix-with-worktrees.md).

Related: [Corpus Cleaner Direction](./corpus-cleaner-direction.md) ·
[A/B A Fix With Worktrees And A Scratch Binary](./ab-fix-with-worktrees.md) ·
[Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md)