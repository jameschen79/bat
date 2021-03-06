use std::fs::File;
use std::io::{self, BufRead, BufReader};

use ansi_term::Colour::Green;

use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxDefinition;

use app::Config;
use assets::HighlightingAssets;
use diff::get_git_diff;
use errors::*;
use line_range::LineRange;
use output::OutputType;
use printer::Printer;

pub fn list_languages(assets: &HighlightingAssets, term_width: usize) {
    let mut languages = assets
        .syntax_set
        .syntaxes()
        .iter()
        .filter(|syntax| !syntax.hidden && !syntax.file_extensions.is_empty())
        .collect::<Vec<_>>();
    languages.sort_by_key(|lang| lang.name.to_uppercase());

    let longest = languages
        .iter()
        .map(|syntax| syntax.name.len())
        .max()
        .unwrap_or(32); // Fallback width if they have no language definitions.

    let comma_separator = ", ";
    let separator = " ";
    // Line-wrapping for the possible file extension overflow.
    let desired_width = term_width - longest - separator.len();

    for lang in languages {
        print!("{:width$}{}", lang.name, separator, width = longest);

        // Number of characters on this line so far, wrap before `desired_width`
        let mut num_chars = 0;

        let mut extension = lang.file_extensions.iter().peekable();
        while let Some(word) = extension.next() {
            // If we can't fit this word in, then create a line break and align it in.
            let new_chars = word.len() + comma_separator.len();
            if num_chars + new_chars >= desired_width {
                num_chars = 0;
                print!("\n{:width$}{}", "", separator, width = longest);
            }

            num_chars += new_chars;
            print!("{}", Green.paint(&word[..]));
            if extension.peek().is_some() {
                print!("{}", comma_separator);
            }
        }
        println!();
    }
}

pub fn list_themes(assets: &HighlightingAssets) {
    let themes = &assets.theme_set.themes;
    for (theme, _) in themes.iter() {
        println!("{}", theme);
    }
}

pub fn print_files(assets: &HighlightingAssets, config: &Config) -> Result<bool> {
    let theme = assets.get_theme(&config.theme);

    let mut output_type = OutputType::from_mode(config.paging_mode);
    let handle = output_type.handle()?;
    let mut printer = Printer::new(handle, &config, &theme);
    let mut no_errors: bool = true;

    for file in &config.files {
        printer.ansi_prefix_sgr.clear();
        printer.line_changes = file.and_then(|filename| get_git_diff(filename));
        let syntax = assets.get_syntax(config.language, *file);

        let result = print_file(theme, &syntax, &mut printer, *file);

        if let Err(error) = result {
            handle_error(&error);
            no_errors = false;
        }
    }

    Ok(no_errors)
}

fn print_file(
    theme: &Theme,
    syntax: &SyntaxDefinition,
    printer: &mut Printer,
    filename: Option<&str>,
) -> Result<()> {
    let stdin = io::stdin(); // TODO: this variable is not always needed
    {
        let reader: Box<BufRead> = match filename {
            None => Box::new(stdin.lock()),
            Some(filename) => Box::new(BufReader::new(File::open(filename)?)),
        };

        let highlighter = HighlightLines::new(syntax, theme);

        printer.print_header(filename)?;
        print_file_ranges(printer, reader, highlighter, &printer.config.line_range)?;
        printer.print_footer()?;
    }
    Ok(())
}

fn print_file_ranges<'a>(
    printer: &mut Printer,
    mut reader: Box<BufRead + 'a>,
    mut highlighter: HighlightLines,
    line_ranges: &Option<LineRange>,
) -> Result<()> {
    let mut buffer = Vec::new();

    while reader.read_until(b'\n', &mut buffer)? > 0 {
        {
            let line = String::from_utf8_lossy(&buffer);
            let regions = highlighter.highlight(line.as_ref());

            match line_ranges {
                &Some(ref range) => {
                    if printer.line_number + 1 < range.lower {
                        // skip line
                        printer.line_number += 1;
                    } else if printer.line_number >= range.upper {
                        // no more lines in range
                        break;
                    } else {
                        printer.print_line(&regions)?;
                    }
                }
                &None => {
                    printer.print_line(&regions)?;
                }
            }
        }
        buffer.clear();
    }
    Ok(())
}
