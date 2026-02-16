mod error;
mod exec;
mod format;
mod parse;

use std::fs;
use std::process;

use clap::Parser;

use error::RunError;

#[derive(Parser)]
#[command(
    name = "zhttp",
    version,
    about = "Execute HTTP requests from .http files"
)]
struct Cli {
    /// Path to the .http file
    file: String,
    /// Line number within the request block
    #[arg(long)]
    line: usize,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), RunError> {
    let content =
        fs::read_to_string(&cli.file).map_err(|e| RunError::FileRead(cli.file.clone(), e))?;

    let block = parse::find_request_block(&content, cli.line)?;
    let request = parse::parse_request(&block)?;
    exec::execute_request(&request)
}
