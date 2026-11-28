//! DiaryCraft without a window: every command of the diary, and an MCP server for agents.
#![forbid(unsafe_code)]

mod mcp;

use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use diarycraft_commands::confine::Confinement;
use diarycraft_commands::{Command, Ctx, Session, WeekStart};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

/// Reads and writes a DiaryCraft diary without a window.
#[derive(Debug, Parser)]
#[command(name = "diarycraft-cli", version)]
struct Cli {
    /// Folder of the diary
    #[arg(long, env = "DIARYCRAFT_DIARY", value_name = "DIR")]
    diary: PathBuf,
    /// Print the whole answer as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Number of entries, first and last day
    Info,
    /// Days with an entry, newest first
    List {
        #[arg(long, value_parser = date)]
        from: Option<NaiveDate>,
        #[arg(long, value_parser = date)]
        to: Option<NaiveDate>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Print the entry of a day
    Read {
        #[arg(value_parser = date, default_value = "today")]
        date: NaiveDate,
    },
    /// Replace the entry of a day with TEXT (standard input without it)
    Write {
        #[arg(value_parser = date)]
        date: NaiveDate,
        text: Option<String>,
        /// Version given by `read --json`; needed to replace an entry that exists
        #[arg(long = "entry-version", value_name = "VERSION")]
        version: Option<String>,
    },
    /// Add a paragraph to the entry of a day (standard input without TEXT)
    Append {
        #[arg(value_parser = date)]
        date: NaiveDate,
        text: Option<String>,
    },
    /// Add a thought to the speed list of a day (standard input without TEXT)
    Capture {
        text: Option<String>,
        /// The day; today without it
        #[arg(long, value_parser = date, default_value = "today")]
        date: NaiveDate,
        /// One to three words that bring the thought back to mind
        #[arg(long)]
        hint: Option<String>,
        /// What the thought is about, one word
        #[arg(long)]
        subject: Option<String>,
    },
    /// Entries with every word of the search
    Search {
        #[arg(required = true)]
        words: Vec<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Copy a file into the attachments of a day
    Attach {
        #[arg(value_parser = date)]
        date: NaiveDate,
        file: PathBuf,
    },
    /// Copies left by conflicting changes
    Conflicts,
    /// The diary's settings, shared by every computer; with an option, change it
    Settings {
        /// First day of the calendar's weeks
        #[arg(long, value_parser = week_start, value_name = "monday|sunday")]
        week_start: Option<WeekStart>,
    },
    /// Serve these commands to an agent (MCP on standard input and output)
    Mcp {
        /// Folder the agent may read files to attach from (repeatable)
        #[arg(long, required = true, value_name = "DIR")]
        automation_read_root: Vec<PathBuf>,
        /// Folder the agent may write in; the diary must be inside one (repeatable)
        #[arg(long, required = true, value_name = "DIR")]
        automation_write_root: Vec<PathBuf>,
    },
}

/// A day as `YYYY-MM-DD`, or `today`.
fn date(s: &str) -> Result<NaiveDate, String> {
    if s == "today" {
        return Ok(Local::now().date_naive());
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| "a day is YYYY-MM-DD, or today".to_owned())
}

fn week_start(s: &str) -> Result<WeekStart, String> {
    match s {
        "monday" => Ok(WeekStart::Monday),
        "sunday" => Ok(WeekStart::Sunday),
        _ => Err("the week starts on monday or sunday".to_owned()),
    }
}

/// The computer and the time, for the name of a conflict copy.
fn ctx() -> Ctx {
    let label = ["COMPUTERNAME", "HOSTNAME"].iter().find_map(|v| std::env::var(v).ok()).unwrap_or_default();
    Ctx { label, stamp: Local::now().format("%Y%m%d-%H%M%S").to_string() }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("diarycraft-cli: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let text_or_stdin = |text: Option<String>| match text {
        Some(t) => Ok(t),
        None => {
            let mut t = String::new();
            std::io::stdin().read_to_string(&mut t).map(|_| t).map_err(|e| e.to_string())
        }
    };
    let command = match cli.cmd {
        Cmd::Mcp { automation_read_root, automation_write_root } => {
            let roots = Confinement::new(&automation_read_root, &automation_write_root).map_err(|e| e.to_string())?;
            let session = Session::open(&cli.diary, Some(roots)).map_err(|e| e.to_string())?;
            return mcp::serve(session).map_err(|e| e.to_string());
        }
        Cmd::Info => Command::Info,
        Cmd::List { from, to, limit } => Command::List { from, to, limit },
        Cmd::Read { date } => Command::Read { date },
        Cmd::Write { date, text, version } => Command::Write { date, text: text_or_stdin(text)?, version },
        Cmd::Append { date, text } => Command::Append { date, text: text_or_stdin(text)? },
        Cmd::Capture { text, date, hint, subject } => {
            Command::Capture { date, text: text_or_stdin(text)?, hint, subject }
        }
        Cmd::Search { words, limit } => Command::Search { query: words.join(" "), limit },
        Cmd::Attach { date, file } => Command::Attach { date, file },
        Cmd::Conflicts => Command::Conflicts,
        Cmd::Settings { week_start } => Command::Settings { week_start },
    };
    let mut session = Session::open(&cli.diary, None).map_err(|e| e.to_string())?;
    let answer = session.run(command.clone(), &ctx()).map_err(|e| e.to_string())?;
    let text = if cli.json {
        serde_json::to_string_pretty(&answer).map_err(|e| e.to_string())?
    } else {
        plain(&command, &answer)
    };
    let mut out = std::io::stdout().lock();
    out.write_all(text.as_bytes())
        .and_then(|()| if text.is_empty() || text.ends_with('\n') { Ok(()) } else { out.write_all(b"\n") })
        .map_err(|e| e.to_string())?;
    // A text left in a conflict copy is kept, but the entry was not written.
    match answer.get("conflict_copy").and_then(Value::as_str) {
        Some(copy) => Err(format!("the entry changed meanwhile; the text is in {copy}")),
        None => Ok(()),
    }
}

/// The answer for a person.
fn plain(command: &Command, answer: &Value) -> String {
    let field = |v: &Value, k: &str| match v.get(k) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    let rows = |list: &str, keys: &[&str]| {
        let items = answer.get(list).and_then(Value::as_array);
        items
            .into_iter()
            .flatten()
            .map(|i| keys.iter().map(|k| field(i, k)).collect::<Vec<_>>().join("\t") + "\n")
            .collect::<String>()
    };
    match command {
        Command::Read { .. } => field(answer, "text"),
        Command::List { .. } => rows("entries", &["date", "words", "title"]),
        Command::Search { .. } => rows("hits", &["date", "count", "snippet"]),
        Command::Conflicts => rows("conflicts", &["date", "path"]),
        _ => answer
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(_, v)| !v.is_null())
            .map(|(k, _)| format!("{k}: {}\n", field(answer, k)))
            .collect(),
    }
}
