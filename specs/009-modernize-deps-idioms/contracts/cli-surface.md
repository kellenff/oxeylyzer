# Contract: CLI Surface Parity (xflags → clap, ansi_rgb → helper)

**Objective**: prove FR-015 — the two CLIs accept exactly what they accepted before and reject exactly what
they rejected. The parser swap must be invisible.

## Terms

- `ARGS` = one argument vector (for the REPL: the `shlex`-split tokens of one input line, passed to the
  parser the same way `commands.rs::parse_flags` does today via `Repl::from_vec`; for bench: `argv[1..]`).
- `decision(ARGS)` = `accept(parsed-values)` or `reject(error)` — for the REPL a rejection MUST be a
  returned `Err` (the prompt loop prints it and continues), never a process exit; for bench it is the
  `from_env_or_exit` exit path as today.
- Baseline = current xflags build; Candidate = clap build.

> **Verified parse topology** (2026-10-13 analysis pass): the `oxeylyzer` binary (`oxeylyzer-repl/src/bin/main.rs`)
> takes NO argv — `main()` calls `Repl::run()`, which prompts via rustyline; each input line is
> `shlex::split` → `Repl::from_vec(args)` in `oxeylyzer-repl/src/commands.rs`. Parity vectors for the REPL
> therefore drive `from_vec` (or its clap successor `try_get_matches_from`) directly in unit tests, and
> `cargo run -- analyze sturdy` is NOT a valid test harness for the REPL parser. Bench is a plain argv CLI
> (`flags::Bench::from_env_or_exit()` at `oxeylyzer-bench/src/main.rs:18`).

## Parity test method (encoded as table-driven tests in each crate)

1. **Extract the surface from `oxeylyzer-repl/src/flags.rs`** (`xflags!`) and `oxeylyzer-bench/src/flags.rs`.
   For each subcommand and alias, enumerate:
   - every positional (required/optional/repeated, arity)
   - every short/long flag and its arity
   - aliases: `analyze|a`, `compare|c|comp|cmp`, `generate|gen|g|improve|i|optimize`, `save|s`,
     `remove|rm`, `swap`, `rank`, `sfbs`, `fspeed`, `scissors`, `lsbs`, plus REPL bare-`<key>` behavior
     (the REPL's interactive dispatch is *not* a parser concern; the contract covers argv parsing only).
   - `bench`: `-l/--language`, `-b/--basis`, `-n/--count`, `-t/--time`, `-a/--algorithms`, `-p/--pins`,
     `--headless`.
2. **For each enumerated ARGS**, assert `decision_baseline(ARGS) == decision_candidate(ARGS)`:
   - accept vectors: also assert identical parsed values (arity, order of `repeated swaps`, `Option` vs
     `Vec` emptiness, numeric parses for `usize`/`u64`).
   - reject vectors: `missing required`, `unknown subcommand`, `unknown flag`, `bad numeric value`,
     `flag expects value` — the *class* must match; verbatim message text is allowed to differ (parser
     wording is not part of the surface; the accept/reject **outcome and error-return shape** are).
3. At least: one accept case, one missing-required, one unknown-flag, one bad-value, one
   repeated-argument case per subcommand/alias.
4. **Exit-behavior test (REPL only)**: a rejection vector must produce `Err(...)` from the parse call —
   asserting the function *returns* is itself the test that no `std::process::exit` occurred. This pins
   `try_get_matches_from` over `get_matches_from`.

## Migration checklist (from old grammar, must appear in new)

- [ ] all subcommands + all aliases (a `cmd` in xflags with multiple names → clap `alias`/`visible_alias`)
- [ ] `required X: String` → `Arg::required(true)`
- [ ] `optional X: T` → `Arg::num_args(0..=1).value_parser`
- [ ] `repeated X: T` → `Arg::num_args(1..).action(Append)` or `Vec<T>` derive
- [ ] `optional -p, --pins name: String` (flag-with-value) → `Arg::short('p').long("pins")`
- [ ] `--yes` style bare flags → `Arg::action(SetTrue)`
- [ ] `bench` flat-options-only parser: no subcommands, keep `cmd bench`'s single-command shape as clap
      `Command` root (the binary *is* the command; `from_env_or_exit` exit-on-error behavior is preserved)
- [ ] REPL call-path adaptation in `oxeylyzer-repl/src/commands.rs:81` (`Repl::from_vec(args)?` → clap
      `try_get_matches_from`, error mapped in `oxeylyzer-repl/src/repl.rs:98` replacing `XflagsError`):
      the `ReplCmd` subcommand enum shape stays so the `match flags.subcommand` dispatch is untouched
- [ ] REPL `cmd repl` root semantics: xflags `from_vec` expects tokens starting at the subcommand under
      the `repl` root (the `repl` literal itself is NOT part of the line — `parse_flags` feeds it the
      shlexed command text). The clap successor must accept the identical token stream. Verified by T041's
      parity vectors, not by assumption

## ansi_rgb replacement

**Carrier**: small in-repo helper (e.g. `oxeylyzer-repl/src/color.rs`), single import site (`display.rs`).
No user-facing surface (color escape emission only). Proven by: existing REPL tests + manual smoke; no
behavioral contract beyond byte-identical SGR sequences for the used codes.
