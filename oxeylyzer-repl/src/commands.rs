use sexp::Sexp;

use crate::repl::*;

fn add_outside_parens(expr: &str) -> String {
    match expr.starts_with('(') {
        true => expr.to_string(),
        _ => format!("({})", expr),
    }
}

impl std::ops::Add for ReplStatus {
    type Output = ReplStatus;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Quit, _) | (_, Self::Quit) => Self::Quit,
            _ => Self::Continue,
        }
    }
}

impl Repl {
    fn walk_sexp(&mut self, value: &Sexp, depth: usize) -> Result<(String, String, ReplStatus)> {
        match value {
            Sexp::Atom(atom) => Ok((atom.to_string(), atom.to_string(), ReplStatus::Continue)),
            Sexp::List(sexps) => {
                let (command_parts, original_parts, statuses) = sexps
                    .iter()
                    .map(|v| match v {
                        Sexp::Atom(_) => self.walk_sexp(v, depth + 1),
                        Sexp::List(_) => {
                            let (result, original, status) = self.walk_sexp(v, depth + 1)?;

                            Ok((md5_hash(&result), format!("({original})"), status))
                        }
                    })
                    .collect::<Result<(Vec<_>, Vec<_>, Vec<_>)>>()?;

                let subcommand = command_parts.join(" ");
                let original = original_parts.join(" ");
                let repl_status = statuses
                    .into_iter()
                    .reduce(|acc, e| acc + e)
                    .unwrap_or(ReplStatus::Continue);

                let repl_status = repl_status + self.parse_flags(&subcommand, &original, depth)?;

                Ok((subcommand, original, repl_status))
            }
        }
    }

    pub fn walk(&mut self, value: &Sexp) -> Result<ReplStatus> {
        self.walk_sexp(value, 0).map(|(_, _, status)| status)
    }

    pub fn respond(&mut self, line: &str) -> Result<ReplStatus> {
        self.clear_temp_layouts();

        let expr = add_outside_parens(line);

        let sexp = sexp::parse(&expr).map_err(|e| ReplError::SexpError {
            err_message: e.message.to_string(),
            line: line.to_string(),
            idx: e.index,
        })?;

        self.walk(&sexp)
    }

    fn parse_flags(&mut self, command: &str, original: &str, depth: usize) -> Result<ReplStatus> {
        use crate::flags::{Repl, ReplCmd::*};

        let args = shlex::split(command)
            .ok_or(ReplError::ShlexError)?
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();

        let flags = Repl::from_vec(args)?;

        let response = match flags.subcommand {
            Analyze(a) => self.analyze(&a.name_or_nr),
            Compare(c) => self.compare(&c.name1, &c.name2),
            Swap(s) => self.swap(&s.name, &s.swaps),
            Rank(_) => self.rank(),
            Generate(i) => self.generate(&i.name, i.count, i.pins),
            Save(s) => self.save(&s.name_or_nr, s.name),
            Sfbs(s) => self.sfbs(&s.name, s.count),
            Remove(r) => self.remove(&r.name, r.yes),
            Fspeed(s) => self.fspeed(&s.name, s.count),
            Stretches(s) => self.stretches(&s.name, s.count),
            Scissors(s) => self.scissors(&s.name, s.count),
            Lsbs(s) => self.lsbs(&s.name, s.count),
            Pinkyring(s) => self.pinky_ring(&s.name, s.count),
            Language(l) => self.language(l.language),
            Include(l) => self.include(&l.languages),
            Languages(_) => self.languages(),
            Load(l) => self.load(l.language, l.all, l.raw),
            Ngram(n) => self.ngram(&n.ngram),
            Config(c) => self.config(c),
            Reload(_) => self.reload(),
            Quit(_) => return Ok(ReplStatus::Quit),
        };

        let response = match response {
            Ok(response) => response,
            Err(ReplError::CommandDoesNotReturnLayout(_)) => {
                return Err(ReplError::CommandDoesNotReturnLayout(original.to_string()));
            }
            Err(e) => return Err(e),
        };

        use ReplResponse as RR;

        match (response, depth) {
            (RR::NoLayout { printable }, 0) => println!("{printable}"),
            (RR::SingleLayout { layout, printable }, 0) => {
                let mut layout = *layout;
                layout.name = original.to_string();
                self.insert_temp_layout(command, layout);
                println!("{printable}");
            }
            (RR::MultipleLayouts { layouts, printable }, 0) => {
                let mut layout = layouts[0].clone();
                layout.name = format!("{original} (idx 0)");
                self.insert_temp_layout(command, layout);
                println!("{printable}");
            }
            (RR::SingleLayout { layout, .. }, _) => {
                let mut layout = *layout;
                layout.name = original.to_string();
                self.insert_temp_layout(command, layout)
            }
            (RR::MultipleLayouts { layouts, .. }, _) => {
                let mut layout = layouts[0].clone();
                layout.name = format!("{original} (idx 0)");
                self.insert_temp_layout(command, layout)
            }
            _ => {}
        }

        Ok(ReplStatus::Continue)
    }
}

