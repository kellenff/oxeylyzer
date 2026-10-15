xflags::xflags! {
    /// Benchmark oxeylyzer search algorithms against each other.
    cmd bench {
        /// Language corpus to use (default: english)
        optional -l, --language language: String
        /// Basis layout name; defines the key set and geometry (default: sturdy, or the first found)
        optional -b, --basis basis: String
        /// Number of layouts to generate per algorithm (default: 100)
        optional -n, --count count: usize
        /// Wall-clock budget per algorithm in seconds (overrides --count)
        optional -t, --time time: u64
        /// Comma-separated algorithms: hill,sa,ils,lahc,d2,vnd (default: all)
        optional -a, --algorithms algorithms: String
        /// Characters to pin in place on the basis layout
        optional -p, --pins pins: String
        /// Print a markdown table instead of the TUI
        optional --headless
    }
}

#[cfg(test)]
mod probe {
    use super::Bench;
    #[test]
    fn dump_baseline() {
        for v in [
            vec![],
            vec!["-n", "5"],
            vec!["-n"],
            vec!["-n", "abc"],
            vec!["--headless"],
            vec!["--headless", "--headless"],
            vec!["-l", "en", "-b", "colman"],
            vec!["-a", "hill,sa"],
            vec!["-p", "abc"],
            vec!["-t", "30"],
            vec!["--nope"],
            vec!["extra"],
            vec!["-n", "5", "-t", "3"],
            vec!["-lt", "3"],
            vec!["-n", "5", "-n", "6"],
            vec!["-p", "-a"],
            vec!["-a", "-p"],
            vec!["-n5"],
            vec!["-l", "-b"],
            vec!["--language=de"],
            vec!["-p", "ab", "-p", "cd"],
            vec!["-"],
            vec!["--"],
            vec!["-n", "-5"],
            vec!["-t", "-1"],
            vec!["-n", "-1"],
            vec!["-n", "1.5"],
            vec!["-t", "1.5"],
            vec!["-n", ""],
            vec![""],
            vec!["-h"],
            vec!["--help"],
            vec!["help"],
            vec!["--nope", "-n", "1"],
            vec!["-n", "5", "extra"],
            vec!["-nn"],
            vec!["-n=5"],
            vec!["-l", "en", "--nope"],
            vec!["-b"],
            vec!["-l"],
            vec!["-a"],
            vec!["-t"],
            vec!["--time"],
            vec!["-t", "abc"],
            vec!["-l", "en", "-l", "de"],
            vec!["-a", "hill", "-a", "sa"],
            vec!["--headless", "--headless", "--headless"],
            vec!["-t", "0"],
            vec!["-n", "0"],
            vec!["-p", ""],
            vec!["-l", ""],
            vec!["--headless", "-n", "3"],
            vec!["-n", "3", "--headless"],
            vec!["--language", "de"],
            vec!["--basis", "x"],
            vec!["--count", "7"],
            vec!["--algorithms", "hill"],
            vec!["--pins", "ab"],
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
        ] {
            let args = v.iter().map(std::ffi::OsString::from).collect::<Vec<_>>();
            let out = match Bench::from_vec(args) {
                Ok(f) => format!("ACCEPT {f:?}"),
                Err(e) => format!("REJECT: {}", e.to_string().replace('\n', " | ")),
            };
            println!("{:?} => {}", v, out);
        }
    }
}

#[cfg(test)]
mod flags_parity {
    //! CLI surface parity table for the bench binary (T042, contracts/cli-surface.md,
    //! FR-015). Bench IS a plain argv CLI (`from_env_or_exit` at `src/main.rs`), unlike
    //! the REPL, which parses shlexed interactive lines.
    //!
    //! The 53 rows are machine-generated from the CURRENT xflags build's measured
    //! outcomes and MUST NOT be edited when the parser is swapped: only `parse()` below
    //! may change. Accepts compare the `Debug` rendering of `Bench`; rejects compare an
    //! error CLASS (parser wording is not part of the surface).

