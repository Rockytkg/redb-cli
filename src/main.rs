mod cli;
mod engine;
mod error;
mod parser;

use clap::Parser as ClapParser;

/// redb-cli — A SQL-like interactive shell for the redb embedded database.
#[derive(ClapParser)]
#[command(name = "redb-cli")]
#[command(version = "0.1.0")]
#[command(about = "A SQL-like CLI for the redb embedded key-value database")]
struct Cli {
    /// Path to a redb database file to open
    #[arg(value_name = "FILE")]
    file: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    let mut repl = cli::Repl::new();

    // If a file path was provided, open it on startup
    if let Some(ref path) = cli.file {
        if !repl.open_on_start(path) {
            std::process::exit(1);
        }
    }

    // Run the interactive REPL
    repl.run();
}
