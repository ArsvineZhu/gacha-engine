use super::*;

pub(crate) fn command_catalog(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let path = required(args, 0, "catalog requires CONTENT.json")?;
    let catalog = gacha_pack::load_catalog(path)?;
    println!("{}", serde_json::to_string_pretty(&catalog)?);
    Ok(())
}

pub(crate) fn command_validate(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    let info = engine.info();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"valid":true,"game":info.id,"version":info.version,
        "states":info.states,"items":info.items,"pools":info.pools,"distributions":info.distributions,
        "selectors":info.selectors,"banners":info.banners,"preparation":engine.preparation()}))?
    );
    Ok(())
}

pub(crate) fn command_inspect(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let engine = sdk_engine(args)?;
    let actions = engine.actions();
    let mut banners = std::collections::BTreeMap::<String, Value>::new();
    for action in actions {
        let entry = banners.entry(action.banner.clone()).or_insert_with(||json!({
            "id":action.banner,"pity_group":action.pity_group,"progress_group":action.progress_group,"actions":[]}));
        entry["actions"]
            .as_array_mut()
            .unwrap()
            .push(json!(action.action));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"game":engine.id(),"version":engine.version(),
        "banners":banners.into_values().collect::<Vec<_>>(),"sources":engine.sources(),
        "assumptions":engine.assumptions(),"fingerprint":engine.fingerprint(),"preparation":engine.preparation()}))?
    );
    Ok(())
}
