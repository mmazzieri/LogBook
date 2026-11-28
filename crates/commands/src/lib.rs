//! DiaryCraft commands: one serializable layer over the diary folder, for the command line, the MCP
//! server and any other front end without a window.
//!
//! A command names entries by day, never by path: the only path is the file to attach. There is no
//! command that deletes, because DiaryCraft never deletes files.
#![forbid(unsafe_code)]

pub mod confine;

use chrono::NaiveDate;
use confine::{Access, Confinement};
use diarycraft_engine::{search, speed_list, text};
use diarycraft_file::{Diary, Fingerprint, Loaded, Saved};
pub use diarycraft_model::WeekStart;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Entries returned by a list when no limit is given, and the most a caller may ask for.
const DEFAULT_LIMIT: usize = 50;
pub const MAX_LIMIT: usize = search::MAX_HITS;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    File(#[from] diarycraft_file::Error),
    /// Outside the folders of a confined process.
    #[error("{0}")]
    Refused(String),
    #[error("the entry of {0} changed since it was read; read it again before writing")]
    Changed(NaiveDate),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    /// Number of entries, first and last day.
    Info,
    /// Days with an entry, newest first.
    List {
        #[serde(default)]
        from: Option<NaiveDate>,
        #[serde(default)]
        to: Option<NaiveDate>,
        #[serde(default)]
        limit: Option<usize>,
    },
    Read {
        date: NaiveDate,
    },
    /// Replaces the text of a day. `version` is what `read` returned, absent for a day with no entry.
    Write {
        date: NaiveDate,
        text: String,
        #[serde(default)]
        version: Option<String>,
    },
    /// Adds a paragraph at the end of a day.
    Append {
        date: NaiveDate,
        text: String,
    },
    /// Adds a thought to the speed list of a day.
    Capture {
        date: NaiveDate,
        text: String,
        #[serde(default)]
        hint: Option<String>,
        #[serde(default)]
        subject: Option<String>,
    },
    Search {
        query: String,
        #[serde(default)]
        limit: Option<usize>,
    },
    /// Copies a file into the attachments of a day.
    Attach {
        date: NaiveDate,
        file: PathBuf,
    },
    /// Copies left by conflicting changes.
    Conflicts,
    /// The settings shared by every computer that opens the diary; with a value, changes it.
    Settings {
        #[serde(default)]
        week_start: Option<WeekStart>,
    },
}

/// Who runs the commands and when: the diary has no clock of its own.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// Names this computer in a conflict copy.
    pub label: String,
    /// Makes the name of a conflict copy unique.
    pub stamp: String,
}

/// An open diary and, for an agent, the folders it may touch.
#[derive(Debug)]
pub struct Session {
    diary: Diary,
    confinement: Option<Confinement>,
}

impl Session {
    /// Opens the diary. Confined, the diary folder must be inside a write root.
    pub fn open(root: &Path, confinement: Option<Confinement>) -> Result<Self> {
        let root = match &confinement {
            Some(c) => c.check(&std::path::absolute(root).unwrap_or_else(|_| root.to_owned()), Access::Write)?,
            None => root.to_owned(),
        };
        if let Some(c) = &confinement {
            // Opening reads and may write the settings file.
            c.check(&root.join(diarycraft_file::META_DIR).join("config.json"), Access::Write)?;
        }
        Ok(Self { diary: Diary::open(&root)?, confinement })
    }

    fn check(&self, path: &Path, access: Access) -> Result<()> {
        match &self.confinement {
            Some(c) => c.check(path, access).map(|_| ()),
            None => Ok(()),
        }
    }

    fn load(&self, date: NaiveDate, access: Access) -> Result<Loaded> {
        self.check(&self.diary.entry_path(date), access)?;
        Ok(self.diary.read(date)?)
    }

