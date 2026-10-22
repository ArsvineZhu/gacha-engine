#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use gacha_engine::{Diagnostic, Source, compile, format_source};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first() else {
        usage();
        return ExitCode::from(2);
    };
    match command.as_str() {
        "check" => cmd_check(&args[1..]),
        "fmt" => cmd_fmt(&args[1..]),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}

fn usage() {
    eprintln!(
        "用法:\n  gacha check [--format=text|json] [--stdin 名称] 文件...\n  gacha fmt [--stdin 名称] [文件]"
    );
}

fn cmd_check(args: &[String]) -> ExitCode {
    let flags = match parse_check_args(args) {
        Ok(flags) => flags,
        Err(message) => {
            eprintln!("{message}");
            usage();
            return ExitCode::from(2);
        }
    };
    let stdin = if flags.stdin_name.is_some() {
        let mut buf = String::new();
        if let Err(error) = io::stdin().read_to_string(&mut buf) {
            return fail_io(&flags, "", &error.to_string());
        }
        Some(buf)
    } else {
        None
    };
    let mut texts = Vec::new();
    for path in &flags.files {
        if flags.stdin_name.as_ref() == Some(path) {
            texts.push(stdin.clone().unwrap_or_default());
            continue;
        }
        match fs::read_to_string(path) {
            Ok(text) => texts.push(text),
            Err(error) => return fail_io(&flags, path, &error.to_string()),
        }
    }
    let sources: Vec<Source<'_>> = flags
        .files
        .iter()
        .zip(texts.iter())
        .map(|(path, text)| Source { name: path, text })
        .collect();
    match compile(&sources) {
        Ok(_) => {
            if flags.json {
                println!("{{\"diagnostics\":[]}}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            emit_diagnostics(&flags, &[error]);
            ExitCode::from(1)
        }
    }
}

fn cmd_fmt(args: &[String]) -> ExitCode {
    let parsed = match parse_fmt_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            usage();
            return ExitCode::from(2);
        }
    };
    let text = if parsed.stdin {
        let mut buf = String::new();
        if let Err(error) = io::stdin().read_to_string(&mut buf) {
            eprintln!("{}: {error}", parsed.name);
            return ExitCode::from(1);
        }
        buf
    } else {
        match fs::read_to_string(&parsed.name) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("{}: {error}", parsed.name);
                return ExitCode::from(1);
            }
        }
    };
    match format_source(&parsed.name, &text) {
        Ok(formatted) => {
            let mut stdout = io::stdout().lock();
            if stdout.write_all(formatted.as_bytes()).is_err() {
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}:{}:{}: {} 期望 {}，实际 {}",
                error.file, error.line, error.column, error.code, error.expected, error.actual
            );
            ExitCode::from(1)
        }
    }
}

struct CheckFlags {
    json: bool,
    stdin_name: Option<String>,
    files: Vec<String>,
}

struct FmtArgs {
    name: String,
    stdin: bool,
}

fn parse_check_args(args: &[String]) -> Result<CheckFlags, String> {
    let mut json = false;
    let mut stdin_name = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format=json" => json = true,
            "--format=text" => json = false,
            "--format" => {
                i += 1;
                match args.get(i).map(String::as_str) {
                    Some("json") => json = true,
                    Some("text") => json = false,
                    Some(other) => return Err(format!("未知格式 {other}")),
                    None => return Err("缺少 --format 的值".to_string()),
                }
            }
            "--stdin" => {
                i += 1;
                let Some(name) = args.get(i) else {
                    return Err("缺少 --stdin 的文件名".to_string());
                };
                stdin_name = Some(name.clone());
            }
            "--" => {
                files.extend(args[i + 1..].iter().cloned());
                break;
            }
            other if other.starts_with('-') => return Err(format!("未知参数 {other}")),
            other => files.push(other.to_string()),
        }
        i += 1;
    }
    if files.is_empty()
        && let Some(name) = &stdin_name
    {
        files.push(name.clone());
    }
    if files.is_empty() {
        return Err("缺少文件".to_string());
    }
    if let Some(name) = &stdin_name
        && !files.iter().any(|file| file == name)
    {
        return Err(format!("--stdin {name} 不在文件列表中"));
    }
    Ok(CheckFlags {
        json,
        stdin_name,
        files,
    })
}

fn parse_fmt_args(args: &[String]) -> Result<FmtArgs, String> {
    let mut stdin_name = None;
    let mut file = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--stdin" => {
                i += 1;
                let Some(name) = args.get(i) else {
                    return Err("缺少 --stdin 的文件名".to_string());
                };
                stdin_name = Some(name.clone());
            }
            "--" => {
                if args.len() > i + 2 || file.is_some() {
                    return Err("fmt 只接受一个文件".to_string());
                }
                if let Some(name) = args.get(i + 1) {
                    file = Some(name.clone());
                }
                break;
            }
            other if other.starts_with('-') => return Err(format!("未知参数 {other}")),
            other => {
                if file.is_some() {
                    return Err("fmt 只接受一个文件".to_string());
                }
                file = Some(other.to_string());
            }
        }
        i += 1;
    }
    match (stdin_name, file) {
        (Some(_), Some(_)) => Err("fmt 的 --stdin 与文件参数只能使用一个".to_string()),
        (Some(name), None) => Ok(FmtArgs { name, stdin: true }),
        (None, Some(name)) => Ok(FmtArgs { name, stdin: false }),
        (None, None) => Err("缺少文件".to_string()),
    }
}

fn fail_io(flags: &CheckFlags, file: &str, actual: &str) -> ExitCode {
    let diagnostic = Diagnostic {
        code: "E_IO",
        file: file.to_string(),
        line: 1,
        column: 1,
        expected: "可读文件".to_string(),
        actual: actual.to_string(),
    };
    emit_diagnostics(flags, &[diagnostic]);
    ExitCode::from(1)
}

fn emit_diagnostics(flags: &CheckFlags, diagnostics: &[Diagnostic]) {
    if flags.json {
        println!("{}", diagnostics_json(diagnostics));
        return;
    }
    for error in diagnostics {
        eprintln!(
            "{}:{}:{}: {} {}",
            error.file, error.line, error.column, error.code, error.actual
        );
    }
}

fn diagnostics_json(diagnostics: &[Diagnostic]) -> String {
    let mut out = String::from("{\"diagnostics\":[");
    for (index, error) in diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"code\":");
        push_json(&mut out, error.code);
        out.push_str(",\"file\":");
        push_json(&mut out, &error.file);
        out.push_str(",\"line\":");
        out.push_str(&error.line.to_string());
        out.push_str(",\"column\":");
        out.push_str(&error.column.to_string());
        out.push_str(",\"expected\":");
        push_json(&mut out, &error.expected);
        out.push_str(",\"actual\":");
        push_json(&mut out, &error.actual);
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn push_json(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
}
