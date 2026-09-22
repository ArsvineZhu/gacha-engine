use gacha_analysis::probability_of_item_within;
use gacha_core::{ScopeContext, SplitMix64, StateStore, enumerate_transitions, sample_transition};
use gacha_pack::{compile_pack, load_pack};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

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
        "pull" => command_pull(&args[1..]),
        "transitions" => command_transitions(&args[1..]),
        "probability" => command_probability(&args[1..]),
        other => Err(format!("unknown command: {other}").into()),
    }
}

fn command_validate(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let path = required(args, 0, "validate requires PACK.json")?;
    let pack = load_pack(path)?;
    let game = compile_pack(&pack)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "valid": true,
            "game": game.id,
            "version": game.version,
            "states": game.states.len(),
            "items": game.items.len(),
            "pools": game.pools.len(),
            "distributions": game.distributions.len(),
            "selectors": game.selectors.len(),
            "banners": game.banners.len()
        }))?
    );
    Ok(())
}

fn command_inspect(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let path = required(args, 0, "inspect requires PACK.json")?;
    let pack = load_pack(path)?;
    let game = compile_pack(&pack)?;
    let banners = game
        .banners
        .iter()
        .map(|banner| {
            json!({
                "id": banner.id.clone(),
                "pity_group": banner.pity_group.clone(),
                "progress_group": banner.progress_group.clone(),
                "actions": banner.actions.iter().map(|a| a.id.clone()).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "game": game.id,
            "version": game.version,
            "banners": banners
        }))?
    );
    Ok(())
}

fn command_pull(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let pack_path = required(args, 0, "pull requires PACK BANNER ACTION")?;
    let banner = required(args, 1, "pull requires PACK BANNER ACTION")?;
    let action = required(args, 2, "pull requires PACK BANNER ACTION")?;
    let count = option_value(args, "--count")
        .map(|v| v.parse::<u32>())
        .transpose()?
        .unwrap_or(1);
    let seed = option_value(args, "--seed")
        .map(|v| v.parse::<u64>())
        .transpose()?
        .unwrap_or_else(default_seed);
    let mut state = load_optional_state(option_value(args, "--state"))?;

    let pack = load_pack(pack_path)?;
    let game = compile_pack(&pack)?;
    let mut rng = SplitMix64::new(seed);
    let mut results = Vec::<Value>::new();
    let mut context = ScopeContext::default();

    for index in 0..count {
        context.pull = index.to_string();
        let branch = sample_transition(&game, banner, action, &mut state, &context, &mut rng)?;
        let item = game.item(branch.outcome.item);
        results.push(json!({
            "item": item.id.clone(),
            "display_name": item.display_name.clone(),
            "rarity": branch.outcome.rarity,
            "branch_probability": branch.probability.to_string(),
            "guarantees": branch.guarantees,
            "events": branch.events
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "seed": seed,
            "results": results,
            "state": state
        }))?
    );
    Ok(())
}

fn command_transitions(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let pack_path = required(args, 0, "transitions requires PACK BANNER ACTION")?;
    let banner = required(args, 1, "transitions requires PACK BANNER ACTION")?;
    let action = required(args, 2, "transitions requires PACK BANNER ACTION")?;
    let state = load_optional_state(option_value(args, "--state"))?;
    let pack = load_pack(pack_path)?;
    let game = compile_pack(&pack)?;
    let context = ScopeContext::default();
    let branches = enumerate_transitions(&game, banner, action, &state, &context)?;

    let output = branches
        .iter()
        .map(|branch| {
            let item = game.item(branch.outcome.item);
            json!({
                "probability": branch.probability.to_string(),
                "probability_f64": branch.probability.to_f64(),
                "item": item.id.clone(),
                "display_name": item.display_name.clone(),
                "rarity": branch.outcome.rarity,
                "guarantees": branch.guarantees.clone(),
                "events": branch.events.clone(),
                "next_state": branch.next_state.clone()
            })
        })
        .collect::<Vec<_>>();
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn command_probability(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let pack_path = required(
        args,
        0,
        "probability requires PACK BANNER ACTION TARGET DRAWS",
    )?;
    let banner = required(
        args,
        1,
        "probability requires PACK BANNER ACTION TARGET DRAWS",
    )?;
    let action = required(
        args,
        2,
        "probability requires PACK BANNER ACTION TARGET DRAWS",
    )?;
    let target = required(
        args,
        3,
        "probability requires PACK BANNER ACTION TARGET DRAWS",
    )?;
    let draws = required(
        args,
        4,
        "probability requires PACK BANNER ACTION TARGET DRAWS",
    )?
    .parse::<u32>()?;
    let state = load_optional_state(option_value(args, "--state"))?;

    let pack = load_pack(pack_path)?;
    let game = compile_pack(&pack)?;
    let result = probability_of_item_within(
        &game,
        banner,
        action,
        &state,
        &ScopeContext::default(),
        target,
        draws,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "target": target,
            "within_draws": draws,
            "probability": result.probability,
            "surviving_states": result.surviving_states,
            "note": "multi-draw mass accumulation uses f64; each one-step transition is generated from exact rational rules"
        }))?
    );
    Ok(())
}

fn required<'a>(
    args: &'a [String],
    index: usize,
    message: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| message.to_string().into())
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

fn load_optional_state(path: Option<&str>) -> Result<StateStore, Box<dyn std::error::Error>> {
    match path {
        Some(path) => {
            let text = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&text)?)
        }
        None => Ok(StateStore::default()),
    }
}

fn default_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0xC0FFEE)
}

fn print_help() {
    println!(
        r#"gacha-engine CLI

USAGE:
  gacha validate PACK.json
  gacha inspect PACK.json
  gacha pull PACK.json BANNER ACTION [--count N] [--seed N] [--state state.json]
  gacha transitions PACK.json BANNER ACTION [--state state.json]
  gacha probability PACK.json BANNER ACTION TARGET_ITEM DRAWS [--state state.json]

The CLI intentionally has no network or database behavior. Rule packs are untrusted data,
not executable scripts."#
    );
}
