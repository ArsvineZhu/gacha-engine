use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    let config = cbindgen::Config::from_file("cbindgen.toml").expect("cbindgen 配置");
    let header = cbindgen::Builder::new()
        .with_config(config)
        .with_crate(".")
        .generate()
        .expect("生成 C 头文件");
    let mut bytes = Vec::new();
    header.write(&mut bytes);
    let text = String::from_utf8(bytes).expect("头文件应为 UTF-8");
    let path = Path::new("include/gacha_engine.h");
    fs::create_dir_all(path.parent().expect("头文件目录")).expect("创建头文件目录");
    if fs::read_to_string(path).ok().as_deref() != Some(text.as_str()) {
        fs::write(path, text).expect("写入头文件");
    }
}
