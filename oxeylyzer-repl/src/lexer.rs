//! The token grammar clap does not share, so FR-015's surface survived the parser swap.
//!
//! clap's lexer accepts forms the REPL and bench have always rejected — attached values
//! (`-n5`, `-nn`, `-c=5`, `--pins=ab`) and a lone `-` where a value was wanted — and it
//! rejects hyphen-leading tokens the old parser took as option values (`-a -p`, `-n -5`).
//! FR-015 binds accept and reject, so `check` screens the stream against the frozen
//! grammar before clap sees it, reading clap's own arg metadata for which tokens take a
//! value rather than restating the grammar.
//!
//! The two halves have to meet: screening alone only tightens. Every value-taking option
//! is also declared `allow_hyphen_values`, which is what lets `-a -p`, `-l -b` and
//! `--sfbs -1.5` through the way they used to. What reaches clap is then exactly the
//! token stream the old parser was handed.

use std::ffi::OsString;

use clap::{Arg, Command, error::ErrorKind};

fn unexpected(token: &str) -> clap::Error {
    clap::Error::raw(
        ErrorKind::UnknownArgument,
        format!("unexpected argument '{token}' found\n"),
    )
}

fn help_text(cmd: &mut Command) -> clap::Error {
    clap::Error::raw(ErrorKind::DisplayHelp, cmd.render_help().to_string())
}

/// `-x` and `--name`: the flag shapes the old grammar looked up. Attached forms (`-n5`,
/// `-c=5`, `--pins=ab`, `-nn`) were never flags it recognised, and neither was `-`.
fn flag_name(token: &str) -> Option<(String, bool)> {
    let long = token.starts_with("--");
    let name = token.get(long as usize + 1..)?;
    if name.is_empty() || name.contains('=') || (!long && name.chars().count() != 1) {
        return None;
    }
    Some((name.to_string(), long))
}

fn collect_value_options(cmd: &Command, into: &mut Vec<(String, bool)>) {
    into.extend(
        cmd.get_arguments()
            .filter(|arg| arg.get_action().takes_values())
            .flat_map(|arg: &Arg| {
                arg.get_long()
                    .map(|long| (long.to_string(), true))
                    .into_iter()
                    .chain(arg.get_short().map(|short| (short.to_string(), false)))
            }),
    );
    for sub in cmd.get_subcommands() {
        collect_value_options(sub, into);
    }
}

/// Reject what the frozen grammar rejects, then leave the accepted stream to clap.
/// A value-taking option swallows the token after it, and that token is never screened:
/// `generate x -p --count=5` pins `--count=5`, because `-p` had already claimed it.
pub fn check<C: clap::CommandFactory>(args: &[OsString]) -> Result<(), clap::Error> {
    let mut root = C::command();
    let mut value_options = Vec::new();
    collect_value_options(&root, &mut value_options);
    let mut in_value_position = false;

    for raw in args {
        let token = raw.to_string_lossy();
        if in_value_position {
            in_value_position = false;
            continue;
        }
        if token == "--" {
            break; // end of flags; everything after this is a value to both grammars
        }
        let Some((name, long)) = flag_name(&token) else {
            if token.starts_with('-') {
                return Err(unexpected(&token)); // `-n5`, `-nn`, `--x=y`, a lone `-`
            }
            // `help` as a bare word: the old grammar printed help for it. A CLI with
            // subcommands gets that from clap's own `help` subcommand instead.
            if token == "help" && !root.has_subcommands() && root.get_positionals().next().is_none()
            {
                return Err(help_text(&mut root));
            }
            continue; // a positional value: clap's business
        };
        in_value_position = value_options.contains(&(name, long));
    }
    Ok(())
}
