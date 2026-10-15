use clap::Parser;

/// Benchmark oxeylyzer search algorithms against each other.
///
/// Flat option set, no subcommands: the binary IS the command. Derived `parse()`
/// keeps bench's existing exit-on-error path (it prints the error and exits
/// non-zero), which is what the old parser's `from_env_or_exit` did.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Bench {
    /// Language corpus to use (default: english)
    #[arg(short = 'l', long)]
    pub language: Option<String>,

    /// Basis layout name; defines the key set and geometry (default: sturdy, or the first found)
    #[arg(short = 'b', long)]
    pub basis: Option<String>,

    /// Number of layouts to generate per algorithm (default: 100)
    #[arg(short = 'n', long)]
    pub count: Option<usize>,

    /// Wall-clock budget per algorithm in seconds (overrides --count)
    #[arg(short = 't', long)]
    pub time: Option<u64>,

    /// Comma-separated algorithms: hill,sa,ils,lahc,d2,vnd (default: all)
    #[arg(short = 'a', long)]
    pub algorithms: Option<String>,

    /// Characters to pin in place on the basis layout.
    ///
    /// A hyphen can be a key to pin, so a leading dash here is a value, not a flag.
    #[arg(short = 'p', long, allow_hyphen_values(true))]
    pub pins: Option<String>,

    /// Print a markdown table instead of the TUI
    #[arg(long)]
    pub headless: bool,
}

impl Bench {
    /// Parse a token vector with no program-name element, which is how the parity
    /// table drives it. Clap reads argv[0] as the program name, so a dummy is
    /// prepended. Production uses the derived `Bench::parse()`.
    #[cfg(test)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> Result<Self, clap::Error> {
        Self::try_parse_from(std::iter::once(std::ffi::OsString::from("bench")).chain(args))
    }
}

#[cfg(test)]
mod flags_parity {
    //! CLI surface parity table for the bench binary (T042, contracts/cli-surface.md,
    //! FR-015). Bench IS a plain argv CLI (`Bench::parse()` at `src/main.rs`), unlike
    //! the REPL, which parses shlexed interactive lines.
    //!
    //! 59 rows, each recording the measured outcome of the parser that is in the build.
    //! The baseline was captured from the parser this one replaces; every row where the
    //! outcome differs carries a `// DIVERGENT:` comment naming the old outcome, so a
    //! change of surface is a reviewed decision rather than an accident. Accepts compare the
    //! `Debug` rendering of `Bench`; rejects compare an error CLASS (parser wording is
    //! not part of the surface). `UnknownFlag`/`UnknownCommand` collapse into one
    //! `Unknown` class because clap reports both under a single error kind.

    use std::ffi::OsString;

    use super::Bench;

    #[derive(Debug, PartialEq, Eq)]
    enum Class {
        /// The line asked for help; clap delivers the help text as an error too.
        Help,
        MissingRequired,
        Duplicate,
        /// An unrecognised token: clap reports unknown flags and unknown positionals
        /// under one kind, so the old two-class split collapses here.
        Unknown,
        MissingValue,
        BadValue,
    }

    #[derive(Debug)]
    enum Exp {
        /// Accepted: the expected `Debug` rendering of `Bench`.
        Ok(&'static str),
        /// Rejected: the expected error class.
        Err(Class),
    }

    fn classify(err: &clap::Error) -> Class {
        use clap::error::ErrorKind as K;
        match err.kind() {
            K::DisplayHelp | K::DisplayVersion => Class::Help,
            K::InvalidValue => Class::MissingValue,
            K::ValueValidation => Class::BadValue,
            K::ArgumentConflict => Class::Duplicate,
            K::MissingRequiredArgument => Class::MissingRequired,
            K::TooManyValues | K::TooFewValues | K::WrongNumberOfValues => Class::BadValue,
            K::UnknownArgument | K::InvalidSubcommand => Class::Unknown,
            other => panic!("unmapped clap error kind {:?}: {err}", other),
        }
    }

