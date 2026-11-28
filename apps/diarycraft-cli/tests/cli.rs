//! The built program, from the outside.
#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn cli(diary: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_diarycraft-cli"))
        .arg("--diary")
        .arg(diary)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A program that refuses to start closes its input before reading it.
    let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    child.wait_with_output().unwrap()
}

fn out(o: &Output) -> String {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout.clone()).unwrap()
}

#[test]
fn every_command() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    out(&cli(d, &["write", "2026-11-24", "# Day\n\nlunch with Lucia\n"], ""));
    out(&cli(d, &["append", "2026-11-24"], "from standard input"));
    assert_eq!(out(&cli(d, &["read", "2026-11-24"], "")), "# Day\n\nlunch with Lucia\n\nfrom standard input\n");
    assert!(out(&cli(d, &["list"], "")).starts_with("2026-11-24\t"));
    assert!(out(&cli(d, &["search", "lucia", "lunch"], "")).contains("lunch with Lucia"));
    let file = d.join("note.txt");
    std::fs::write(&file, "x").unwrap();
    out(&cli(d, &["attach", "2026-11-24", file.to_str().unwrap()], ""));
    assert!(d.join("2026/11/2026-11-24/note.txt").exists());
    assert!(out(&cli(d, &["info", "--json"], "")).contains("\"entries\": 1"));
    assert_eq!(out(&cli(d, &["conflicts"], "")), "");
    assert!(!cli(d, &["write", "2026-11-24", "no version"], "").status.success());
    assert!(!cli(d, &["read", "--no-such-flag"], "").status.success());
}

#[test]
fn mcp_speaks_the_protocol_and_needs_both_roots() {
    let tmp = tempfile::tempdir().unwrap();
    let (read, diary) = (tmp.path().join("in"), tmp.path().join("diary"));
    std::fs::create_dir(&read).unwrap();
    std::fs::create_dir(&diary).unwrap();
    let (r, w) = (read.to_str().unwrap(), diary.to_str().unwrap());
    assert!(!cli(&diary, &["mcp"], "").status.success());
    assert!(!cli(&diary, &["mcp", "--automation-read-root", r], "").status.success());
    assert!(!cli(&diary, &["mcp", "--automation-write-root", w], "").status.success());
    assert!(!cli(&diary, &["mcp", "--automation-read-root", w, "--automation-write-root", r], "").status.success());

    let input = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"append","arguments":{"date":"2026-11-24","text":"hello"}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"attach","arguments":{"date":"2026-11-24","file":"/etc/passwd"}}}"#,
        "",
    ]
    .join("\n");
    let o = cli(&diary, &["mcp", "--automation-read-root", r, "--automation-write-root", w], &input);
    let text = out(&o);
    let lines: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0]["result"]["serverInfo"]["name"], "diarycraft");
    assert_eq!(lines[1]["result"]["tools"].as_array().unwrap().len(), 10);
    assert_ne!(lines[2]["result"]["isError"], true);
    assert_eq!(lines[3]["result"]["isError"], true);
    assert_eq!(std::fs::read_to_string(diary.join("2026/11/2026-11-24.md")).unwrap(), "hello\n");
}
