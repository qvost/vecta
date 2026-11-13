use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};
#[derive(Parser)]
#[command(version, about = "Inspect compiled artifacts and runtime requirements")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Write a deterministic JSON binary snapshot.
    Inspect {
        binary: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Inspect { binary, output } => {
            let snapshot = vecta_core::inspect_path(binary)?;
            for warning in &snapshot.warnings {
                eprintln!("warning: {warning}");
            }
            let json = serde_json::to_string_pretty(&snapshot)? + "\n";
            if let Some(path) = output {
                std::fs::write(&path, json)
                    .with_context(|| format!("writing {}", path.display()))?;
            } else {
                io::stdout().lock().write_all(json.as_bytes())?;
            }
        }
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vecta: {error:#}");
            ExitCode::from(2)
        }
    }
}
