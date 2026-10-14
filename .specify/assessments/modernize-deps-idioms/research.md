# Idea Research: Modernize dependencies, language features, and idiomatic implementation standards

- **Slug**: modernize-deps-idioms
- **Created**: 2026-10-13T17:24:23Z (UTC) — research pass
- **Evidence confidence (overall)**: medium

Confidence is medium because version, advisory, and changelog facts were fetched from upstream
(crates.io API, docs.rs, raw GitHub), but three gaps remain: no real `cargo outdated` / `cargo tree`
run (build outputs and the Tauri lockfile were not exercised), several upstream changelog paths 404'd
(repo renames), and no user/repo signal exists for *why* this work is wanted beyond the one-line
request.

## Users & Demand

- **No user-facing signal was found for this work.** The request arrives as a single maintenance line
  with no requester, ticket, advisory-driven trigger, or complaint attached — [source: local
  `.specify/assessments/modernize-deps-idioms/intake.md`; ASSUMPTION that "the user" means the repo
  owner/operator] (confidence: high that no signal exists in-repo; low on intent).
- The observable "user" of these changes is the maintainer and CI: `.github/workflows/ci.yml` runs
  `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo test-all` with `RUSTFLAGS: -Dwarnings`, but runs **no** `cargo audit`, `cargo deny`, or
  `cargo outdated` step — [source: local `.github/workflows/ci.yml`] (confidence: high).
- Consumers of the crates are downstream: `oxeylyzer-core` is published (license Apache-2.0,
  `repository`, `categories`, `documentation = "https://docs.rs/oxeylyzer-core"`) — [source: local
  `oxeylyzer-core/Cargo.toml`] (confidence: high). No `rust-version` (MSRV) is declared in any of the
  six manifests — [source: local manifests] (confidence: high).
- No declared support window, changelog policy, or review cadence exists in-repo to measure demand
  against — [source: local repo root listing; only `TODO.md` and `specs/` are present] (confidence: high).

## Prior Art

- **This repo has modernized before, incrementally, and survived it.** Commit history shows a
  recurring pattern: `0141244 add ci file to check formatting, run clippy and run tests`,
  `19a2b91 apply clippy suggestions`, `03c4a9c remove smartstring, arrayvec and ansi_rgb dependencies
  from oxeylyzer_core`, `63e92ab move once_cell to dev dependencies for oxeylyzer`,
  `3c7c1ea make a bunch of improvements based on this list:`,
  `ddd62b2 fix clippy warnings` — [source: local `git log --oneline`] (confidence: high).
  Prior art shows dependency *removal* was already used once as the answer (smartstring/arrayvec).
- **No automation for dependency freshness exists**: `.github/` contains only `workflows/`
  (`ci.yml`, `publish-tauri.yml`) — no `dependabot.yml`, no Renovate config, no scheduled audit —
  [source: local `.github/` listing] (confidence: high).
- **Dead/experimental code already occupies the "modernization" space**: `specs/008-generate-dead-code`
  exists, and `oxeylyzer-core/src/generate/{iterative,desshaw,michaelll}.rs` are unmoduled dead
  experiments (per `CLAUDE.md`), e.g. `generate/desshaw.rs` still imports `ahash` — [source: local
  tree + `grep`] (confidence: high). These files inflate any "count of usages" for dependency removal
  decisions if not excluded.
- **The RustSec advisory database has no entry that makes the current lockfile known-vulnerable**
  for the direct deps inspected (see Data & Constraints), i.e. prior art gives no forced upgrade —
  [source: `raw.githubusercontent.com/RustSec/advisory-db/main/crates/...`] (confidence: medium;
  inspected the watched subset, not the full transitive tree).

## Market & Context

