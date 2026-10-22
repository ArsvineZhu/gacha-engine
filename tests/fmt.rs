use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use gacha_engine::{Source, compile, format_source};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn positive() -> Vec<(&'static str, String)> {
    let base = root().join("spec/gacharule/positive");
    let order = [
        "endfield.gmr",
        "genshin.gmr",
        "weapon_ten.gmr",
        "sample.gml",
        "banners.gmb",
    ];
    order
        .into_iter()
        .map(|name| {
            let text = fs::read_to_string(base.join(name)).unwrap();
            (name, text)
        })
        .collect()
}

fn squash(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(trimmed);
    }
    out.push('\n');
    out
}

#[test]
fn positive_format_is_stable_and_compiles() {
    let files = positive();
    let mut formatted = Vec::new();
    for (name, text) in &files {
        let once = format_source(name, text).unwrap_or_else(|error| {
            panic!(
                "{name} 格式化失败 {}:{}:{} {} {}",
                error.file, error.line, error.column, error.code, error.actual
            )
        });
        let twice = format_source(name, &once).unwrap();
        assert_eq!(twice, once, "{name} 二次格式化");
        assert_eq!(once, *text, "{name} 应保持规范正例的布局");
        let squashed = format_source(name, &squash(text)).unwrap();
        assert_eq!(squashed, *text, "{name} 压平后应回到同一布局");
        formatted.push(((*name).to_string(), once));
    }
    let sources: Vec<Source<'_>> = formatted
        .iter()
        .map(|(name, text)| Source { name, text })
        .collect();
    compile(&sources).unwrap_or_else(|error| {
        panic!(
            "格式化后的正例不能编译 {}:{}:{} {}",
            error.file, error.line, error.column, error.code
        )
    });
}

#[test]
fn comments_round_trip_and_tabs_become_spaces() {
    let source = "\
# 头
rule demo:
\t# 内
\tpool: Pool[5r]
\tsel: Selector[5r] = from pool # 尾
\trates: Distribution:
\t\t5r: 1p -> sel
\tdraw: Action[1t..1t] = rates
";
    let once = format_source("demo.gmr", source).unwrap();
    let twice = format_source("demo.gmr", &once).unwrap();
    assert_eq!(twice, once);
    assert!(once.contains("# 头\n"));
    assert!(once.contains("    # 内\n"));
    assert!(once.contains("# 尾\n"));
    assert!(!once.contains('\t'));
    compile(&[Source {
        name: "demo.gmr",
        text: &once,
    }])
    .unwrap();
}

#[test]
fn check_json_and_fmt_cli() {
    let bin = env!("CARGO_BIN_EXE_gacha");
    let sample = root().join("spec/gacharule/positive/weapon_ten.gmr");
    let ok = Command::new(bin)
        .args(["check", "--format=json", sample.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert_eq!(
        String::from_utf8(ok.stdout).unwrap().trim(),
        "{\"diagnostics\":[]}"
    );

    let mut broken = Command::new(bin)
        .args([
            "check",
            "--format=json",
            "--stdin",
            "broken.gmr",
            "broken.gmr",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    broken.stdin.take().unwrap().write_all(b"rule =\n").unwrap();
    let broken = broken.wait_with_output().unwrap();
    assert_eq!(broken.status.code(), Some(1));
    let stdout = String::from_utf8(broken.stdout).unwrap();
    for field in [
        "\"code\"",
        "\"file\"",
        "\"line\"",
        "\"column\"",
        "\"expected\"",
        "\"actual\"",
    ] {
        assert!(stdout.contains(field), "{stdout}");
    }
    assert!(stdout.contains("\"file\":\"broken.gmr\""), "{stdout}");

    let text = fs::read_to_string(&sample).unwrap();
    let mut fmt = Command::new(bin)
        .args(["fmt", "--stdin", "weapon_ten.gmr"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    fmt.stdin
        .take()
        .unwrap()
        .write_all(squash(&text).as_bytes())
        .unwrap();
    let fmt = fmt.wait_with_output().unwrap();
    assert!(
        fmt.status.success(),
        "{}",
        String::from_utf8_lossy(&fmt.stderr)
    );
    assert_eq!(String::from_utf8(fmt.stdout).unwrap(), text);
}
