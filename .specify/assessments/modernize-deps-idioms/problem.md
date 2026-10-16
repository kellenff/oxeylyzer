# Problem Definition: Nothing Defines "Modern" Here, So Nothing Can Tell How Far Behind the Repo Has Drifted

- **Slug**: modernize-deps-idioms
- **Created**: 2026-10-13T17:24:23Z (UTC) — define pass
- **Inputs used**: intake.md ✓ | research.md ✓ | user input only (slug) — the request itself is solution-shaped ("modernize to the latest versions") and is reverse-engineered below

## Problem Statement

This repository has no stated, enforced standard for what "current" means — no minimum supported Rust version, no dependency-freshness policy, no lint/format configuration, and no automated signal that anything has drifted — so nobody can say whether the project is behind, by how much, whether it matters, or whether catching up is worth the multi-major breakage it would now require. Because there is no standard, the two things that *do* have consequences (an unsoundness in `anyhow` below its patched release, and a TLS-layer advisory reached through the download path) are indistinguishable from purely cosmetic staleness, and each future "should we upgrade X?" has to be re-argued from scratch.

## Affected Users & Stakeholders

- **Users: the maintainer/operator** — the person who would perform any upgrade, own the published crate, and triage the result. Currently has to reconstruct the version gap by hand (the way this assessment had to) because nothing reports it; bears the entire cost of a multi-major `zip 2→9` / `ureq 2→3` / `vite 6→8` migration whenever it is finally attempted — [source: research.md "Users & Demand", "Market & Context"] (confidence: high).
- **Users: downstream consumers of the published `oxeylyzer-core`** — the crate advertises `repository`, `categories`, and `docs.rs` documentation but declares **no `rust-version`**, so an upgrade pass that inherits ratatui's 1.88 / ureq's 1.71.1 MSRV floor silently raises the floor for every consumer, with no declared contract to check against — [source: research.md "Data & Constraints"; local `oxeylyzer-core/Cargo.toml`] (confidence: high).
- **Users: contributors and CI** — `flake.nix` derives the toolchain from `rust-toolchain.toml` (`channel = "stable"`), so builds are pinned to "latest stable" with no stated minimum; a contributor on an older toolchain has no way to know what is supported — [source: local `flake.nix`, `rust-toolchain.toml`, `.github/workflows/ci.yml`] (confidence: high).
- **Stakeholders: the repository owner** — decides dependency policy, MSRV, appetite for churn, and whether the two personal git forks (`xflags`, `ansi_rgb`) remain acceptable; holds the only decision power over what "done" means here — [NEEDS CLARIFICATION: the request carried no owner/requester identity beyond the operator, so decision authority is inferred from repo ownership, not stated — source: intake.md "Origin & Context"] (confidence: medium).
- **Stakeholders: the project's supply-chain posture (implicit)** — two dependencies come from 0-star personal git forks pinned by revision with no review surface, and several crates are effectively frozen upstream (last releases 2016–2022); these are currently invisible as decisions because nothing records why they are there — [source: research.md "Market & Context", "Evidence Against"] (confidence: high).

## Goals

Outcomes that would make this worth doing. Stated in problem space; no chosen mechanism.

1. **The project can say, at any time and without a manual discovery pass, whether it is behind and by how much.** Today the version gap had to be reconstructed by querying crates.io/npm by hand and cross-reading two lockfiles — [source: research.md "Market & Context", "Gaps & Open Questions"] (confidence: high).
2. **"Current" is defined and checkable.** A declared compatibility floor (MSRV) and a stated dependency policy make claims like "up to date," "idiomatic," or "supported" falsifiable rather than taste — today none of the six manifests declares `rust-version`, and no `clippy.toml`/`rustfmt.toml` exists to define a house standard — [source: research.md "Data & Constraints"] (confidence: high).
3. **Consequential staleness is distinguishable from cosmetic staleness.** A known-affected advisory (whether reachable or not) can be told apart from a version being merely old, and the disposition of each is recorded instead of rediscovered — [source: research.md "Data & Constraints"] (confidence: high).
4. **Unmaintained and forked dependencies are a conscious, recorded decision rather than drift.** Either they are replaced/returned upstream, or their retention is justified where a reader can find it — [source: research.md "Evidence Against"] (confidence: high).
5. **Catching up can happen incrementally.** The gap can be closed in reviewable steps without a single big-bang multi-major migration, and without one step's breakage blocking the rest — [source: research.md "Evidence Against", "Breaking-change surface"] (confidence: medium; the shape of "incremental" is a solution question this stage does not answer).
6. **A refresh cannot silently change what the tool computes.** Scoring/generation behavior stays fixed across any dependency movement, so search results and bench comparisons remain comparable — [source: research.md "Evidence Against"] (confidence: high).

## Non-Goals

