use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use corporate_nayose::tables::{companies, establishments};

#[derive(Parser)]
#[command(about = "Japanese corporate registry (法人番号) data pipeline")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the companies and establishments parquet tables from the raw CSVs
    BuildTables {
        /// Directory holding the raw registry CSVs
        #[arg(default_value = "data")]
        data_dir: PathBuf,
        /// Directory to write the parquet tables into
        #[arg(default_value = "data/parquet")]
        out_dir: PathBuf,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::BuildTables { data_dir, out_dir } => companies::build(
            &data_dir.join(companies::SOURCE),
            &out_dir.join(companies::OUTPUT),
        )
        .and_then(|_| {
            establishments::build(
                &data_dir.join(establishments::SOURCE),
                &out_dir.join(establishments::OUTPUT),
            )
        }),
    };
    if let Err(err) = result {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
