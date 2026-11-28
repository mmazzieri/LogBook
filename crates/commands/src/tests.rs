use crate::confine::{Access, Confinement};
use crate::{Command, Ctx, Error, Session};
use chrono::NaiveDate;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn ctx() -> Ctx {
    Ctx { label: "PC".into(), stamp: "20261124-120000".into() }
}

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 24).unwrap()
}

fn run(s: &mut Session, v: Value) -> Result<Value, Error> {
    s.run(serde_json::from_value(v).unwrap(), &ctx())
}

#[test]
fn write_needs_the_version_that_was_read() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::open(dir.path(), None).unwrap();
    let w = run(&mut s, json!({"cmd": "write", "date": "2026-11-24", "text": "one\n"})).unwrap();
    assert_eq!(w["saved"], true);
    let r = run(&mut s, json!({"cmd": "read", "date": "2026-11-24"})).unwrap();
    assert_eq!((&r["text"], &r["version"]), (&json!("one\n"), &w["version"]));

    // Without a version, and after a change made elsewhere, nothing is written.
    let stale = run(&mut s, json!({"cmd": "write", "date": "2026-11-24", "text": "x"}));
    assert!(matches!(stale, Err(Error::Changed(d)) if d == day()));
    let path = dir.path().join("2026/11/2026-11-24.md");
    fs::write(&path, "another computer\n").unwrap();
    let stale = run(&mut s, json!({"cmd": "write", "date": "2026-11-24", "text": "x", "version": w["version"]}));
    assert!(matches!(stale, Err(Error::Changed(_))));
    assert_eq!(fs::read_to_string(&path).unwrap(), "another computer\n");
}

#[test]
fn settings_change_the_first_day_of_the_week() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::open(dir.path(), None).unwrap();
    assert_eq!(run(&mut s, json!({"cmd": "settings"})).unwrap()["week_start"], "monday");
    run(&mut s, json!({"cmd": "settings", "week_start": "sunday"})).unwrap();
    let again = Session::open(dir.path(), None).unwrap().run(Command::Settings { week_start: None }, &ctx()).unwrap();
    assert_eq!(again["week_start"], "sunday");
    assert!(serde_json::from_value::<Command>(json!({"cmd": "settings", "week_start": "friday"})).is_err());
    // A settings file broken after the diary was opened is an error, not the old answer.
    fs::write(dir.path().join(".diarycraft/config.json"), "not json").unwrap();
    assert!(run(&mut s, json!({"cmd": "settings"})).is_err());
}

#[test]
fn append_list_search_info() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::open(dir.path(), None).unwrap();
    for text in ["# Title", "lunch with Lucia"] {
        run(&mut s, json!({"cmd": "append", "date": "2026-11-24", "text": text})).unwrap();
    }
    run(&mut s, json!({"cmd": "append", "date": "2026-11-23", "text": "work"})).unwrap();
    assert!(run(&mut s, json!({"cmd": "append", "date": "2026-11-23", "text": " \n"})).is_err());
    let r = run(&mut s, json!({"cmd": "read", "date": "2026-11-24"})).unwrap();
    assert_eq!(r["text"], "# Title\n\nlunch with Lucia\n");
    let l = run(&mut s, json!({"cmd": "list", "limit": 1})).unwrap();
    assert_eq!((&l["total"], &l["entries"][0]["title"]), (&json!(2), &json!("Title")));
    let l = run(&mut s, json!({"cmd": "list", "to": "2026-11-23"})).unwrap();
    assert_eq!(l["entries"][0]["date"], "2026-11-23");
    let h = run(&mut s, json!({"cmd": "search", "query": "LUCIA"})).unwrap();
    assert_eq!((&h["total"], &h["hits"][0]["date"]), (&json!(1), &json!("2026-11-24")));
    let i = run(&mut s, json!({"cmd": "info"})).unwrap();
    assert_eq!((&i["entries"], &i["first"]), (&json!(2), &json!("2026-11-23")));
}

#[test]
fn unknown_fields_are_refused() {
    for v in [
        json!({"cmd": "read", "date": "2026-02-30"}),
        json!({"cmd": "delete", "date": "2026-11-24"}),
        json!({"cmd": "read", "date": "2026-11-24", "path": "/etc/passwd"}),
    ] {
        assert!(serde_json::from_value::<Command>(v.clone()).is_err(), "{v}");
    }
}

struct Sandbox {
    _tmp: tempfile::TempDir,
    read: PathBuf,
    write: PathBuf,
    outside: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let [read, write, outside] = ["read", "write", "write-other"].map(|n| base.join(n));
        for d in [&read, &write, &outside] {
            fs::create_dir(d).unwrap();
        }
        fs::write(outside.join("secret.md"), "MARKER").unwrap();
        fs::write(read.join("photo.txt"), "photo").unwrap();
        Self { _tmp: tmp, read, write, outside }
    }

    fn confinement(&self) -> Confinement {
        Confinement::new(std::slice::from_ref(&self.read), std::slice::from_ref(&self.write)).unwrap()
    }
}

