// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate lazy_static;

extern crate ansi_term;
extern crate atty;
extern crate console;
extern crate directories;
extern crate git2;
extern crate syntect;

mod app;
mod assets;
mod decorations;
mod diff;
mod features;
mod line_range;
mod output;
mod printer;
mod style;
mod terminal;

use std::io;
use std::path::Path;
use std::process;

use app::App;
use assets::{clear_assets, config_dir, HighlightingAssets};
use features::{list_languages, list_themes, print_files};

mod errors {
    error_chain! {
        foreign_links {
            Clap(::clap::Error);
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }

    pub fn handle_error(error: &Error) {
        match error {
            &Error(ErrorKind::Io(ref io_error), _)
                if io_error.kind() == super::io::ErrorKind::BrokenPipe =>
            {
                super::process::exit(0);
            }
            _ => {
                use ansi_term::Colour::Red;
                eprintln!("{}: {}", Red.paint("[bat error]"), error);
            }
        };
    }
}

use errors::*;

fn run_cache_subcommand(matches: &clap::ArgMatches) -> Result<()> {
    if matches.is_present("init") {
        let source_dir = matches.value_of("source").map(Path::new);
        let target_dir = matches.value_of("target").map(Path::new);

        let blank = matches.is_present("blank");

        let assets = HighlightingAssets::from_files(source_dir, blank)?;
        assets.save(target_dir)?;
    } else if matches.is_present("clear") {
        clear_assets();
    } else if matches.is_present("config-dir") {
        println!("{}", config_dir());
    }

    Ok(())
}

/// Returns `Err(..)` upon fatal errors. Otherwise, returns `Some(true)` on full success and
/// `Some(false)` if any intermediate errors occurred (were printed).
fn run() -> Result<bool> {
    let app = App::new();

    match app.matches.subcommand() {
        ("cache", Some(cache_matches)) => {
            run_cache_subcommand(cache_matches)?;
            Ok(true)
        }
        _ => {
            let config = app.config()?;
            let assets = HighlightingAssets::new();

            if app.matches.is_present("list-languages") {
                list_languages(&assets, config.term_width);

                Ok(true)
            } else if app.matches.is_present("list-themes") {
                list_themes(&assets);

                Ok(true)
            } else {
                print_files(&assets, &config)
            }
        }
    }
}

fn main() {
    let result = run();

    match result {
        Err(error) => {
            handle_error(&error);
            process::exit(1);
        }
        Ok(false) => {
            process::exit(1);
        }
        Ok(true) => {
            process::exit(0);
        }
    }
}