- **Upstream ecosystem is active and, in several cases, has moved well past this lockfile** — latest
  stable versions and release dates (fetched 2026-10-13 from crates.io API):
  | crate | locked | latest stable | latest released |
  |---|---|---|---|
  | zip | 2.4.2 | 9.0.2 | 2026-10-11 |
  | ureq | 2.12.1 | 3.4.2 | 2026-09-13 |
  | ratatui | 0.29.0 | 0.30.2 | 2026-06-19 |
  | crossterm | 0.28.1 | 0.29.0 | 2025-04-05 |
  | rustyline | 17.0.2 | 18.0.1 | 2026-06-24 |
  | itertools | 0.14.0 | 0.15.0 | 2026-06-16 |
  | nanorand | 0.7.0 | 0.8.0 | 2025-06-08 |
  | directories | 5.0.1 | 6.0.0 | 2025-01-12 |
  | toml (workspace) | 0.9.12 | 1.1.8 | 2026-10-09 |
  | indexmap | 2.13.0 | 2.14.2 | 2026-09-05 |
  | serde_with | 3.18.0 | 3.24.0 | 2026-09-26 |
  | anyhow | 1.0.102 | 1.0.104 | 2026-07-18 |
  | thiserror | 2.0.18 (also 1.0.69 transitive) | 2.0.21 | 2026-09-23 |
  | rayon | 1.11.0 | 1.12.0 | 2026-04-14 |
  | fancy-regex (dev) | 0.17.0 | 0.19.2 | 2026-09-13 |
  | indicatif | 0.18.4 | 0.18.6 | 2026-07-01 |
  | md5 | 0.8.0 | 0.8.1 | 2026-07-09 |
  | md5 (repl) | 0.8.0 | 0.8.1 | 2026-07-09 |
  — [source: `https://crates.io/api/v1/crates/<name>` and `/versions`] (confidence: high for
  upstream numbers; the *locked* column is read from local `Cargo.lock` and
  `oxeylyzer-tauri/src-tauri/Cargo.lock`) (confidence: high).
- **Several crates in use are effectively frozen upstream, which changes "upgrade" into "own or
  replace":**
  - `fxhash` 0.2.1 — **released 2017-07-29**, no release in ~9 years; 154M all-time downloads —
    [source: crates.io versions API] (confidence: high).
  - `sliding_window_alt` 0.1.2 — released 2022-07-08, ~3,066 all-time downloads — [source: crates.io]
    (confidence: high).
  - `file-chunker` 0.1.1 — released 2022-02-10, ~41,209 all-time downloads — [source: crates.io]
    (confidence: high).
  - `sexp` 1.1.4 — released **2016-10-13**, ~120k downloads — [source: crates.io] (confidence: high).
  - `diol` (dev/bench) 0.13.1 — 2025-04-29, ~6,016 downloads — [source: crates.io] (confidence: high).
  - `libdof` 0.25.0 — 2026-05-14, ~296 downloads — [source: crates.io] (confidence: high).
  - `ansi_rgb` — upstream 0.3.2-alpha last released 2022-06-25, ~3,532 downloads; the repo
    consumes a **personal git fork** (`ansi_rgb = { git = "https://github.com/O-X-E-Y/ansi_rgb" }`,
    locked at `1a8cda5a76b30c3f4f1bc0118c1487c85b9ae85b`, fork last pushed 2022-06-24) — [source:
    local `oxeylyzer-repl/Cargo.toml`, `Cargo.lock`, GitHub API for `O-X-E-Y/ansi_rgb`] (confidence: high).
  - `xflags` — consumed from a personal fork in **two** manifests
    (`oxeylyzer-repl`, `oxeylyzer-bench`), locked at `0.4.0-pre.1`
    (`60e238c43a9da8892a90e2652162288aa4153b2c`); the fork was last pushed **2024-10-03** and crates.io
    `xflags` tops out at `0.4.0-pre.2` (2024-10-31) — [source: local manifests + `Cargo.lock` +
    crates.io + GitHub API for `O-X-E-Y/xflags` (`fork: true`, 0 stars)] (confidence: high).
