use super::*;

pub(crate) fn command_pull(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
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
    let state = load_optional_state(option_value(args, "--state"))?;

    let engine = gacha_sdk::Engine::load(pack_path)?;
    let mut snapshot = engine.session(seed).snapshot();
    snapshot.state = state;
    let mut session = engine.restore_session(snapshot)?;
    let mut results = Vec::<Value>::new();
    let mut context = ScopeContext::default();

    for index in 0..count {
        context.pull = (index + 1).to_string();
        context.batch = (index + 1).to_string();
        session.set_context(context.clone());
        for branch in session.pull(banner, action)? {
            let item = &branch.item;
            results.push(json!({
                "action_index": index + 1,
                "item": item.id.clone(),
                "display_name": item.name.clone(),
                "kind": item.kind,
                "native_rarity": item.native_rarity,
                "upstream_id": item.upstream_id,
                "rarity": item.rarity,
                "branch_probability": branch.probability.to_string(),
                "guarantees": branch.guarantees,
                "events": branch.events
            }));
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "seed": seed,
            "results": results,
            "state": session.state()
        }))?
    );
    Ok(())
}

pub(crate) fn command_transitions(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let pack_path = required(args, 0, "transitions requires PACK BANNER ACTION")?;
    let banner = required(args, 1, "transitions requires PACK BANNER ACTION")?;
    let action = required(args, 2, "transitions requires PACK BANNER ACTION")?;
    let state = load_optional_state(option_value(args, "--state"))?;
    let engine = gacha_sdk::Engine::load(pack_path)?;
    let context = ScopeContext::default();
    let branches = engine.transitions(banner, action, &state, &context)?;

    let output = branches
        .iter()
        .map(|branch| {
            let item = &branch.item;
            json!({
                "probability": branch.probability.to_string(),
                "probability_f64": branch.probability.to_f64(),
                "item": item.id.clone(),
                "display_name": item.name.clone(),
                "kind": item.kind,
                "native_rarity": item.native_rarity,
                "rarity": item.rarity,
                "guarantees": branch.guarantees.clone(),
                "events": branch.events.clone(),
                "next_state": branch.next_state.clone()
            })
        })
        .collect::<Vec<_>>();
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
