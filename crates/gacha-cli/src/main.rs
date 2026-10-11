use gacha_analysis::FirstHitQuery;
use gacha_sdk::{ScopeContext, StateStore};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

mod arguments;
mod commands;
use arguments::*;
use commands::*;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "validate" => command_validate(&args[1..]),
        "inspect" => command_inspect(&args[1..]),
        "catalog" => command_catalog(&args[1..]),
        "pull" => command_pull(&args[1..]),
        "transitions" => command_transitions(&args[1..]),
        "analyze" => command_analyze(&args[1..]),
        "probability" => command_probability(&args[1..]),
        "study" => command_study(&args[1..]),
        "trace" => command_trace(&args[1..]),
        "bundle" => command_bundle(&args[1..]),
        other => Err(format!("unknown command: {other}").into()),
    }
}