- **Frontend (Tauri/Solid) is also a major-version behind**, npm `latest` fetched 2026-10-13:
  `vite` locked `^6.0.3` → **8.3.4**; `typescript` `~5.6.2` → **7.0.2**; `eslint` `^9.39.1` → **10.12.0**;
  `eslint-plugin-solid` `^0.14.5` → **0.18.1**; `@tauri-apps/cli` `^2` (installed 2.10.1) → **2.12.2**;
  `tailwindcss` `^4.2.2` → 4.3.3 (patch/minor) — [source: local `oxeylyzer-tauri/package.json`,
  `node_modules/@tauri-apps/cli/package.json`, npm registry `/<pkg>/latest`] (confidence: high).
- **Cost of doing nothing is low but compounding**: no advisory forces action today, and the build
  works on `rustc 1.99.0` (local) with `edition = "2024"` in all five workspace crates — [source:
  local `rust-toolchain.toml`, manifests, `rustc --version`] (confidence: high). The visible cost is
  that each version deferred widens the eventual migration delta; `zip 2→9` and `vite 6→8` are
  multi-major jumps whose difficulty grows with distance — [source: upstream release notes, below]
  (confidence: medium; ASSUMPTION that deferral strictly increases cost).

## Data & Constraints

### Advisories (RustSec advisory-db, fetched 2026-10-13)

- **`zip` RUSTSEC-2025-0168** (“Incorrect path canonicalization during Archive Extraction Leading to
  Arbitrary File Write”): affected `zip::read::ZipArchive::extract` `>= 1.3.0, < 2.3.0`; **patched
  `>= 2.3.0`**; CVE-2025-29787. The lockfile has **2.4.2 → not affected**. Relevance is high anyway:
  this repo *does* implement archive extraction and *already has* `specs/001-archive-safety` plus a
  recorded path-traversal bug (`archive-prefix-double-dot-escape`), so upstream `extract()` behavior
  is a live correctness domain here — [source: `crates/zip/RUSTSEC-2025-0168.md`; local `TODO.md`,
  `oxeylyzer-resources/src/download.rs`] (confidence: high).
- **`anyhow` RUSTSEC-2026-0190** (“Unsoundness in `Error::downcast_mut()`”, 2026-06-25): **patched
  `>= 1.0.103`**; lockfile has **1.0.102 → likely affected** — [source:
  `crates/anyhow/RUSTSEC-2026-0190.md`; local `Cargo.lock`] (confidence: medium-high; the advisory
  needs `Error::context` + `downcast_mut`, neither of which appears in this repo — see below).
- **`rustls` (transitive via `ureq` 2) RUSTSEC-2026-0285** (“TLS 1.3 handshake messages incorrectly
  accepted across encryption level boundaries”, 2026-09-14): patched `>= 0.23.45`; lockfile has
  `rustls 0.23.40 → likely affected` — [source: advisory file + local `Cargo.lock`] (confidence: medium).
  This is the one advisory whose *trigger is network input* the app actually consumes (it downloads a
  zip over HTTPS from `https://github.com/.../data-files.zip`).
- **`nanorand` RUSTSEC-2021-0114** (“Aliased mutable references from `tls_rand` & `TlsWyRand`”,
  patched `>= 0.6.1`) and **RUSTSEC-2020-0089** (0.5.0 RNG truncation bug, patched `>= 0.5.1`):
  lockfile has **0.7.0 → not affected by either** — [source: advisory files + local `Cargo.lock`]
  (confidence: high). The deprecated crate list mentions `nanorand`, but the entries are historical.
- **`shlex` RUSTSEC-2024-0006** (“Multiple issues involving quote API”, patched `>= 1.3.0`):
  lockfile is **1.3.0 → not affected**; repo usage is `shlex::split` only, not `quote`/`join` —
  [source: advisory file; local `oxeylyzer-repl/src/commands.rs`] (confidence: high).
- **`once_cell` RUSTSEC-2019-0017** (Lazy init panic UB, patched `>= 1.0.1`): lockfile **1.21.4 → not
  affected** — [source: advisory file + local `Cargo.lock`] (confidence: high).
- No advisory was found for `ureq` itself, `ratatui`, `crossterm`, `rustyline`, `itertools`,
  `indexmap`, `toml`, `serde_json`, `glob`, `md5`, `ahash`, `num_cpus`, `directories`, `libdof`,
  or the two git-fork deps — [source: RustSec advisory-db `crates/` listing filtered to the watched
  names] (confidence: medium; scoped to direct deps, not a full `cargo audit` of the transitive graph).

