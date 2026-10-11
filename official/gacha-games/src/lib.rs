//! 官方游戏资料库。规则、内容、实际卡池名单和来源共同维护于 `pools/`。
//! 依赖方向为资料库 → 通用 SDK；引擎不依赖任何游戏资料。
use gacha_sdk::{Engine, EngineError, GamePack, Result};
use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/documents.rs"));

/// 可组装的卡池标识；包含已注明假设的版本预览和机制参考池。
pub fn banners() -> impl Iterator<Item = &'static str> {
    DOCUMENTS.iter().map(|(id, _)| *id).filter(|id| {
        id.rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("banner."))
    })
}

/// 原始资料查询；内容目录包含不能抽取的角色形态，实际名单由卡池清单限定。
pub fn document(id: &str) -> Result<&'static str> {
    DOCUMENTS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, text)| *text)
        .ok_or_else(|| EngineError::new(format!("未找到官方资料：{id}")))
}

/// 使用通用解析器组装内嵌资料，可指定另一份机制配置。
pub fn pack(banner: &str, profile: Option<&str>) -> Result<GamePack> {
    if !banners().any(|id| id == banner) {
        return Err(EngineError::new(format!("未找到官方卡池：{banner}")));
    }
    gacha_pack::resolve_pack(Path::new(banner), profile.map(Path::new), |path| {
        let id = path.to_string_lossy().replace('\\', "/");
        document(&id).map(str::to_owned)
    })
}

pub fn load(banner: &str) -> Result<Engine> {
    load_with_profile(banner, None)
}

pub fn load_with_profile(banner: &str, profile: Option<&str>) -> Result<Engine> {
    Engine::from_pack(pack(banner, profile)?)
}