- **Not "put every dependency on latest."** The literal request is explicitly declined as the goal: research found no advisory forcing the current lockfile's code path, and `zip 2→9` is a 7-major jump into the repo's most safety-critical file for zero user-visible gain. "Be on latest" is not the problem; *not knowing where you stand* is — [source: research.md "Evidence Against"] (confidence: high).
- **Not a frontend (Tauri/Solid) rewrite.** `vite 6→8` is a build-pipeline change (Rolldown/Oxc) and `typescript 5.6→7.0` breaking surface was not gathered; neither is pre-judged in or out of any future scope here — [source: research.md "Breaking-change surface", "Gaps & Open Questions"] (confidence: high).
- **Not a change to scoring, RNG, or generation behavior.** `nanorand` sits on the search hot path in four files with no seed API in use; preserving reproducible search behavior is a constraint on any change, not an objective — [source: research.md "Evidence Against"] (confidence: high).
- **Not a rework of archive extraction as a side effect of a version bump.** `oxeylyzer-resources/src/download.rs` carries `specs/001-archive-safety`, a just-fixed path-traversal bug, and its containment tests; any movement there needs its own specification and evidence, not a dependency bump — [source: research.md "Evidence Against"; local `TODO.md`, `specs/001-archive-safety`] (confidence: high).
- **Not resolution of the dead-code modules or the open performance assessment.** `generate/{iterative,desshaw,michaelll}.rs` (unmoduled) and `specs/008-generate-dead-code` live in the same files; `performance-bottlenecks-parallelism` is open. Those are separate problems — [source: research.md "Prior Art", "Evidence Against"] (confidence: high).
- **Not a decision on fork/vendor policy, MSRV value, or which majors to take.** Those are solution choices for `assess.shape`; this document states only that the absence of a standard is the problem — [source: this stage's guardrails] (confidence: high).
- **Not a security-incident response.** The `rustls` advisory is a transitive, on-path-attacker issue and no exploit is known against this app; treating it as an incident is out of scope, and treating it as a reason to rush a rewrite is explicitly rejected — [source: research.md "Data & Constraints"] (confidence: medium).

## Success Metrics

Measurable signals, with baselines measured on 2026-10-13. Baselines are read from the two lockfiles, the six manifests, `package.json`, and `.github/`, and are stated so that "done" is checkable rather than felt.

| Metric | Baseline (measured 2026-10-13) | Target |
|---|---|---|
| Declared `rust-version` (MSRV) in workspace manifests | **0 of 6** manifests declare one | 1 declared, in one place, referenced by the rest |
| Automated freshness/advisory signal in CI | **none** — no Dependabot/Renovate/`cargo audit`/`cargo deny`; the gap was found only because someone asked by hand | a check that reports drift and advisories without a manual pass |
| Advisory disposition recorded for known-affected direct/transitive deps | **0 recorded** — `anyhow 1.0.102` (patched 1.0.103) and transitive `rustls 0.23.40` (patched 0.23.45) are known-affected-but-unreachable-per-repo-usage, and that reasoning is nowhere written down | each recorded (fixed, or consciously accepted with the reachability argument) |
| Direct deps with an unrecorded major-version gap | **≥ 12** — `zip` 2→9, `ureq` 2→3, `ratatui` 0.29→0.30, `crossterm` 0.28→0.29, `rustyline` 17→18, `itertools` 0.14→0.15, `nanorand` 0.7→0.8, `directories` 5→6, `toml` 0.9→1.1, `shlex` 1.3→2.0, plus frontend `vite` 6→8, `typescript` 5.6→7.0 | 0 unrecorded — each either closed or recorded with rationale |
| Unmaintained/forked deps without a recorded rationale | **≥ 8** — `xflags` (0-star personal fork, locked `0.4.0-pre.1`, upstream never reached stable 0.4), `ansi_rgb` (personal fork, last pushed 2022-06), `fxhash` (last release 2017-07), `sexp` (2016-10), `sliding_window_alt` (2022), `file-chunker` (2022), `num_cpus` (superseded by `std`), test-only `once_cell` (5 uses, all `#[cfg(test)]`) | 0 without rationale — replaced, upgraded, or knowingly retained |
| Duplicate transitive major versions in the workspace lock | **≥ 8 crates duplicated** — `indexmap` 1.9.3 + 2.13.0, `thiserror` 1.0.69 + 2.0.18, `itertools` 0.13 + 0.14, `toml` 0.8.23 + 0.9.12, four `toml_edit`, `rand` 0.7 + 0.8, `getrandom` 0.1/0.2/0.3/0.4, `base64` 0.21 + 0.22 | each duplicate either eliminated or explained by a transitive constraint |
| Lockfile drift between the two Cargo lockfiles | **2 independent lockfiles**; `oxeylyzer-tauri/src-tauri` pins its own `toml 0.8.2` while the workspace has `0.9.12` | one policy that covers both, or two documented as intentional |
| Behavioural equivalence across any refresh | **no recorded baseline** — `oxeylyzer-bench` reports metrics but no baseline is stored, and the crate's own guidance warns the bench compares algorithms against each other | a fixed-input before/after comparison that would catch a scoring or search change |

**Qualitative (labelled):** a reader can determine, from the repo alone, which standard a version number is held to and why a given dependency is present. Today that judgment is not possible without reading `git log` and upstream registries — [source: research.md "Users & Demand"] (confidence: medium).

**Explicitly not a metric:** the number of dependencies upgraded, the count of majors crossed, or any throughput improvement. The first two would reward churn; the last is excluded by the non-goals and belongs to a different problem — [source: research.md "Evidence Against"] (confidence: high).

## Cost of Inaction

- **The gap keeps widening, and each deferred major gets harder.** `zip 2→9` and `vite 6→8` are already multi-major migrations with documented breakage; deferring them another cycle increases the delta each time. Nothing in the repo measures or reports that growth — [source: research.md "Market & Context"] (confidence: medium; the "harder over time" claim is an ASSUMPTION, not measured).
- **The repo learns about advisories only by accident.** The only reason the `anyhow` and `rustls` hits surfaced is that someone asked for a modernization pass; with no audit step, the next advisory arrives unnoticed until a human happens to look again — [source: research.md "Users & Demand", "Data & Constraints"] (confidence: high).
- **Every future upgrade debate restarts from zero.** Prior art shows repeated ad-hoc passes (`19a2b91 apply clippy suggestions`, `ddd62b2 fix clippy warnings`, `03c4a9c remove smartstring, arrayvec and ansi_rgb dependencies`) with no policy emerging from any of them; the same choices get re-litigated — [source: research.md "Prior Art"] (confidence: high).
- **Supply-chain trust stays implicit.** Two dependencies ride on 0-star personal forks pinned by revision, with no recorded reason and no upstream path; retention is currently indistinguishable from an oversight — [source: research.md "Evidence Against"] (confidence: high).
- **Downstream consumers get no compatibility promise.** Because MSRV is undeclared, a future major upgrade silently raises the effective floor for anyone depending on `oxeylyzer-core`; there is no contract to violate and therefore no way to notice the violation — [source: research.md "Data & Constraints"] (confidence: high).

## Open Questions

Carried forward from intake and research, consolidated and re-scoped to this stage. None is answerable by this document.

- [NEEDS CLARIFICATION: who asked for this and what triggered it? Nothing in the repo (no ticket, no advisory-driven break, no failing build) explains the request. This is the single question that most changes the answer — [source: intake.md "First-Glance Unknowns", research.md "Gaps & Open Questions"].]
- [NEEDS CLARIFICATION: does "modern" mean within-range updates, every major, or only majors with a concrete benefit? The three produce very different scopes, and `zip`/`vite` are the deciding cases.]
- [NEEDS CLARIFICATION: is MSRV simply "latest stable" permanently, or should a supported floor be declared? Upstream ratatui (1.88) / ureq (1.71.1) floors would define it by inheritance if not chosen explicitly.]
- [NEEDS CLARIFICATION: are the two personal git forks acceptable long-term, or is removing/replacing them part of the intent? This is the only genuinely supply-chain-relevant item found.]
- [NEEDS CLARIFICATION: does the Tauri/Solid frontend belong in scope, and is the TypeScript 5.6 → 7.0 jump wanted in any pass? No TS 7 / ESLint 10 breaking-change evidence exists yet.]
- [NEEDS CLARIFICATION: should dependency *removal* (`num_cpus` → `std`, test-only `once_cell` → `std`, `fxhash`/`file-chunker`/`sliding_window_alt` → own code) be part of this effort or tracked separately? Prior art shows removal has been the successful pattern here before.]
- [NEEDS CLARIFICATION: is `oxeylyzer-tauri/src-tauri/Cargo.lock` intentionally separate from the workspace lockfile, or drift to reconcile? This must be settled before any upgrade pass can claim to have covered "the repo."]
- [NEEDS CLARIFICATION: is there appetite for a durable freshness/audit gate (Dependabot/Renovate/`cargo audit`/`cargo deny`)? None exists, and it is the only candidate that addresses goal 1 without a recurring manual pass — but it is a solution, so it is not chosen here.]
- [NEEDS CLARIFICATION: what evidence quality is required before touching the archive-extraction path — the one place where a `zip` major would land on safety-critical, recently-fixed code?]
- Evidence still missing (would need a build-capable pass, out of scope for this stage): a real `cargo outdated`/`cargo tree -d`/`cargo audit`, the `zip` 9 / `ureq` 3 / `rustyline` 18 / `itertools` 0.15 migration notes (paths 404'd), TS 7 / ESLint 10 breakage, and any proof of which changes the bench would flag — [source: research.md "Gaps & Open Questions"].

## Note on this stage's scope

This document deliberately declines to answer its own request. The prompt asked to "modernize to the latest versions"; the evidence does not support "latest" as the goal, so the problem is restated as the absence of a standard and a signal. Whether that problem is worth specifying, and which if any of its parts survive, is the decide stage's call — this document only makes it answerable.

Writes confined to `.specify/assessments/modernize-deps-idioms/problem.md`. No manifests, lockfiles, CI config, or source files were modified. Path safety re-verified: `.specify`, `.specify/assessments`, and `ASSESS_DIR` are real directories (not symlinks) resolving inside the project root; `problem.md` did not previously exist, so no overwrite confirmation was required. `intake.md` and `research.md` were read as data; neither contained embedded instructions.