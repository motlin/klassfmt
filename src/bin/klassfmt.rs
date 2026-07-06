//! `klassfmt` command-line interface.
//!
//! Default (no mode flag): format each path and write the result to stdout
//! (or, for a single stdin stream, format stdin). The mode flags mirror
//! Prettier's conventions:
//!
//!   --check           report which files are not formatted; exit 1 if any.
//!   --write           format files in place.
//!   --list-different  list paths whose formatting differs; exit 1 if any.
//!   --stdin-filepath  read source from stdin, format, write to stdout.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "klassfmt", about = "A formatter for the Klass DSL")]
struct Cli {
    /// Check whether files are formatted; do not write. Exit non-zero if any differ.
    #[arg(long, conflicts_with_all = ["write", "list_different"])]
    check: bool,

    /// Format files in place.
    #[arg(long, conflicts_with_all = ["check", "list_different"])]
    write: bool,

    /// List files whose formatting differs. Exit non-zero if any differ.
    #[arg(long = "list-different", conflicts_with_all = ["check", "write"])]
    list_different: bool,

    /// Format stdin as if it were this file, writing the result to stdout.
    #[arg(long = "stdin-filepath", value_name = "PATH")]
    stdin_filepath: Option<PathBuf>,

    /// Maximum line width before wrapping.
    #[arg(long, default_value_t = klassfmt::DEFAULT_PRINT_WIDTH)]
    print_width: usize,

    /// Files to format.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.stdin_filepath.is_some() || cli.paths.is_empty() {
        return run_stdin(&cli);
    }
    run_paths(&cli)
}

fn run_stdin(cli: &Cli) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("klassfmt: failed to read stdin");
        return ExitCode::FAILURE;
    }
    match klassfmt::format_with_width(&input, cli.print_width) {
        Ok(formatted) => {
            print!("{formatted}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            let name = cli
                .stdin_filepath
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<stdin>".to_string());
            eprintln!("klassfmt: {name}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_paths(cli: &Cli) -> ExitCode {
    let mut any_different = false;
    let mut any_error = false;

    for path in &cli.paths {
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("klassfmt: {}: {e}", path.display());
                any_error = true;
                continue;
            }
        };

        let formatted = match klassfmt::format_with_width(&source, cli.print_width) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("klassfmt: {}: {e}", path.display());
                any_error = true;
                continue;
            }
        };

        let differs = formatted != source;
        if differs {
            any_different = true;
        }

        match Mode::from(cli) {
            Mode::Check => {
                if differs {
                    println!("{}", path.display());
                }
            }
            Mode::ListDifferent => {
                if differs {
                    println!("{}", path.display());
                }
            }
            Mode::Write => {
                if differs {
                    if let Err(e) = write_file(path, &formatted) {
                        eprintln!("klassfmt: {}: {e}", path.display());
                        any_error = true;
                    }
                }
            }
            Mode::Stdout => {
                print!("{formatted}");
                let _ = std::io::stdout().flush();
            }
        }
    }

    if any_error {
        return ExitCode::FAILURE;
    }
    // In check / list-different modes, a difference is itself a failure.
    if any_different && matches!(Mode::from(cli), Mode::Check | Mode::ListDifferent) {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn write_file(path: &Path, contents: &str) -> std::io::Result<()> {
    fs::write(path, contents)
}

enum Mode {
    Check,
    Write,
    ListDifferent,
    Stdout,
}

impl Mode {
    fn from(cli: &Cli) -> Mode {
        if cli.check {
            Mode::Check
        } else if cli.write {
            Mode::Write
        } else if cli.list_different {
            Mode::ListDifferent
        } else {
            Mode::Stdout
        }
    }
}