    /// The seam: drives the real parse path for the table.
    fn parse(tokens: &[&str]) -> Result<String, Class> {
        let args = tokens.iter().map(OsString::from).collect::<Vec<_>>();
        match Bench::from_vec(args) {
            Ok(flags) => Ok(format!("{flags:?}")),
            Err(e) => Err(classify(&e)),
        }
    }

    fn vectors() -> Vec<(Vec<&'static str>, Exp)> {
        vec![
            (
                vec![],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["-n", "5"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(5), time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["-n"], Exp::Err(Class::MissingValue)),
            (vec!["-n", "abc"], Exp::Err(Class::BadValue)),
            (
                vec!["--headless"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: None, headless: true }",
                ),
            ),
            (vec!["--headless", "--headless"], Exp::Err(Class::Duplicate)),
            (
                vec!["-l", "en", "-b", "colman"],
                Exp::Ok(
                    "Bench { language: Some(\"en\"), basis: Some(\"colman\"), count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["-a", "hill,sa"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: Some(\"hill,sa\"), pins: None, headless: false }",
                ),
            ),
            (
                vec!["-p", "abc"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: Some(\"abc\"), headless: false }",
                ),
            ),
            (
                vec!["-t", "30"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: Some(30), algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["--nope"], Exp::Err(Class::Unknown)),
            (vec!["extra"], Exp::Err(Class::Unknown)),
            (
                vec!["-n", "5", "-t", "3"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(5), time: Some(3), algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["-lt", "3"], Exp::Err(Class::Unknown)),
            (vec!["-n", "5", "-n", "6"], Exp::Err(Class::Duplicate)),
            (
                vec!["-p", "-a"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: Some(\"-a\"), headless: false }",
                ),
            ),
            (vec!["-a", "-p"], Exp::Err(Class::MissingValue)), // DIVERGENT: the old parser accepted "Bench { language: None, basis: None, count: None, time: None, algorithms: Some(\"-p\"), pins: None, headless: false }", clap rejects it as Class::MissingValue
            (
                vec!["-n5"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(5), time: None, algorithms: None, pins: None, headless: false }",
                ),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (vec!["-l", "-b"], Exp::Err(Class::MissingValue)), // DIVERGENT: the old parser accepted "Bench { language: Some(\"-b\"), basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }", clap rejects it as Class::MissingValue
            (
                vec!["--language=de"],
                Exp::Ok(
                    "Bench { language: Some(\"de\"), basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (vec!["-p", "ab", "-p", "cd"], Exp::Err(Class::Duplicate)),
            (vec!["-"], Exp::Err(Class::Unknown)),
            (
                vec!["--"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["-n", "-5"], Exp::Err(Class::Unknown)), // DIVERGENT: the old parser rejected it as Class::BadValue, clap rejects it as Class::Unknown
            (vec!["-t", "-1"], Exp::Err(Class::Unknown)), // DIVERGENT: the old parser rejected it as Class::BadValue, clap rejects it as Class::Unknown
            (vec!["-n", "-1"], Exp::Err(Class::Unknown)), // DIVERGENT: the old parser rejected it as Class::BadValue, clap rejects it as Class::Unknown
            (vec!["-n", "1.5"], Exp::Err(Class::BadValue)),
            (vec!["-t", "1.5"], Exp::Err(Class::BadValue)),
            (vec!["-n", ""], Exp::Err(Class::BadValue)),
            (vec![""], Exp::Err(Class::Unknown)),
            (vec!["-h"], Exp::Err(Class::Help)),
            (vec!["--help"], Exp::Err(Class::Help)),
            (vec!["help"], Exp::Err(Class::Unknown)), // DIVERGENT: the old parser rejected it as Class::Help, clap rejects it as Class::Unknown
            (vec!["--nope", "-n", "1"], Exp::Err(Class::Unknown)),
            (vec!["-n", "5", "extra"], Exp::Err(Class::Unknown)),
            (vec!["-nn"], Exp::Err(Class::BadValue)), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap rejects it as Class::BadValue
            (
                vec!["-n=5"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(5), time: None, algorithms: None, pins: None, headless: false }",
                ),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (vec!["-l", "en", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["-b"], Exp::Err(Class::MissingValue)),
            (vec!["-l"], Exp::Err(Class::MissingValue)),
            (vec!["-a"], Exp::Err(Class::MissingValue)),
            (vec!["-t"], Exp::Err(Class::MissingValue)),
            (vec!["--time"], Exp::Err(Class::MissingValue)),
            (vec!["-t", "abc"], Exp::Err(Class::BadValue)),
            (vec!["-l", "en", "-l", "de"], Exp::Err(Class::Duplicate)),
            (vec!["-a", "hill", "-a", "sa"], Exp::Err(Class::Duplicate)),
            (
                vec!["--headless", "--headless", "--headless"],
                Exp::Err(Class::Duplicate),
            ),
            (
                vec!["-t", "0"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: Some(0), algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["-n", "0"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(0), time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["-p", ""],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: Some(\"\"), headless: false }",
                ),
            ),
            (
                vec!["-l", ""],
                Exp::Ok(
                    "Bench { language: Some(\"\"), basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["--headless", "-n", "3"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(3), time: None, algorithms: None, pins: None, headless: true }",
                ),
            ),
            (
                vec!["-n", "3", "--headless"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(3), time: None, algorithms: None, pins: None, headless: true }",
                ),
            ),
            (
                vec!["--language", "de"],
                Exp::Ok(
                    "Bench { language: Some(\"de\"), basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["--basis", "x"],
                Exp::Ok(
                    "Bench { language: None, basis: Some(\"x\"), count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["--count", "7"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(7), time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (
                vec!["--algorithms", "hill"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: Some(\"hill\"), pins: None, headless: false }",
                ),
            ),
            (
                vec!["--pins", "ab"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: Some(\"ab\"), headless: false }",
                ),
            ),
            (
                vec![
                    "-n",
                    "1",
                    "-t",
                    "2",
                    "-a",
                    "sa",
                    "-p",
                    "q",
                    "-l",
                    "en",
                    "-b",
                    "colman",
                    "--headless",
                ],
                Exp::Ok(
                    "Bench { language: Some(\"en\"), basis: Some(\"colman\"), count: Some(1), time: Some(2), algorithms: Some(\"sa\"), pins: Some(\"q\"), headless: true }",
                ),
            ),
        ]
    }

    #[test]
    fn cli_surface_parity() {
        for (tokens, expected) in vectors() {
            let actual = parse(&tokens);
            match (&expected, &actual) {
                (Exp::Ok(debug), Ok(got)) => {
                    assert_eq!(got, debug, "parsed value differs for {tokens:?}");
                }
                (Exp::Err(class), Err(got)) => {
                    assert_eq!(got, class, "error class differs for {tokens:?}");
                }
                _ => {
                    panic!("outcome differs for {tokens:?}: expected {expected:?}, got {actual:?}")
                }
            }
        }
    }

    /// Every flag in the grammar is represented so a port that drops one fails here.
    #[test]
    fn every_flag_is_covered() {
        let covered: String = vectors()
            .iter()
            .map(|(t, _)| t.join(" "))
            .collect::<Vec<_>>()
            .join(" | ");
        for flag in [
            "-l",
            "--language",
            "-b",
            "--basis",
            "-n",
            "--count",
            "-t",
            "--time",
            "-a",
            "--algorithms",
            "-p",
            "--pins",
            "--headless",
        ] {
            let mentioned = covered
                .split(" | ")
                .any(|line| line.split(' ').any(|token| token == flag));
            assert!(mentioned, "{flag} has no parity vector");
        }
    }
}