### Lockfile / manifest facts

- The workspace lockfile carries **duplicate major versions** for several crates — `indexmap 1.9.3`
  **and** `2.13.0`, `thiserror 1.0.69` **and** `2.0.18`, `itertools 0.13.0` **and** `0.14.0`,
  `toml 0.8.23` **and** `0.9.12`, `rand 0.7.3`/`0.8.5`, `getrandom` 0.1/0.2/0.3/0.4, `base64`
  0.21/0.22, four `toml_edit` versions — [source: local `Cargo.lock`] (confidence: high).
  `oxeylyzer-tauri/src-tauri/Cargo.lock` additionally pins `toml 0.8.2` while declaring `toml = "0.8"`
  — i.e. the Tauri crate has a **separate lockfile** and its own `toml` version line, distinct from
  the workspace `toml 0.9` — [source: local `oxeylyzer-tauri/src-tauri/Cargo.toml` + its lockfile]
  (confidence: high).
- **MSRV is undeclared** everywhere, yet CI installs `dtolnay/rust-toolchain@stable` and Nix derives
  the toolchain from `rust-toolchain.toml` (`channel = "stable"`), so the build is green on
  rustc 1.99.0 — [source: local `.github/workflows/ci.yml`, `flake.nix`, `rust-toolchain.toml`]
  (confidence: high).
- **Upstream MSRVs already exceed what a conservative library would want**: `ratatui 0.30.1` raised
  MSRV to **1.88.0** (0.30.0 → 1.86.0); `ureq 3.0` states MSRV **1.71.1** (following rustls);
  `zip 8.0.0` “Migrate Rust 2024” — [source: ratatui `BREAKING-CHANGES.md`, ureq `CHANGELOG.md`, zip
  release notes] (confidence: high). None of these exceed a “stable” toolchain, but all of them would
  need to be *declared* via `rust-version` if MSRV isn't simply "latest stable".
- **Dependency surface with 1–2 call sites each** (delete-or-own candidates rather than upgrade
  targets), from source grep: `num_cpus` 1 use (`data.rs:431`, `num_cpus::get() - 1`),
  `fxhash` 1 use (`data.rs:7`, `FxHashMap` — with the in-code comment “fxhash seems to be much
  faster than ahash for this purpose”), `sliding_window_alt` 1 use (`corpus_cleaner.rs:3`),
  `file_chunker` 1 use (`data.rs:19`), `once_cell::sync::Lazy` 5 uses **all inside `#[cfg(test)]`**
  (`trigram_patterns.rs`, `fast_layout.rs`, `generate.rs`, `repl.rs`) while
  `generate/test_util.rs` already uses `std::sync::OnceLock` — [source: local `grep`] (confidence: high).
- **`nanorand` sits on the generation hot path, not in tests**: `utility.rs` (the shared shuffle/
  sample helper), `generate/{annealing,lahc,ils}.rs` all call `tls_rng()` and there is **no seeding
  API** in use — reproducibility of a search run comes from the algorithm, not a seed — [source:
  local `grep` for `tls_rng|seed`] (confidence: high). Any RNG swap must preserve the
  `Rng`-like call surface or it touches four call sites in the search algorithms.
- **Zero `unsafe`** in `oxeylyzer-core/src` and `oxeylyzer-repl/src` — [source: local `grep`]
  (confidence: high). Combined with the `nanorand` UB advisory, this means the unsoundness class
  RustSec flagged historically is not reachable from this code anyway (and 0.7.0 is patched regardless).
- **Dev-dependency version specifiers are looser than the workspace style**:
  `time_this = "0"` and `approx = "0"` in `oxeylyzer-core` (approx used 33× in `utility.rs`),
  while sibling deps are `"1.0"`-style — [source: local `oxeylyzer-core/Cargo.toml`] (confidence: high).