#[test]
fn both_roots_are_needed() {
    let b = Sandbox::new();
    let roots = [b.read.clone()];
    assert!(Confinement::new(&roots, &[]).is_err());
    assert!(Confinement::new(&[], &roots).is_err());
    assert!(Confinement::new(&roots, &[b.write.join("missing")]).is_err());
    assert!(Confinement::new(&roots, &[b.read.join("photo.txt")]).is_err());
}

#[test]
fn paths_outside_the_roots_are_refused() {
    let b = Sandbox::new();
    let c = b.confinement();
    for p in [
        b.outside.join("secret.md"),
        b.write.join("../write-other/secret.md"),
        b.read.join("missing/../../write-other/secret.md"),
        PathBuf::from("../write-other/secret.md"),
    ] {
        assert!(c.check(&p, Access::Read).is_err(), "{}", p.display());
        assert!(c.check(&p, Access::Write).is_err(), "{}", p.display());
    }
    // The read root can be read and not written; the write root can be both.
    assert!(c.check(&b.read.join("photo.txt"), Access::Read).is_ok());
    assert!(c.check(&b.read.join("photo.txt"), Access::Write).is_err());
    assert!(c.check(&b.write.join("new/2026/x.md"), Access::Write).is_ok());
    assert!(Session::open(&b.read, Some(c.clone())).is_err());
    assert!(Session::open(&b.outside, Some(c)).is_err());
}

#[test]
fn attach_takes_files_of_the_read_root_only() {
    let b = Sandbox::new();
    let mut s = Session::open(&b.write, Some(b.confinement())).unwrap();
    let ok = run(&mut s, json!({"cmd": "attach", "date": "2026-11-24", "file": b.read.join("photo.txt")})).unwrap();
    assert_eq!(ok["attached"], "photo.txt");
    let no = run(&mut s, json!({"cmd": "attach", "date": "2026-11-24", "file": b.outside.join("secret.md")}));
    assert!(matches!(no, Err(Error::Refused(_))));
    let r = run(&mut s, json!({"cmd": "read", "date": "2026-11-24"})).unwrap();
    assert_eq!(r["attachments"], json!(["photo.txt"]));
}

#[cfg(unix)]
#[test]
fn links_do_not_lead_out_of_the_diary() {
    use std::os::unix::fs::symlink;
    let b = Sandbox::new();
    let month = b.write.join("2026/11");
    fs::create_dir_all(&month).unwrap();
    let secret = b.outside.join("secret.md");
    symlink(&secret, month.join("2026-11-01.md")).unwrap();
    fs::hard_link(&secret, month.join("2026-11-02.md")).unwrap();
    symlink(&b.outside, month.join("2026-11-03")).unwrap();
    symlink(b.outside.join("new.md"), month.join("2026-11-04.md")).unwrap();
    symlink(&b.outside, b.write.join("2025")).unwrap();
    let mut s = Session::open(&b.write, Some(b.confinement())).unwrap();
    for date in ["2026-11-01", "2026-11-02", "2026-11-04", "2025-01-01"] {
        for cmd in ["read", "append"] {
            let mut v = json!({"cmd": cmd, "date": date});
            if cmd == "append" {
                v["text"] = json!("overwritten");
            }
            assert!(matches!(run(&mut s, v), Err(Error::Refused(_))), "{cmd} {date}");
        }
    }
    let attach = json!({"cmd": "attach", "date": "2026-11-03", "file": b.read.join("photo.txt")});
    assert!(matches!(run(&mut s, attach), Err(Error::Refused(_))));
    let found = run(&mut s, json!({"cmd": "search", "query": "marker"})).unwrap();
    assert_eq!(found["total"], 0);
    assert_eq!(fs::read_to_string(&secret).unwrap(), "MARKER");
    assert_eq!(fs::read_dir(&b.outside).unwrap().count(), 1);

    let meta = b.write.join("linked");
    fs::create_dir(&meta).unwrap();
    symlink(&b.outside, meta.join(".diarycraft")).unwrap();
    assert!(Session::open(&meta, Some(b.confinement())).is_err());
}

#[test]
fn capture_numbers_the_thoughts_of_a_day() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::open(dir.path(), None).unwrap();
    run(&mut s, json!({"cmd": "append", "date": "2026-11-24", "text": "A day."})).unwrap();
    let first =
        run(&mut s, json!({"cmd": "capture", "date": "2026-11-24", "text": "call the agency", "hint": "house"}))
            .unwrap();
    let second =
        run(&mut s, json!({"cmd": "capture", "date": "2026-11-24", "text": "maps", "subject": "writing"})).unwrap();
    assert_eq!((&first["number"], &second["number"]), (&json!(1), &json!(2)));
    let r = run(&mut s, json!({"cmd": "read", "date": "2026-11-24"})).unwrap();
    assert!(r["text"].as_str().unwrap().starts_with("A day.\n"));
    assert_eq!(r["thoughts"][0]["hint"], "house");
    assert_eq!(r["thoughts"][1]["subject"], "writing");
    assert!(matches!(
        run(&mut s, json!({"cmd": "capture", "date": "2026-11-24", "text": " \n"})),
        Err(Error::Invalid(_))
    ));
}
