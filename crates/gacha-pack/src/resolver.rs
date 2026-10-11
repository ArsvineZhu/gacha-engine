//! 文件系统和内嵌资源共用的文档解析入口。
use crate::{compose_pack, io::read_document};
use gacha_core::{EngineError, Result};
use gacha_schema::{ContentCatalog, GamePack, PackManifest, RulePack, SourceDef};
use std::path::Path;

pub(crate) fn resolve_sources(
    sources: &mut Vec<SourceDef>,
    documents: &[String],
    parent: &Path,
    read: &mut impl FnMut(&Path) -> Result<String>,
) -> Result<()> {
    if documents.len() > 256 {
        return Err(EngineError::new("too many source documents"));
    }
    for document in documents {
        let definitions: Vec<SourceDef> = parse(&read(&parent.join(document))?)?;
        for source in definitions {
            if let Some(existing) = sources.iter().find(|s| s.id == source.id) {
                if existing != &source {
                    return Err(EngineError::new(format!(
                        "conflicting source {}",
                        source.id
                    )));
                }
            } else {
                sources.push(source);
            }
        }
    }
    Ok(())
}

fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > 16 * 1024 * 1024 {
        return Err(EngineError::new("document exceeds 16 MiB"));
    }
    serde_json::from_str(text).map_err(|error| EngineError::new(error.to_string()))
}

/// 引用路径相对于引用文档解析；读取器可以提供本地文件或内嵌资源。
pub fn resolve_pack(
    path: &Path,
    rule_override: Option<&Path>,
    mut read: impl FnMut(&Path) -> Result<String>,
) -> Result<GamePack> {
    let manifest: PackManifest = parse(&read(path)?)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let rule_path = rule_override
        .map(Path::to_path_buf)
        .unwrap_or_else(|| parent.join(&manifest.rules));
    let mut rules: RulePack = parse(&read(&rule_path)?)?;
    resolve_sources(
        &mut rules.sources,
        &rules.source_documents,
        rule_path.parent().unwrap_or_else(|| Path::new(".")),
        &mut read,
    )?;
    if manifest.catalogs.len() > 256 {
        return Err(EngineError::new("too many catalog files"));
    }
    let mut catalogs = Vec::new();
    for reference in &manifest.catalogs {
        let path = parent.join(reference);
        let mut catalog: ContentCatalog = parse(&read(&path)?)?;
        resolve_sources(
            &mut catalog.sources,
            &catalog.source_documents,
            path.parent().unwrap_or_else(|| Path::new(".")),
            &mut read,
        )?;
        catalogs.push(catalog);
    }
    compose_pack(&manifest, &rules, &catalogs)
}

pub fn load_pack_with_rules(path: impl AsRef<Path>, rules: impl AsRef<Path>) -> Result<GamePack> {
    resolve_pack(path.as_ref(), Some(rules.as_ref()), |p| read_document(p))
}
