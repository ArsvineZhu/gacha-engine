use super::*;

pub(super) fn required<'a>(
    args: &'a [String],
    index: usize,
    message: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| message.to_string().into())
}

pub(super) fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == name)
        .map(|window| window[1].as_str())
}

pub(super) fn load_optional_state(
    path: Option<&str>,
) -> Result<StateStore, Box<dyn std::error::Error>> {
    match path {
        Some(path) => {
            let text = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&text)?)
        }
        None => Ok(StateStore::default()),
    }
}

pub(super) fn default_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0xC0FFEE)
}

pub(super) fn print_help() {
    println!(
        r#"gacha-engine CLI

用法：
  gacha validate PACK.json
  gacha inspect PACK.json
  gacha catalog CONTENT.json
  gacha pull PACK.json BANNER ACTION [--count N] [--seed N] [--state state.json]
  gacha transitions PACK.json BANNER ACTION [--state state.json]
  gacha analyze PACK.json QUERY.json [--copies N] [--state state.json]
  gacha probability PACK.json BANNER ACTION TARGET_ITEM DRAWS [--state state.json]
  gacha study PACK.json REQUEST.json [--trials N] [--seed N] [--csv output.csv] [--output report.json]
  gacha trace PACK.json BANNER ACTION [--count N] [--seed N] [--session snapshot.json] [--save-session snapshot.json]
  gacha bundle PACK.json OUTPUT.json [--rules RULES.json]

study 的 actions 表示完整动作次数，提供逐抽及逐动作 PMF/CDF、分位数、期望、模拟与对照。
trace 返回保底检查、候选概率、转移规则检查、状态变化和事件。会话可保存及精确续跑。
bundle 打包已组装的独立数据，接入方无需引用原目录；加载时仍执行规则校验。
study、trace、bundle 可通过 --rules 指定机制配置，目标由原清单自动绑定。
analyze 的 draws 保留底层抽数语义。所有规则均为声明式数据。
"#
    );
}