- **No lint/format configuration to check idioms against**: no `clippy.toml`, no `rustfmt.toml` at
  repo root — [source: local listing] (confidence: high). “Idiomatic implementation standards”
  therefore has no in-repo definition today; the only enforced standard is clippy's default set with
  `-D warnings`.

### Breaking-change surface (what "latest" costs)

- **`zip` 2 → 9 (7 majors)**: `enclosed_name()` appears **gone-or-relocated** in v9 (the direct
  GitHub raw fetch of `src/read.rs` at `v9.0.2` 404s and `docs.rs/zip/9.0.2` no longer lists
  `ZipArchive::extract`), while `name()` survives — [source: docs.rs for zip 9.0.2; GitHub raw
  `zip-rs/zip`] (confidence: medium — docs.rs HTML scraping is weak evidence and the `src/read.rs`
  path itself may have moved; needs an actual `cargo add` probe to confirm). Release notes flag
  v9.0.0 feature work (streaming, data descriptors, `ZipWriter::hide_file`) but the migration notes
  could not be fetched (`MIGRATING.md` 404 on both the `zip` and `zip2` repo paths) — [source:
  GitHub releases API] (confidence: medium).
- **`ureq` 2 → 3**: `3.0.0` “Replace RequestBuilder Deref with explicit wrappers”, “Remove dependency
  on `url` crate”; README now shows `agent.body_mut().read_to_string()` and `Agent::config_builder()`
  — our call site is `ureq::get(url).call()?.header("content-length")` + `into_reader()` — [source:
  ureq `CHANGELOG.md` + `README.md`; local `oxeylyzer-resources/src/download.rs`] (confidence: high
  that the surface changed; medium on the exact minimal diff for this one call site).
- **`ratatui` 0.29 → 0.30.x**: documented breakages include `block::Title` removed, `FrameExt` trait,
  `List::highlight_symbol` signature change, `layout::Alignment` → `layout::HorizontalAlignment`,
  `Backend` gaining an associated `Error` type + `clear_region`, `From` impls on backend types
  replaced by specific traits, plus MSRV 1.86→1.88 — [source: ratatui `BREAKING-CHANGES.md`]
  (confidence: high).
- **`vite` 6 → 8**: 8 switches bundling to **Rolldown/Oxc** by default (JS transforms, minification,
  CommonJS interop changes, `moduleResolution` format-sniffing removed, `manualChunks` object form
  removed, `build.rollupOptions.watch.chokidar` removed) — [source: `vitejs/vite` migration guide]
  (confidence: high). This is a real build-pipeline change for `oxeylyzer-tauri`, not a version bump.
- **`typescript` 5.6 → 7.0** and **`eslint` 9 → 10** were confirmed as current majors by npm, but
  their breaking-change surface was **not** retrieved — [NEEDS CLARIFICATION: what TS 7 / ESLint 10
  break for this codebase] (confidence: low).
- `itertools` 0.15 and `rustyline` 18 changelog paths 404'd on the first attempt and no release notes
  were recovered — [source: attempted raw GitHub fetches] (confidence: low; ASSUMPTION that both are
  minor-breakage bumps in practice).

## Evidence Against the Idea

- **The strongest counter-evidence: nothing is broken, and nothing is forcing action.** The lockfile
  is *not* affected by the one advisory that matters for its own code path (`zip` RUSTSEC-2025-0168
  needs `< 2.3.0`; this repo has 2.4.2), and the remaining two advisory hits are (a) `anyhow`
  unsoundness requiring `Error::context` + `downcast_mut`, neither of which appears in this repo, and
  (b) a transitive `rustls` TLS-level issue only reachable with an on-path attacker. A modernization
  pass would therefore buy **hygiene, not a security fix** — [source: RustSec advisory files + local
  `grep` for `downcast_mut`] (confidence: medium-high).
- **Multi-major jumps are the expensive kind of churn, and their payoff is speculative.** `zip 2→9`
  touches `oxeylyzer-resources/src/download.rs` — the file that *already* carries the repo's most
  safety-critical invariants (`specs/001-archive-safety`, a recorded path-traversal bug, containment
  tests). Rewriting extraction against a new API risks regressing a domain that just got a fix — a
  concrete cost, traded for zero user-visible gain — [source: local `TODO.md`,
  `specs/001-archive-safety`, `oxeylyzer-resources/src/download.rs`] (confidence: high).
