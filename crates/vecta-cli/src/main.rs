mod report;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};
use vecta_core::*;

#[derive(Parser)]
#[command(version, about = "See and gate the compiled impact of a change")]
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
    /// Compare binaries, snapshots, or one of each.
    Diff(Comparison),
    /// Exit 1 for budget violations; exit 2 for input or configuration errors.
    Gate {
        #[command(flatten)]
        comparison: Comparison,
        #[command(flatten)]
        budgets: GateArgs,
    },
}
#[derive(Args)]
struct Comparison {
    base: PathBuf,
    head: PathBuf,
    #[arg(long, value_enum, default_value = "terminal")]
    format: OutputFormat,
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Number of largest symbol changes in human-readable reports.
    #[arg(long, default_value_t = 20)]
    top: usize,
    /// Emit a bounded #z= share URL for this viewer (no fragment).
    #[arg(long)]
    viewer_url: Option<String>,
}
#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Terminal,
    Markdown,
    Json,
}
#[derive(Args)]
struct GateArgs {
    /// Maximum file growth in bytes (inclusive).
    #[arg(long)]
    max_growth: Option<u64>,
    #[arg(long)]
    max_growth_percent: Option<f64>,
    /// Repeat for per-section logical-size budgets, e.g. --section-budget .text=4096.
    #[arg(long = "section-budget", value_parser = parse_section_budget)]
    section_budgets: Vec<(String, u64)>,
    #[arg(long)]
    forbid_new_libraries: bool,
    #[arg(long)]
    max_glibc: Option<Version>,
    #[arg(long)]
    max_glibcxx: Option<Version>,
    #[arg(long)]
    forbid_abi_bump: bool,
}
fn parse_section_budget(s: &str) -> std::result::Result<(String, u64), String> {
    let (name, bytes) = s.rsplit_once('=').ok_or("expected SECTION=BYTES")?;
    if name.is_empty() {
        return Err("section name cannot be empty".into());
    }
    Ok((
        name.into(),
        bytes
            .parse()
            .map_err(|_| "budget must be a nonnegative byte count")?,
    ))
}
fn write_output(path: Option<&PathBuf>, output: &str) -> Result<()> {
    if let Some(path) = path {
        std::fs::write(path, output).with_context(|| format!("writing {}", path.display()))?;
    } else {
        io::stdout().lock().write_all(output.as_bytes())?;
    }
    Ok(())
}
fn run() -> Result<u8> {
    match Cli::parse().command {
        Command::Inspect { binary, output } => {
            let snapshot = inspect_path(binary)?;
            for warning in &snapshot.warnings {
                eprintln!("warning: {warning}");
            }
            write_output(
                output.as_ref(),
                &(serde_json::to_string_pretty(&snapshot)? + "\n"),
            )?;
            Ok(0)
        }
        Command::Diff(comparison) => run_comparison(comparison, None),
        Command::Gate {
            comparison,
            budgets,
        } => {
            let section_growth: BTreeMap<_, _> = budgets.section_budgets.into_iter().collect();
            let budgets = Budgets {
                max_growth: budgets.max_growth,
                max_growth_percent: budgets.max_growth_percent,
                section_growth,
                forbid_new_libraries: budgets.forbid_new_libraries,
                max_glibc: budgets.max_glibc,
                max_glibcxx: budgets.max_glibcxx,
                forbid_abi_bump: budgets.forbid_abi_bump,
            };
            anyhow::ensure!(
                budgets.max_growth.is_some()
                    || budgets.max_growth_percent.is_some()
                    || !budgets.section_growth.is_empty()
                    || budgets.forbid_new_libraries
                    || budgets.max_glibc.is_some()
                    || budgets.max_glibcxx.is_some()
                    || budgets.forbid_abi_bump,
                "gate requires at least one budget or policy flag"
            );
            run_comparison(comparison, Some(budgets))
        }
    }
}
fn run_comparison(args: Comparison, budgets: Option<Budgets>) -> Result<u8> {
    let diff = compare(&load(&args.base)?, &load(&args.head)?)?;
    let gate = budgets.as_ref().map(|b| evaluate(&diff, b)).transpose()?;
    let share = args
        .viewer_url
        .as_deref()
        .map(|url| share_url(&diff, url))
        .transpose()?;
    let output = match args.format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(
                &serde_json::json!({"diff":diff,"gate":gate,"share_url":share}),
            )? + "\n"
        }
        OutputFormat::Markdown => {
            report::markdown(&diff, gate.as_ref(), args.top, share.as_deref())
        }
        OutputFormat::Terminal => report::terminal(
            &diff,
            gate.as_ref(),
            args.top,
            share.as_deref(),
            args.output.is_none(),
        ),
    };
    write_output(args.output.as_ref(), &output)?;
    Ok(if gate.as_ref().is_some_and(|g| !g.passed) {
        1
    } else {
        0
    })
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("vecta: {error:#}");
            ExitCode::from(2)
        }
    }
}