#[cfg(test)]
mod flags_parity {
    //! CLI surface parity table (T041, contracts/cli-surface.md, FR-015).
    //!
    //! Every row records the measured accept/reject outcome, captured by driving
    //! `Repl::from_vec` with the exact token stream `parse_flags` feeds it (no program
    //! name, shlexed line). 167 rows; the baseline was captured from the parser this
    //! one replaces, and every row where the outcome differs carries a `// DIVERGENT:`
    //! comment naming the old outcome, so a surface change is a reviewed decision
    //! rather than an accident. A row that fails for any other reason is a real
    //! surface regression, not a test-maintenance task.
    //!
    //! Accepts compare the `Debug` rendering of `ReplCmd`, which pins variant names,
    //! field names, arity, `Option` vs `Vec` emptiness, repeated-argument order, and
    //! numeric parses. Rejects compare an error CLASS (contracts/cli-surface.md: the
    //! class must match; verbatim parser wording is not part of the surface).
    //! `UnknownFlag`/`UnknownCommand` collapse into one `Unknown` class because clap
    //! reports both under a single error kind.

    use std::ffi::OsString;

    use crate::flags::Repl;

    #[derive(Debug, PartialEq, Eq)]
    enum Class {
        /// The line asked for help; the parser returns it as an error.
        Help,
        MissingSubcommand,
        MissingRequired,
        Duplicate,
        /// An unrecognised token. clap reports unknown flags and unknown extra
        /// positionals under one kind, so the old two-class split collapses here.
        Unknown,
        MissingValue,
        BadValue,
    }

    #[derive(Debug)]
    enum Exp {
        /// Accepted: the expected `Debug` rendering of `ReplCmd`.
        Ok(&'static str),
        /// Rejected: the expected error class.
        Err(Class),
    }

