use std::env;
use std::fs;
use std::process::ExitCode;

use gacha_engine::{Source, compile};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("用法: gacha check 文件...");
        return ExitCode::from(2);
    };
    if command != "check" {
        eprintln!("用法: gacha check 文件...");
        return ExitCode::from(2);
    }
    let paths: Vec<String> = args.collect();
    if paths.is_empty() {
        eprintln!("用法: gacha check 文件...");
        return ExitCode::from(2);
    }
    let mut texts = Vec::new();
    for path in &paths {
        match fs::read_to_string(path) {
            Ok(text) => texts.push(text),
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::from(1);
            }
        }
    }
    let sources: Vec<Source<'_>> = paths
        .iter()
        .zip(texts.iter())
        .map(|(path, text)| Source {
            name: path.rsplit(['/', '\\']).next().unwrap_or(path),
            text,
        })
        .collect();
    match compile(&sources) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "{}:{}:{}: {} {}",
                error.file, error.line, error.column, error.code, error.actual
            );
            ExitCode::from(1)
        }
    }
}
