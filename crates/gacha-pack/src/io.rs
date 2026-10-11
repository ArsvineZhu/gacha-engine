use gacha_core::{EngineError, Result};
use gacha_schema::GamePack;
use std::{fs, path::Path};
const MAX_PACK_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn read_document(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|error| {
        EngineError::new(format!("failed to stat pack {}: {error}", path.display()))
    })?;
    if metadata.len() > MAX_PACK_BYTES {
        return Err(EngineError::new(format!(
            "pack {} is {} bytes, exceeding the {} byte limit",
            path.display(),
            metadata.len(),
            MAX_PACK_BYTES
        )));
    }
    let text = fs::read_to_string(path).map_err(|error| {
        EngineError::new(format!("failed to read pack {}: {error}", path.display()))
    })?;
    Ok(text)
}

pub fn parse_pack(text: &str) -> Result<GamePack> {
    if text.len() as u64 > MAX_PACK_BYTES {
        return Err(EngineError::new(format!(
            "pack text is {} bytes, exceeding the {} byte limit",
            text.len(),
            MAX_PACK_BYTES
        )));
    }
    serde_json::from_str(text)
        .map_err(|error| EngineError::new(format!("invalid pack JSON: {error}")))
}
