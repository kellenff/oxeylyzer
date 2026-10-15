use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// The REPL's argument grammar, one interactive line at a time.
///
/// `try_parse_from` is the only parse path: it RETURNS an error instead of
/// printing-and-exiting, because the prompt loop must keep running after a bad
/// line (contracts/cli-surface.md, FR-015's exit-behavior rule). Same reason
/// `subcommand_required` is set: a bare line is an error to report, not a
/// help screen to display-and-exit on.
#[derive(Debug, Parser)]
#[command(subcommand_required(true))]
pub struct Repl {
    #[command(subcommand)]
    pub subcommand: ReplCmd,
}

impl Repl {
    /// Parse one shlexed input line. Clap reads argv[0] as the program name and
    /// the REPL line has no such token, so a dummy is prepended — the token stream
    /// `parse_flags` feeds stays exactly what it was under the old parser.
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> Result<Self, clap::Error> {
        Self::try_parse_from(std::iter::once(std::ffi::OsString::from("repl")).chain(args))
    }
}

#[derive(Debug, Subcommand)]
pub enum ReplCmd {
    /// Analyze a layout. You can also specify a number to analyze a previously generated layout.
    #[command(visible_alias("a"), visible_alias("view"), visible_alias("layout"))]
    Analyze(Analyze),
    /// Compare two layouts
    #[command(visible_alias("c"), visible_alias("comp"), visible_alias("cmp"))]
    Compare(Compare),
    /// Swaps a number of keys on a certain layout. Cycles between one key and the next:
    /// swap sturdy ab -> swaps ab
    /// swap sturdy abc -> swaps a -> b -> c
    /// swap sturdy ab abc swap -> swaps a -> b and then a -> b -> c
    Swap(Swap),
    /// Rank all layouts for the currently specified language. A higher score is better.
    #[command(visible_alias("list"))]
    Rank(Rank),
    /// Improves the given layout. Optionally, you can provide a list of pinned characters
    /// to keep in place during optimization.
    #[command(
        visible_alias("gen"),
        visible_alias("g"),
        visible_alias("improve"),
        visible_alias("i"),
        visible_alias("optimize")
    )]
    Generate(Generate),
    /// Saves a layout by index or name. Optionally, you can provide a save name as a second argument.
    #[command(visible_alias("s"))]
    Save(Save),
    /// Removes a saved layout by name, deleting it from disk.
    #[command(visible_alias("rm"))]
    Remove(Remove),
    /// Shows the top n sfbs on a layout. 10 by default.
    Sfbs(Sfbs),
    /// Shows the top n fspeed pairs on a layout. 10 by default.
    Fspeed(Fspeed),
    /// Shows the top n scissors on a layout. 10 by default.
    Scissors(Scissors),
    /// Shows the top n lsbs on a layout. 10 by default.
    Lsbs(Lsbs),
    /// Shows the top n pinky-ring bigrams on a layout. 10 by default.
    #[command(visible_alias("pinky-ring"), visible_alias("pr"))]
    Pinkyring(Pinkyring),
    /// Shows the top n stretches on a layout. 10 by default.
    Stretches(Stretches),
    /// Set a language to be used for analysis. Tries to load corpus when not present.
    #[command(visible_alias("l"), visible_alias("lang"))]
    Language(Language),
    /// Include layouts stored under a different language
    Include(Include),
    /// Lists all currently available languages.
    #[command(visible_alias("langs"))]
    Languages(Languages),
    /// Loads a corpus for a certain language.
    Load(Load),
    /// Gives information about a certain ngram. for 2 letter ones, skipgram info will be provided as well.
    #[command(visible_alias("n"), visible_alias("occ"), visible_alias("freq"))]
    Ngram(Ngram),
    /// Show current weights, or modify them. Pass weight flags to update values (e.g. `config
    /// --sfbs 1.0`). Pass `--edit` to open the config file in $EDITOR.
    #[command(visible_alias("cfg"), visible_alias("weights"))]
    Config(Config),
    /// Refreshes the config, default characters for the analyzer. Will retain previously generated layouts.
    #[command(visible_alias("r"))]
    Reload(Reload),
    /// Quits the analyzer.
    #[command(visible_alias("q"), visible_alias("exit"))]
    Quit(Quit),
}

#[derive(Debug, Args)]
pub struct Analyze {
    pub name_or_nr: String,
}

#[derive(Debug, Args)]
pub struct Compare {
    pub name1: String,
    pub name2: String,
}

#[derive(Debug, Args)]
pub struct Swap {
    pub name: String,
    pub swaps: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Rank {}

#[derive(Debug, Args)]
pub struct Generate {
    pub name: String,
    pub count: Option<usize>,
    /// Sets pinned characters on the layout to optimize, `-p abc` pins `abc`.
    ///
    /// A hyphen can be a key to pin, so a leading dash here is a value, not a flag.
    #[arg(short = 'p', long, allow_hyphen_values(true))]
    pub pins: Option<String>,
}

#[derive(Debug, Args)]
pub struct Save {
    pub name_or_nr: String,
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct Remove {
    pub name: String,
    /// Skip the confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// The six top-n viewers share one shape. Separate types rather than aliases:
/// the parity table compares the `Debug` rendering, which names the struct.
macro_rules! top_n {
    ($name:ident) => {
        #[derive(Debug, Args)]
        pub struct $name {
            pub name: String,
            #[arg(short = 'c', long)]
            pub count: Option<usize>,
        }
    };
}

top_n!(Sfbs);
top_n!(Fspeed);
top_n!(Scissors);
top_n!(Lsbs);
top_n!(Pinkyring);
top_n!(Stretches);

#[derive(Debug, Args)]
pub struct Language {
    pub language: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct Include {
    pub languages: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub struct Languages {}

#[derive(Debug, Args)]
pub struct Load {
    /// The name of the corpus. This value will be used when looking for a corpus config,
    /// which is the configuration used to clean the corpus.
    pub language: String,
    /// If set, processes the corpus as-is without cleaning it.
    #[arg(short = 'r', long)]
    pub raw: bool,
    /// If set, processes all corpora found in ./static/text where the folder name is the
    /// language name.
    #[arg(short = 'a', long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct Ngram {
    pub ngram: String,
}

#[derive(Debug, Args)]
pub struct Config {
    /// Open the config file in $EDITOR (or $VISUAL, vi, notepad as fallbacks).
    #[arg(long)]
    pub edit: bool,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub sfbs: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub sfs: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub inrolls: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub outrolls: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub onehands: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub alternates: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub alternates_sfs: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub redirects: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub redirects_sfs: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub bad_redirects: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub bad_redirects_sfs: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub stretches: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub pinky_ring: Option<f64>,
    #[arg(long, allow_hyphen_values(true))]
    #[arg(long)]
    pub lateral_penalty: Option<f64>,
    #[arg(long)]
    pub trigram_precision: Option<usize>,
    #[arg(long)]
    pub max_cores: Option<usize>,
}

#[derive(Debug, Args)]
pub struct Reload {}

#[derive(Debug, Args)]
pub struct Quit {}