    use std::ffi::OsString;

    use super::Bench;

    #[derive(Debug, PartialEq, Eq)]
    enum Class {
        /// The line asked for help; xflags returns the help text as an error.
        Help,
        MissingRequired,
        Duplicate,
        UnknownCommand,
        UnknownFlag,
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

    fn classify(msg: &str) -> Class {
        if msg.starts_with("Usage:") || msg.contains("Print this message") {
            Class::Help
        } else if msg.contains("Flag specified more than once") {
            Class::Duplicate
        } else if msg.contains("Flag is required") {
            Class::MissingRequired
        } else if msg.contains("Unknown command") {
            Class::UnknownCommand
        } else if msg.contains("Unknown flag") {
            Class::UnknownFlag
        } else if msg.contains("expected a value for") {
            Class::MissingValue
        } else {
            Class::BadValue
        }
    }

    /// THE ONE STABLE SEAM. Today this is `Bench::from_vec` (xflags). After T049 it
    /// becomes clap's `try_get_matches_from` mapped onto the same `Bench` struct, and
    /// the table above stays byte-for-byte the same.
    fn parse(tokens: &[&str]) -> Result<String, Class> {
        let args = tokens.iter().map(OsString::from).collect::<Vec<_>>();
        match Bench::from_vec(args) {
            Ok(flags) => Ok(format!("{flags:?}")),
            Err(e) => Err(classify(&e.to_string())),
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
            (vec!["--nope"], Exp::Err(Class::UnknownFlag)),
            (vec!["extra"], Exp::Err(Class::UnknownCommand)),
            (
                vec!["-n", "5", "-t", "3"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: Some(5), time: Some(3), algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["-lt", "3"], Exp::Err(Class::UnknownFlag)),
            (vec!["-n", "5", "-n", "6"], Exp::Err(Class::Duplicate)),
            (
                vec!["-p", "-a"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: Some(\"-a\"), headless: false }",
                ),
            ),
            (
                vec!["-a", "-p"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: Some(\"-p\"), pins: None, headless: false }",
                ),
            ),
            (vec!["-n5"], Exp::Err(Class::UnknownFlag)),
            (
                vec!["-l", "-b"],
                Exp::Ok(
                    "Bench { language: Some(\"-b\"), basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["--language=de"], Exp::Err(Class::UnknownFlag)),
            (vec!["-p", "ab", "-p", "cd"], Exp::Err(Class::Duplicate)),
            (vec!["-"], Exp::Err(Class::UnknownFlag)),
            (
                vec!["--"],
                Exp::Ok(
                    "Bench { language: None, basis: None, count: None, time: None, algorithms: None, pins: None, headless: false }",
                ),
            ),
            (vec!["-n", "-5"], Exp::Err(Class::BadValue)),
            (vec!["-t", "-1"], Exp::Err(Class::BadValue)),
            (vec!["-n", "-1"], Exp::Err(Class::BadValue)),
            (vec!["-n", "1.5"], Exp::Err(Class::BadValue)),
            (vec!["-t", "1.5"], Exp::Err(Class::BadValue)),
            (vec!["-n", ""], Exp::Err(Class::BadValue)),
            (vec![""], Exp::Err(Class::UnknownCommand)),
            (vec!["-h"], Exp::Err(Class::Help)),
            (vec!["--help"], Exp::Err(Class::Help)),
            (vec!["help"], Exp::Err(Class::Help)),
            (vec!["--nope", "-n", "1"], Exp::Err(Class::UnknownFlag)),
            (vec!["-n", "5", "extra"], Exp::Err(Class::UnknownCommand)),
            (vec!["-nn"], Exp::Err(Class::UnknownFlag)),
            (vec!["-n=5"], Exp::Err(Class::UnknownFlag)),
            (vec!["-l", "en", "--nope"], Exp::Err(Class::UnknownFlag)),
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