    /// Runs a command on the diary as it is on disk now.
    pub fn run(&mut self, command: Command, ctx: &Ctx) -> Result<Value> {
        self.diary.scan()?;
        match command {
            Command::Info => {
                let dates: Vec<NaiveDate> = self.diary.dates().collect();
                Ok(json!({
                    "folder": self.diary.root().display().to_string(),
                    "entries": dates.len(),
                    "first": dates.first(),
                    "last": dates.last(),
                    "conflicts": self.diary.conflict_copies().len(),
                }))
            }
            Command::List { from, to, limit } => {
                let wanted = |d: &NaiveDate| from.is_none_or(|f| *d >= f) && to.is_none_or(|t| *d <= t);
                let days: Vec<NaiveDate> = self.diary.dates().rev().filter(wanted).collect();
                let entries: Vec<Value> = days
                    .iter()
                    .take(clamp(limit))
                    .map(|&date| match self.load(date, Access::Read) {
                        Ok(l) => {
                            json!({"date": date, "title": text::title(&l.text, 80), "words": text::word_count(&l.text)})
                        }
                        Err(e) => json!({"date": date, "error": e.to_string()}),
                    })
                    .collect();
                Ok(json!({"total": days.len(), "entries": entries}))
            }
            Command::Read { date } => {
                let loaded = self.load(date, Access::Read)?;
                Ok(json!({
                    "date": date,
                    "exists": loaded.base.is_some(),
                    "version": version(&loaded),
                    "text": loaded.text,
                    "thoughts": thoughts(&loaded.text),
                    "attachments": self.attachments(date)?,
                }))
            }
            Command::Write { date, text, version: expected } => {
                let loaded = self.load(date, Access::Write)?;
                if version(&loaded) != expected.map(|v| v.to_ascii_lowercase()) {
                    return Err(Error::Changed(date));
                }
                self.save(&loaded, &text, ctx)
            }
            Command::Append { date, text: added } => {
                if text::is_blank(&added) {
                    return Err(Error::Invalid("there is nothing to add: the text is blank".into()));
                }
                let loaded = self.load(date, Access::Write)?;
                let before = loaded.text.trim_end();
                let added = added.trim_end();
                let text = if before.is_empty() { format!("{added}\n") } else { format!("{before}\n\n{added}\n") };
                self.save(&loaded, &text, ctx)
            }
            Command::Capture { date, text: typed, hint, subject } => {
                let thought = speed_list::Thought { hint, text: typed, subject };
                let loaded = self.load(date, Access::Write)?;
                let Some((text, number)) = speed_list::add(&loaded.text, &thought) else {
                    return Err(Error::Invalid("there is nothing to add: the thought is blank".into()));
                };
                let mut answer = self.save(&loaded, &text, ctx)?;
                if let Some(map) = answer.as_object_mut() {
                    map.insert("number".into(), json!(number));
                }
                Ok(answer)
            }
            Command::Search { query, limit } => {
                if query.trim().is_empty() {
                    return Err(Error::Invalid("the search is blank".into()));
                }
                let mut unreadable = Vec::new();
                let mut entries = Vec::new();
                for date in self.diary.dates() {
                    match self.load(date, Access::Read) {
                        Ok(l) => entries.push((date, l.text)),
                        Err(_) => unreadable.push(date),
                    }
                }
                let hits = search::search(entries.iter().map(|(d, t)| (*d, t.as_str())), &query);
                let shown: Vec<Value> = hits
                    .iter()
                    .take(clamp(limit))
                    .map(|h| json!({"date": h.date, "snippet": h.snippet, "count": h.count}))
                    .collect();
                Ok(json!({"total": hits.len(), "hits": shown, "unreadable": unreadable}))
            }
            Command::Attach { date, file } => {
                let source = match &self.confinement {
                    Some(c) => c.check(&file, Access::Read)?,
                    None => file,
                };
                if !source.is_file() {
                    return Err(Error::Invalid(format!("{} is not a file", source.display())));
                }
                self.check(&self.diary.attachments_dir(date), Access::Write)?;
                let copy = self.diary.attach(date, &source)?;
                let name = copy.file_name().map(|n| n.to_string_lossy().into_owned());
                Ok(json!({"date": date, "attached": name, "path": copy.display().to_string()}))
            }
            Command::Conflicts => {
                let copies: Vec<Value> = self
                    .diary
                    .conflict_copies()
                    .iter()
                    .map(|(date, path)| json!({"date": date, "path": path.display().to_string()}))
                    .collect();
                Ok(json!({"conflicts": copies}))
            }
            Command::Settings { week_start } => {
                let config = self.diary.root().join(diarycraft_file::META_DIR).join("config.json");
                match week_start {
                    Some(start) => {
                        self.check(&config, Access::Write)?;
                        self.diary.set_week_start(start)?;
                    }
                    None => {
                        self.check(&config, Access::Read)?;
                        self.diary.reload_config()?;
                    }
                }
                Ok(json!({"week_start": self.diary.config().week_start()}))
            }
        }
    }

    fn save(&mut self, loaded: &Loaded, text: &str, ctx: &Ctx) -> Result<Value> {
        // The writes the command is not given a path for: the entry's folder takes its temporary
        // file and, if the entry changes right now, the conflict copy.
        if let Some(dir) = loaded.path.parent() {
            self.check(dir, Access::Write)?;
        }
        Ok(match self.diary.save(loaded, text, &ctx.label, &ctx.stamp)? {
            Saved::Written { base, .. } => json!({"date": loaded.date, "saved": true, "version": base.to_hex()}),
            Saved::Skipped => json!({"date": loaded.date, "saved": false, "reason": "blank text, no entry created"}),
            Saved::Conflict { copy, disk } => json!({
                "date": loaded.date,
                "saved": false,
                "reason": "the entry changed on disk meanwhile and was left as it is; the text is in the conflict copy",
                "conflict_copy": copy.display().to_string(),
                "version": version(&disk),
            }),
        })
    }

    fn attachments(&self, date: NaiveDate) -> Result<Vec<String>> {
        self.check(&self.diary.attachments_dir(date), Access::Read)?;
        let files = self.diary.attachments(date);
        Ok(files.iter().filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).collect())
    }
}

/// The speed list of an entry, for an answer.
fn thoughts(text: &str) -> Vec<Value> {
    speed_list::items(text)
        .into_iter()
        .map(|i| json!({"number": i.number, "hint": i.thought.hint, "text": i.thought.text, "subject": i.thought.subject}))
        .collect()
}

fn version(loaded: &Loaded) -> Option<String> {
    loaded.base.map(Fingerprint::to_hex)
}

fn clamp(limit: Option<usize>) -> usize {
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

#[cfg(test)]
mod tests;