    fn classify(err: &clap::Error) -> Class {
        use clap::error::ErrorKind as K;
        match err.kind() {
            K::DisplayHelp | K::DisplayVersion => Class::Help,
            // An empty line: clap shows help on missing subcommand, which is this
            // table's MissingSubcommand class, not a help request.
            K::MissingSubcommand | K::DisplayHelpOnMissingArgumentOrSubcommand => {
                Class::MissingSubcommand
            }
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
        match Repl::from_vec(args) {
            Ok(flags) => Ok(format!("{:?}", flags.subcommand)),
            Err(e) => Err(classify(&e)),
        }
    }

    fn vectors() -> Vec<(Vec<&'static str>, Exp)> {
        vec![
            (vec![], Exp::Err(Class::MissingSubcommand)),
            (
                vec!["analyze", "sturdy"],
                Exp::Ok("Analyze(Analyze { name_or_nr: \"sturdy\" })"),
            ),
            (
                vec!["a", "3"],
                Exp::Ok("Analyze(Analyze { name_or_nr: \"3\" })"),
            ),
            (vec!["analyze"], Exp::Err(Class::MissingRequired)),
            (vec!["analyze", "a", "b"], Exp::Err(Class::Unknown)),
            (vec!["analyze", "--nope"], Exp::Err(Class::Unknown)),
            (
                vec!["compare", "x", "y"],
                Exp::Ok("Compare(Compare { name1: \"x\", name2: \"y\" })"),
            ),
            (
                vec!["c", "x", "y"],
                Exp::Ok("Compare(Compare { name1: \"x\", name2: \"y\" })"),
            ),
            (
                vec!["comp", "x", "y"],
                Exp::Ok("Compare(Compare { name1: \"x\", name2: \"y\" })"),
            ),
            (vec!["cmp", "x"], Exp::Err(Class::MissingRequired)),
            (vec!["compare", "x"], Exp::Err(Class::MissingRequired)),
            (
                vec!["swap", "sturdy", "ab"],
                Exp::Ok("Swap(Swap { name: \"sturdy\", swaps: [\"ab\"] })"),
            ),
            (
                vec!["swap", "sturdy", "abc"],
                Exp::Ok("Swap(Swap { name: \"sturdy\", swaps: [\"abc\"] })"),
            ),
            (
                vec!["swap", "sturdy", "ab", "abc", "swap"],
                Exp::Ok("Swap(Swap { name: \"sturdy\", swaps: [\"ab\", \"abc\", \"swap\"] })"),
            ),
            (
                vec!["swap", "sturdy"],
                Exp::Ok("Swap(Swap { name: \"sturdy\", swaps: [] })"),
            ),
            (vec!["swap"], Exp::Err(Class::MissingRequired)),
            (vec!["rank"], Exp::Ok("Rank(Rank)")),
            (vec!["list"], Exp::Ok("Rank(Rank)")),
            (vec!["rank", "extra"], Exp::Err(Class::Unknown)),
            (
                vec!["generate", "sturdy"],
                Exp::Ok("Generate(Generate { name: \"sturdy\", count: None, pins: None })"),
            ),
            (
                vec!["gen", "sturdy", "10"],
                Exp::Ok("Generate(Generate { name: \"sturdy\", count: Some(10), pins: None })"),
            ),
            (
                vec!["g", "sturdy", "10", "-p", "abc"],
                Exp::Ok(
                    "Generate(Generate { name: \"sturdy\", count: Some(10), pins: Some(\"abc\") })",
                ),
            ),
            (
                vec!["improve", "sturdy", "-p", "abc"],
                Exp::Ok(
                    "Generate(Generate { name: \"sturdy\", count: None, pins: Some(\"abc\") })",
                ),
            ),
            (
                vec!["i", "sturdy", "0"],
                Exp::Ok("Generate(Generate { name: \"sturdy\", count: Some(0), pins: None })"),
            ),
            (vec!["optimize", "sturdy", "-1"], Exp::Err(Class::Unknown)),
            (vec!["generate", "sturdy", "abc"], Exp::Err(Class::BadValue)),
            (
                vec!["generate", "sturdy", "-p"],
                Exp::Err(Class::MissingValue),
            ),
            (
                vec!["generate", "sturdy", "10", "20"],
                Exp::Err(Class::Unknown),
            ),
            (
                vec!["generate", "sturdy", "10", "-p", "ab", "-p", "cd"],
                Exp::Err(Class::Duplicate),
            ),
            (
                vec!["save", "x"],
                Exp::Ok("Save(Save { name_or_nr: \"x\", name: None })"),
            ),
            (
                vec!["s", "x", "newname"],
                Exp::Ok("Save(Save { name_or_nr: \"x\", name: Some(\"newname\") })"),
            ),
            (vec!["save", "x", "new", "extra"], Exp::Err(Class::Unknown)),
            (
                vec!["remove", "x"],
                Exp::Ok("Remove(Remove { name: \"x\", yes: false })"),
            ),
            (
                vec!["rm", "x", "-y"],
                Exp::Ok("Remove(Remove { name: \"x\", yes: true })"),
            ),
            (
                vec!["remove", "x", "--yes"],
                Exp::Ok("Remove(Remove { name: \"x\", yes: true })"),
            ),
            (vec!["remove", "x", "-y", "-y"], Exp::Err(Class::Duplicate)),
            (vec!["remove"], Exp::Err(Class::MissingRequired)),
            (
                vec!["sfbs", "x"],
                Exp::Ok("Sfbs(Sfbs { name: \"x\", count: None })"),
            ),
            (
                vec!["sfbs", "x", "-c", "5"],
                Exp::Ok("Sfbs(Sfbs { name: \"x\", count: Some(5) })"),
            ),
            (
                vec!["fspeed", "x", "--count", "5"],
                Exp::Ok("Fspeed(Fspeed { name: \"x\", count: Some(5) })"),
            ),
            (vec!["scissors", "x", "-c"], Exp::Err(Class::MissingValue)),
            (vec!["lsbs", "x", "-c", "abc"], Exp::Err(Class::BadValue)),
            (
                vec!["pinkyring", "x", "-c", "2"],
                Exp::Ok("Pinkyring(Pinkyring { name: \"x\", count: Some(2) })"),
            ),
            (
                vec!["pinky-ring", "x"],
                Exp::Ok("Pinkyring(Pinkyring { name: \"x\", count: None })"),
            ),
            (
                vec!["pr", "x"],
                Exp::Ok("Pinkyring(Pinkyring { name: \"x\", count: None })"),
            ),
            (
                vec!["stretches", "x", "-c", "1"],
                Exp::Ok("Stretches(Stretches { name: \"x\", count: Some(1) })"),
            ),
            (
                vec!["language"],
                Exp::Ok("Language(Language { language: None })"),
            ),
            (
                vec!["l", "english"],
                Exp::Ok("Language(Language { language: Some(\"english\") })"),
            ),
            (
                vec!["lang", "static/text/en"],
                Exp::Ok("Language(Language { language: Some(\"static/text/en\") })"),
            ),
            (
                vec!["include", "en", "de"],
                Exp::Ok("Include(Include { languages: [\"en\", \"de\"] })"),
            ),
            (
                vec!["include"],
                Exp::Ok("Include(Include { languages: [] })"),
            ),
            (vec!["languages"], Exp::Ok("Languages(Languages)")),
            (vec!["langs"], Exp::Ok("Languages(Languages)")),
            (
                vec!["load", "english"],
                Exp::Ok("Load(Load { language: \"english\", raw: false, all: false })"),
            ),
            (
                vec!["load", "english", "-r"],
                Exp::Ok("Load(Load { language: \"english\", raw: true, all: false })"),
            ),
            (
                vec!["load", "english", "-a"],
                Exp::Ok("Load(Load { language: \"english\", raw: false, all: true })"),
            ),
            (
                vec!["load", "english", "--raw", "--all"],
                Exp::Ok("Load(Load { language: \"english\", raw: true, all: true })"),
            ),
            (vec!["load"], Exp::Err(Class::MissingRequired)),
            (
                vec!["ngram", "th"],
                Exp::Ok("Ngram(Ngram { ngram: \"th\" })"),
            ),
            (vec!["n", "th"], Exp::Ok("Ngram(Ngram { ngram: \"th\" })")),
            (vec!["occ", "th"], Exp::Ok("Ngram(Ngram { ngram: \"th\" })")),
            (
                vec!["freq", "th"],
                Exp::Ok("Ngram(Ngram { ngram: \"th\" })"),
            ),
            (vec!["ngram"], Exp::Err(Class::MissingRequired)),
            (
                vec!["config"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["cfg", "--edit"],
                Exp::Ok(
                    "Config(Config { edit: true, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["weights", "--sfbs", "1.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(1.5), sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (vec!["config", "--sfbs", "abc"], Exp::Err(Class::BadValue)),
            (
                vec!["config", "--trigram-precision", "4"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: Some(4), max_cores: None })",
                ),
            ),
            (
                vec!["config", "--max-cores", "0"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: Some(0) })",
                ),
            ),
            (
                vec!["config", "--pinky-ring", "2.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: Some(2.5), lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--sfbs", "1.0", "--sfs", "2.0"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(1.0), sfs: Some(2.0), inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (vec!["config", "--sfbs"], Exp::Err(Class::MissingValue)),
            (
                vec!["config", "--edit", "--edit"],
                Exp::Err(Class::Duplicate),
            ),
            (vec!["config", "--nope", "1"], Exp::Err(Class::Unknown)),
            (vec!["reload"], Exp::Ok("Reload(Reload)")),
            (vec!["r"], Exp::Ok("Reload(Reload)")),
            (vec!["quit"], Exp::Ok("Quit(Quit)")),
            (vec!["q"], Exp::Ok("Quit(Quit)")),
            (vec!["exit"], Exp::Ok("Quit(Quit)")),
            (vec!["quit", "extra"], Exp::Err(Class::Unknown)),
            (vec!["badcmd"], Exp::Err(Class::Unknown)),
            (vec!["--help"], Exp::Err(Class::Help)),
            (vec!["help"], Exp::Err(Class::Help)),
            (
                vec!["analyze", "sturdy", "extra", "--nope"],
                Exp::Err(Class::Unknown),
            ),
            (
                vec!["config", "--sfbs=1.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(1.5), sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (
                vec!["sfbs", "x", "-c=5"],
                Exp::Ok("Sfbs(Sfbs { name: \"x\", count: Some(5) })"),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (
                vec!["generate", "x", "-p", "--count=5"],
                Exp::Ok(
                    "Generate(Generate { name: \"x\", count: None, pins: Some(\"--count=5\") })",
                ),
            ),
            (
                vec!["generate", "x", "-p", "-abc"],
                Exp::Ok("Generate(Generate { name: \"x\", count: None, pins: Some(\"-abc\") })"),
            ),
            (
                vec!["generate", "x", "-p", "a=b"],
                Exp::Ok("Generate(Generate { name: \"x\", count: None, pins: Some(\"a=b\") })"),
            ),
            (
                vec!["generate", "x", "-p", "-c"],
                Exp::Ok("Generate(Generate { name: \"x\", count: None, pins: Some(\"-c\") })"),
            ),
            (
                vec!["generate", "x", "--pins=ab"],
                Exp::Ok("Generate(Generate { name: \"x\", count: None, pins: Some(\"ab\") })"),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (
                vec!["generate", "x", "--pins"],
                Exp::Err(Class::MissingValue),
            ),
            (vec!["swap", "x", "-y"], Exp::Err(Class::Unknown)),
            (vec!["swap", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["load", "english", "-raw"], Exp::Err(Class::Unknown)),
            (vec!["analyze", "-x"], Exp::Err(Class::Unknown)),
            (vec!["rank", "--help"], Exp::Err(Class::Help)),
            (vec!["quit", "-y"], Exp::Err(Class::Unknown)),
            (vec!["help", "analyze"], Exp::Err(Class::Help)),
            (
                vec!["language", "-"],
                Exp::Ok("Language(Language { language: Some(\"-\") })"),
            ), // DIVERGENT: the old parser rejected it as Class::UnknownFlag, clap accepts it
            (vec!["-"], Exp::Err(Class::Unknown)),
            (vec![""], Exp::Err(Class::Unknown)),
            (vec!["", "x"], Exp::Err(Class::Unknown)),
            (
                vec!["analyze", ""],
                Exp::Ok("Analyze(Analyze { name_or_nr: \"\" })"),
            ),
            (vec!["analyze", "-h"], Exp::Err(Class::Help)),
            (vec!["analyze", "--help"], Exp::Err(Class::Help)),
            (vec!["analyze", "--yes"], Exp::Err(Class::Unknown)),
            (vec!["config", "-h"], Exp::Err(Class::Help)),
            (vec!["generate", "x", "-5"], Exp::Err(Class::Unknown)),
            (
                vec!["generate", "x", "-p", "-5"],
                Exp::Ok("Generate(Generate { name: \"x\", count: None, pins: Some(\"-5\") })"),
            ),
            (vec!["sfbs", "x", "-c", "-1"], Exp::Err(Class::Unknown)), // DIVERGENT: the old parser rejected it as Class::BadValue, clap rejects it as Class::Unknown
            (
                vec!["config", "--sfbs", "-1.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(-1.5), sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--sfbs", "1e3"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(1000.0), sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--sfbs", "inf"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(inf), sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--sfbs", "nan"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: Some(NaN), sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--max-cores", "1.5"],
                Exp::Err(Class::BadValue),
            ),
            (
                vec!["generate", "--", "x"],
                Exp::Ok("Generate(Generate { name: \"x\", count: None, pins: None })"),
            ),
            (
                vec!["analyze", "--", "x"],
                Exp::Ok("Analyze(Analyze { name_or_nr: \"x\" })"),
            ),
            (
                vec!["swap", "x", "ab", "--", "cd"],
                Exp::Ok("Swap(Swap { name: \"x\", swaps: [\"ab\", \"cd\"] })"),
            ),
            (
                vec!["a", "b"],
                Exp::Ok("Analyze(Analyze { name_or_nr: \"b\" })"),
            ),
            (vec!["swap", "--headless", "x"], Exp::Err(Class::Unknown)),
            (vec!["quit", "quit"], Exp::Err(Class::Unknown)),
            (vec!["save"], Exp::Err(Class::MissingRequired)),
            (vec!["sfbs"], Exp::Err(Class::MissingRequired)),
            (vec!["fspeed"], Exp::Err(Class::MissingRequired)),
            (vec!["scissors"], Exp::Err(Class::MissingRequired)),
            (vec!["lsbs"], Exp::Err(Class::MissingRequired)),
            (vec!["pinkyring"], Exp::Err(Class::MissingRequired)),
            (vec!["stretches"], Exp::Err(Class::MissingRequired)),
            (vec!["generate"], Exp::Err(Class::MissingRequired)),
            (vec!["compare"], Exp::Err(Class::MissingRequired)),
            (
                vec!["sfbs", "x", "-c", "5", "-c", "6"],
                Exp::Err(Class::Duplicate),
            ),
            (vec!["fspeed", "x", "-c", "abc"], Exp::Err(Class::BadValue)),
            (
                vec!["scissors", "x", "-c", "abc"],
                Exp::Err(Class::BadValue),
            ),
            (vec!["lsbs", "x", "-c"], Exp::Err(Class::MissingValue)),
            (
                vec!["pinkyring", "x", "-c", "abc"],
                Exp::Err(Class::BadValue),
            ),
            (
                vec!["stretches", "x", "-c", "abc"],
                Exp::Err(Class::BadValue),
            ),
            (
                vec!["stretches", "x", "--count"],
                Exp::Err(Class::MissingValue),
            ),
            (vec!["fspeed", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["scissors", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["lsbs", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["stretches", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["pr", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["save", "x", "--nope"], Exp::Err(Class::Unknown)),
            (
                vec!["swap", "x", "ab", "ab"],
                Exp::Ok("Swap(Swap { name: \"x\", swaps: [\"ab\", \"ab\"] })"),
            ),
            (
                vec!["include", "en", "en"],
                Exp::Ok("Include(Include { languages: [\"en\", \"en\"] })"),
            ),
            (vec!["include", "x", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["language", "en", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["load", "english", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["ngram", "th", "--nope"], Exp::Err(Class::Unknown)),
            (vec!["languages", "extra"], Exp::Err(Class::Unknown)),
            (vec!["reload", "extra"], Exp::Err(Class::Unknown)),
            (
                vec!["remove", "x", "-y", "--nope"],
                Exp::Err(Class::Unknown),
            ),
            (vec!["analyze", "3", "extra"], Exp::Err(Class::Unknown)),
            (
                vec!["config", "--sfs", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: Some(0.5), inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--inrolls", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: Some(0.5), outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--outrolls", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: Some(0.5), onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--onehands", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: Some(0.5), alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--alternates", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: Some(0.5), alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--alternates-sfs", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: Some(0.5), redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--redirects", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: Some(0.5), redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--redirects-sfs", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: Some(0.5), bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--bad-redirects", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: Some(0.5), bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--bad-redirects-sfs", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: Some(0.5), stretches: None, pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--stretches", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: Some(0.5), pinky_ring: None, lateral_penalty: None, trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--lateral-penalty", "0.5"],
                Exp::Ok(
                    "Config(Config { edit: false, sfbs: None, sfs: None, inrolls: None, outrolls: None, onehands: None, alternates: None, alternates_sfs: None, redirects: None, redirects_sfs: None, bad_redirects: None, bad_redirects_sfs: None, stretches: None, pinky_ring: None, lateral_penalty: Some(0.5), trigram_precision: None, max_cores: None })",
                ),
            ),
            (
                vec!["config", "--sfbs", "1", "--sfbs", "2"],
                Exp::Err(Class::Duplicate),
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

    /// Every subcommand and alias in the grammar is represented, so an alias
    /// dropped by a port fails here rather than silently changing the surface.
    #[test]
    fn every_command_and_alias_is_covered() {
        let covered: Vec<&str> = vectors()
            .iter()
            .filter(|(t, _)| !t.is_empty())
            .map(|(t, _)| t[0])
            .collect();
        for cmd in [
            "analyze",
            "a",
            "compare",
            "c",
            "comp",
            "cmp",
            "swap",
            "rank",
            "list",
            "generate",
            "gen",
            "g",
            "improve",
            "i",
            "optimize",
            "save",
            "s",
            "remove",
            "rm",
            "sfbs",
            "fspeed",
            "scissors",
            "lsbs",
            "pinkyring",
            "pinky-ring",
            "pr",
            "stretches",
            "language",
            "l",
            "lang",
            "include",
            "languages",
            "langs",
            "load",
            "ngram",
            "n",
            "occ",
            "freq",
            "config",
            "cfg",
            "weights",
            "reload",
            "r",
            "quit",
            "q",
            "exit",
        ] {
            assert!(
                covered.iter().any(|c| *c == cmd),
                "{cmd} has no parity vector"
            );
        }
    }

    /// Every short/long flag in the grammar appears in at least one vector, so a port
    /// that drops one (the `config` weight options are the risk) fails here.
    #[test]
    fn every_flag_is_covered() {
        let tokens: Vec<&str> = vectors()
            .iter()
            .flat_map(|(t, _)| t.iter().copied())
            .collect();
        for flag in [
            "-p",
            "--pins",
            "-c",
            "--count",
            "-y",
            "--yes",
            "-r",
            "--raw",
            "-a",
            "--all",
            "--edit",
            "--sfbs",
            "--sfs",
            "--inrolls",
            "--outrolls",
            "--onehands",
            "--alternates",
            "--alternates-sfs",
            "--redirects",
            "--redirects-sfs",
            "--bad-redirects",
            "--bad-redirects-sfs",
            "--stretches",
            "--pinky-ring",
            "--lateral-penalty",
            "--trigram-precision",
            "--max-cores",
        ] {
            assert!(tokens.contains(&flag), "{flag} has no parity vector");
        }
    }

    /// FR-015 / contracts/cli-surface.md "exit-behavior": a rejection must RETURN an
    /// error, never exit. Asserting this function returns at all is the test — a
    /// `std::process::exit` inside the parse path would kill the harness instead.
    /// This is what pins `try_get_matches_from` over `get_matches_from` in T048.
    #[test]
    fn rejection_returns_instead_of_exiting() {
        for tokens in [
            vec!["badcmd"],
            vec!["analyze"],
            vec!["config", "--nope"],
            vec!["-"],
            vec![],
            // help requests are errors too, and must be returned, not printed-and-exited
            vec!["--help"],
            vec!["analyze", "--help"],
        ] {
            assert!(
                parse(&tokens).is_err(),
                "{tokens:?} should reject but parse() accepted"
            );
        }
    }
}