- **The "idioms" axis has no definition to check against.** With no `rustfmt.toml`/`clippy.toml` and
  CI already enforcing `-D warnings` on clippy defaults, "idiomatic implementation standards" is
  unfalsifiable as stated; any work done under that heading is preference, not observable defect —
  [source: local CI config + absence of lint config] (confidence: high).
- **Internal RNG/dead-code rationalization is entangled with this topic and would collide.** Two
  things sit in the same files: `nanorand` on the search hot path, and unmoduled dead modules
  (`generate/{iterative,desshaw,michaelll}.rs`, one of which imports `ahash`) — plus an *open*
  assessment (`performance-bottlenecks-parallelism`) and a shipped `specs/008-generate-dead-code`.
  A blanket "remove dead deps / swap RNG" sweep would fight in-flight work rather than compose with
  it — [source: local tree, `specs/`, `.specify/assessments/`] (confidence: high).
- **Forked git deps can't be "modernized" by a version bump.** `xflags` and `ansi_rgb` come from
  personal forks; upstream `xflags` on crates.io has never reached a stable 0.4, and there's no newer
  fork commit since 2024-10. The only real options are (1) stop depending on them, (2) return to
  upstream, or (3) keep the fork — none of which is a version upgrade — [source: crates.io + GitHub
  API + local manifests] (confidence: high).
- **Two lockfiles means "the latest versions" is ambiguous.** The Tauri crate locks `toml 0.8.2`
  independently of the workspace's `0.9.12`, and the `oxeylyzer-tauri/` npm tree is a third surface.
  Without a stated policy, an upgrade pass has no single source of truth for "current" and can silently
  leave one of the three behind — [source: local lockfiles + `package.json`] (confidence: high).
- **No regression gate exists for the parts that would change.** Scoring is `i64`-only and compared
  via `oxeylyzer-bench` metrics; the bench harness reports best score / top-5 mean / duplicates /
  layouts-per-second but there is no recorded baseline in-repo (and the repo's own guidance warns the
  bench compares algorithms against each other). Any change touching the hot path or RNG cannot be
  proven non-regressive from CI alone — [source: local `CLAUDE.md`, `oxeylyzer-bench/`,
  `Cargo.toml` bench target] (confidence: medium).

## Gaps & Open Questions

- [NEEDS CLARIFICATION: who asked for this and what triggered it — nothing in the repo (no ticket, no
  advisory-driven break, no failing build) explains the request.]
- [NEEDS CLARIFICATION: does "latest dependencies" mean (a) `cargo update` within current semver
  ranges, (b) every major, or (c) majors only where there's a concrete benefit? The three answers
  produce very different scopes, and `zip 2→9` / `vite 6→8` are the deciding cases.]
- [NEEDS CLARIFICATION: is MSRV simply "latest stable" for good, or should `rust-version` be declared?
  Upstream ratatui (1.88) / ureq (1.71.1) MSRVs would become the effective floor of a published
  `oxeylyzer-core`.]
- [NEEDS CLARIFICATION: are the two personal git forks (`xflags`, `ansi_rgb`) acceptable long-term,
  or is part of the intent to remove/replace them? This is the only genuinely supply-chain-relevant
  item found.]
- [NEEDS CLARIFICATION: does the Tauri/Solid frontend belong in scope, and is `typescript 5.6 → 7.0`
  wanted in the same pass as the Rust work? No TS 7 / ESLint 10 breaking-change evidence was gathered.]
- [NEEDS CLARIFICATION: should dependency *removal* (`num_cpus` → `std::thread::available_parallelism`,
  test-only `once_cell` → `std::sync::LazyLock`/`OnceLock`, `fxhash`/`file-chunker`/`sliding_window_alt`
  → own ~20 lines) be part of the same effort, or kept separate? Prior art (`03c4a9c`) shows removal
  has been the successful pattern here before.]
