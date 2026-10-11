use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=pools");
    let mut entries = Vec::new();
    for game in fs::read_dir("pools").expect("卡池目录") {
        let directory = game.expect("游戏目录").path();
        for file in fs::read_dir(&directory).expect("资料目录") {
            let path = file.expect("资料文件").path();
            if path.extension().and_then(|p| p.to_str()) != Some("json") {
                continue;
            }
            let key = path
                .strip_prefix("pools")
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let absolute = path.canonicalize().unwrap().to_string_lossy().into_owned();
            entries.push((key, absolute));
        }
    }
    entries.sort();
    let rows = entries
        .iter()
        .map(|(key, path)| format!("({key:?}, include_str!({path:?})),"))
        .collect::<String>();
    let generated = format!("pub static DOCUMENTS: &[(&str, &str)] = &[{rows}];");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("documents.rs"),
        generated,
    )
    .unwrap();
}