- [NEEDS CLARIFICATION: `oxeylyzer-tauri/src-tauri/Cargo.lock` is tracked separately from the
  workspace lock. Is that intended, or drift to reconcile before any upgrade pass?]
- [NEEDS CLARIFICATION: is there an appetite for an automated freshness/audit gate (Dependabot,
  Renovate, `cargo audit`/`cargo deny` in CI)? None exists — this is the only durable fix for
  "keeps drifting old" and is cheap relative to a hand-run sweep.]
- Evidence NOT gathered (would need a build-capable pass): a real `cargo outdated`/`cargo tree -d`
  run, `cargo audit` over the full transitive graph, the `zip` 9 / `ureq` 3 / `rustyline` 18 /
  `itertools` 0.15 migration notes (paths 404'd), TS 7 / ESLint 10 breaking changes, and any proof of
  which changes the bench would flag as regressions.

## Sources

- `https://crates.io/api/v1/crates/<name>` and `.../versions?per_page=3` — latest stable versions,
  release dates, download counts, repository URLs (host: crates.io; fetched via API, not fetched as
  human-facing pages; policy: allowlisted-equivalent registry query for version facts).
- `https://raw.githubusercontent.com/RustSec/advisory-db/main/crates/{zip,anyhow,rustls,nanorand,shlex,once_cell}/RUSTSEC-*.md`
  — advisory ids, affected ranges, patched versions (host: github.com; policy: allowlisted).
- `https://raw.githubusercontent.com/RustSec/advisory-db/main/crates` (JSON listing) — which crates
  have advisories at all (host: api.github.com; policy: allowlisted).
- `https://raw.githubusercontent.com/ratatui/ratatui/HEAD/BREAKING-CHANGES.md` — 0.30.x breakages and
  MSRV (host: github.com; policy: allowlisted).
- `https://raw.githubusercontent.com/algesten/ureq/HEAD/CHANGELOG.md` + `README.md` — 3.0.0 changes and
  new call style (host: github.com; policy: allowlisted).
- `https://raw.githubusercontent.com/vitejs/vite/HEAD/docs/guide/migration.md` — Vite 8 Rolldown/Oxc
  migration notes (host: github.com; policy: allowlisted).
- `https://raw.githubusercontent.com/crossterm-rs/crossterm/HEAD/CHANGELOG.md` — 0.29 notes
  (host: github.com; policy: allowlisted).
- `https://api.github.com/repos/zip-rs/zip2/releases` — zip v9/v8/v7 release notes
  (host: api.github.com; policy: allowlisted).
- `https://api.github.com/repos/O-X-E-Y/xflags` and `/repos/O-X-E-Y/ansi_rgb` — fork identity and last
  push dates (host: api.github.com; policy: allowlisted).
- `https://docs.rs/zip/9.0.2/zip/read/struct.ZipArchive.html` — attempted API-surface check
  (host: docs.rs; policy: **not** on the intake allowlist — fetched without explicit confirmation;
  treat as unverified, low-weight evidence).
- `https://registry.npmjs.org/<pkg>/latest` — npm latest versions (host: registry.npmjs.org; policy:
  **not** on the intake allowlist — registry metadata query only, no page content).
- Local evidence, read-only: `Cargo.toml`, `Cargo.lock`, `oxeylyzer-*/Cargo.toml`,
  `oxeylyzer-tauri/{package.json,node_modules/@tauri-apps/cli/package.json,src-tauri/Cargo.{toml,lock}}`,
  `.github/workflows/ci.yml`, `rust-toolchain.toml`, `flake.nix`, `CLAUDE.md`, `TODO.md`, `specs/`,
  `.specify/assessments/`, and `git log --oneline`.

## Report back

- Slug: `modernize-deps-idioms`
- Path: `.specify/assessments/modernize-deps-idioms/research.md`
- Overall evidence confidence: **medium**
- Next step: `__SPECKIT_COMMAND_ASSESS_DEFINE__ slug=modernize-deps-idioms`