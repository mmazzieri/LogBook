//! The application: the open diary, the day being written, saving and watching the folder.

use crate::i18n::{Lang, Strings};
use crate::views;
use chrono::{Local, NaiveDate};
use diarycraft_engine::search::{Hit, search};
use diarycraft_engine::speed_list::{self, Thought};
use diarycraft_engine::text;
use diarycraft_file::{Diary, Encoding, Fingerprint, Loaded, Saved};
use diarycraft_model::WeekStart;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const PREFS_KEY: &str = "diarycraft";
const RECENT_MAX: usize = 8;
/// After an edit, the entry is written this long after the last keystroke.
const SAVE_DELAY: f64 = 1.0;
/// How often the open entry is compared with the file on disk.
const POLL_EVERY: f64 = 2.0;
/// How often the folder is looked through again for new days and conflict copies.
const SCAN_EVERY: f64 = 10.0;
/// How many of the latest entries are listed under the calendar.
const RECENT_ENTRIES: usize = 30;
/// Writes the page now: Ctrl+S, or ⌘S on macOS.
pub const SAVE_SHORTCUT: egui::KeyboardShortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

/// Per-computer preferences, saved by eframe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub lang: Option<Lang>,
    pub recent: Vec<PathBuf>,
    /// The diary open at the last exit, reopened at start.
    pub last: Option<PathBuf>,
    /// The page is shown formatted instead of as Markdown to edit.
    pub preview: bool,
    /// Autosave is off: nothing is written until Save (Ctrl+S). Unsaved pages are kept in memory
    /// while the app is open, and leaving with unsaved pages asks first. Off by default, so
    /// autosave is on.
    pub manual_save: bool,
    /// Light or dark colours, or those of the system (the default).
    pub theme: Theme,
}

/// The colours of the window: those of the system, or always light or dark.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        }
    }
}

/// The editor's text could not be written over the file, which changed on disk.
#[derive(Debug, Clone)]
pub struct Conflict {
    pub copy: PathBuf,
    pub disk_text: String,
    pub disk: Loaded,
    pub comparing: bool,
}

/// The day being written.
#[derive(Debug, Clone)]
pub struct Page {
    pub loaded: Loaded,
    pub text: String,
    pub attachments: Vec<PathBuf>,
}

/// What would end the session with the diary, held while the owner says what to do with the
/// unsaved pages.
#[derive(Debug, Clone)]
pub enum Leave {
    Quit,
    CloseDiary,
    Open(PathBuf),
}

/// What the views ask for while drawing; applied after the frame.
#[derive(Debug, Clone)]
pub enum Action {
    Select(NaiveDate),
    ShowMonth(NaiveDate),
    OpenDialog,
    /// Creates the folder if needed and opens it as a diary.
    Create(PathBuf),
    Open(PathBuf),
    CloseDiary,
    Attach,
    OpenPath(PathBuf),
    UseDisk,
    KeepMine,
    SetLang(Lang),
    SetTheme(Theme),
    SetWeekStart(WeekStart),
    Search,
    /// Adds what the quick field holds to today's speed list.
    Capture,
    Save,
    Quit,
    /// From the unsaved-changes question: save everything, then leave.
    SaveAndLeave,
    /// From the unsaved-changes question: drop the unsaved pages, then leave.
    DiscardAndLeave,
    CancelLeave,
}

pub struct DiaryCraftApp {
    pub prefs: Prefs,
    pub diary: Option<Diary>,
    pub page: Option<Page>,
    /// First day of the month shown by the calendar.
    pub month: NaiveDate,
    pub query: String,
    /// The quick field: a thought on its way to today's speed list.
    pub thought: String,
    pub hits: Option<Vec<Hit>>,
    /// The latest entries with their first line, newest first.
    pub recent: Vec<(NaiveDate, String)>,
    pub dirty_since: Option<f64>,
    pub conflict: Option<Conflict>,
    /// With autosave off, the pages of other days with unsaved changes, kept until Save.
    pub drafts: BTreeMap<NaiveDate, Page>,
    /// Leaving was asked while there are unsaved pages: the question is on screen.
    pub leaving: Option<Leave>,
    /// The owner answered the question: the next close request goes through.
    allow_close: bool,
    pub error: Option<String>,
    pub notice: Option<(String, f64)>,
    pub actions: Vec<Action>,
    /// What the formatted view keeps between frames.
    pub markdown_cache: egui_commonmark::CommonMarkCache,
    last_poll: f64,
    last_scan: f64,
    /// Fingerprint of the file the page shows, as last seen on disk; avoids re-reading it every poll.
    seen: Option<Option<Fingerprint>>,
    /// The system's language, read once at start.
    system_lang: Lang,
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// Name of this computer, for the conflict copies it writes.
fn computer_name() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

fn stamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

impl DiaryCraftApp {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        let prefs: Prefs = cc.storage.and_then(|s| eframe::get_value(s, PREFS_KEY)).unwrap_or_default();
        let mut app = Self {
            prefs,
            diary: None,
            page: None,
            month: diarycraft_engine::calendar::month_start(today()),
            query: String::new(),
            thought: String::new(),
            hits: None,
            recent: Vec::new(),
            dirty_since: None,
            conflict: None,
            drafts: BTreeMap::new(),
            leaving: None,
            allow_close: false,
            error: None,
            notice: None,
            actions: Vec::new(),
            markdown_cache: egui_commonmark::CommonMarkCache::default(),
            last_poll: 0.0,
            last_scan: 0.0,
            seen: None,
            system_lang: Lang::from_system(),
        };
        cc.egui_ctx.set_theme(app.prefs.theme.preference());
        if let Some(path) = open.or_else(|| app.prefs.last.clone()) {
            app.open(&path);
        }
        app
    }

    pub fn strings(&self) -> &'static Strings {
        self.lang().strings()
    }

    /// The language chosen in File › Language, or else the system's.
    pub fn lang(&self) -> Lang {
        self.prefs.lang.unwrap_or(self.system_lang)
    }

    /// Whether something typed is not on disk yet: the page on screen or a kept page.
    pub fn has_unsaved(&self) -> bool {
        self.dirty_since.is_some() || !self.drafts.is_empty()
    }

    /// The days with unsaved changes, the one on screen included.
    pub fn unsaved_days(&self) -> Vec<NaiveDate> {
        let mut days: Vec<NaiveDate> = self.drafts.keys().copied().collect();
        if self.dirty_since.is_some()
            && let Some(page) = &self.page
        {
            days.push(page.loaded.date);
        }
        days.sort();
        days
    }

    /// Leaves now, or asks first when autosave is off and something is not saved.
    fn leave(&mut self, leave: Leave, ctx: &egui::Context) {
        if self.prefs.manual_save && self.has_unsaved() {
            self.leaving = Some(leave);
            return;
        }
        self.do_leave(leave, ctx);
    }

    fn do_leave(&mut self, leave: Leave, ctx: &egui::Context) {
        match leave {
            Leave::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Leave::CloseDiary => self.close_diary(),
            Leave::Open(path) => self.open(&path),
        }
    }

    /// Writes every unsaved page: the one on screen and the kept ones. A kept page that changed
    /// on disk meanwhile is written as a conflict copy, as the page on screen would be.
    fn save_all(&mut self) {
        self.flush();
        let lang = self.lang();
        let Some(diary) = &mut self.diary else { return };
        for (date, draft) in std::mem::take(&mut self.drafts) {
            match diary.save(&draft.loaded, &draft.text, &computer_name(), &stamp()) {
                Ok(Saved::Written { .. } | Saved::Skipped) => {}
                Ok(Saved::Conflict { copy, .. }) => {
                    self.error = Some(format!("{}: {}", lang.strings().conflict_title, copy.display()));
                }
                Err(e) => {
                    self.error = Some(lang.describe(&e));
                    self.drafts.insert(date, draft);
                }
            }
        }
        self.refresh_recent();
    }

    fn open(&mut self, path: &Path) {
        self.flush();
        self.drafts.clear();
        match Diary::open(path) {
            Ok(diary) => {
                self.diary = Some(diary);
                self.error = None;
                self.prefs.recent.retain(|p| p != path);
                self.prefs.recent.insert(0, path.to_owned());
                self.prefs.recent.truncate(RECENT_MAX);
                self.prefs.last = Some(path.to_owned());
                self.hits = None;
                self.query.clear();
                self.conflict = None;
                self.refresh_recent();
                self.select(today());
            }
            Err(e) => self.error = Some(self.lang().describe(&e)),
        }
    }

    fn close_diary(&mut self) {
        self.flush();
        self.drafts.clear();
        self.diary = None;
        self.page = None;
        self.conflict = None;
        self.hits = None;
        self.recent.clear();
        self.prefs.last = None;
    }

    /// Opens the page of a day, after writing the one being left; with autosave off the page
    /// being left is kept in memory instead, and a kept page comes back as it was left.
    fn select(&mut self, date: NaiveDate) {
        if self.prefs.manual_save {
            if self.conflict.is_some() {
                return;
            }
            if self.dirty_since.take().is_some()
                && let Some(page) = self.page.take()
            {
                self.drafts.insert(page.loaded.date, page);
            }
            if let Some(draft) = self.drafts.remove(&date) {
                self.month = diarycraft_engine::calendar::month_start(date);
                self.seen = Some(draft.loaded.base);
                self.page = Some(draft);
                self.dirty_since = Some(0.0);
                return;
            }
        }
        self.flush();
        if self.conflict.is_some() {
            // The conflict must be settled first: leaving would hide it.
            return;
        }
        let Some(diary) = &self.diary else { return };
        match diary.read(date) {
            Ok(loaded) => {
                self.month = diarycraft_engine::calendar::month_start(date);
                self.seen = Some(loaded.base);
                let attachments = diary.attachments(date);
                self.page = Some(Page { text: loaded.text.clone(), loaded, attachments });
                self.dirty_since = None;
            }
            Err(e) => self.error = Some(self.lang().describe(&e)),
        }
    }

    pub fn mark_dirty(&mut self, time: f64) {
        self.dirty_since = Some(time);
    }

    /// Writes the page if it has changes.
    pub fn flush(&mut self) {
        if self.dirty_since.is_none() {
            return;
        }
        let (Some(diary), Some(page)) = (&mut self.diary, &mut self.page) else { return };
        match diary.save(&page.loaded, &page.text, &computer_name(), &stamp()) {
            Ok(Saved::Written { base, path }) => {
                page.loaded.written(base, path);
                self.seen = Some(Some(base));
                self.dirty_since = None;
                self.error = None;
            }
            Ok(Saved::Skipped) => self.dirty_since = None,
            Ok(Saved::Conflict { copy, disk }) => {
                self.seen = Some(disk.base);
                self.conflict = Some(Conflict { copy, disk_text: disk.text.clone(), disk, comparing: false });
                self.dirty_since = None;
            }
            Err(e) => self.error = Some(self.lang().describe(&e)),
        }
        self.refresh_recent();
    }

    fn refresh_recent(&mut self) {
        let Some(diary) = &self.diary else { return };
        self.recent = diary
            .dates()
            .rev()
            .take(RECENT_ENTRIES)
            .map(|date| {
                let title = diary.read(date).map(|l| text::title(&l.text, 60)).unwrap_or_default();
                (date, title)
            })
            .collect();
    }

    /// Reloads the page if its file changed on disk and the editor has nothing unsaved.
    fn poll(&mut self) {
        if self.dirty_since.is_some() || self.conflict.is_some() {
            return;
        }
        let (Some(diary), Some(page)) = (&self.diary, &mut self.page) else { return };
        let Ok(disk) = diary.read(page.loaded.date) else { return };
        if Some(disk.base) == self.seen {
            return;
        }
        self.seen = Some(disk.base);
        if disk.base != page.loaded.base {
            page.text = disk.text.clone();
            page.loaded = disk;
            let s = self.prefs.lang.unwrap_or(self.system_lang).strings();
            self.notice = Some((s.reloaded.to_owned(), 0.0));
        }
    }

    fn tick(&mut self, time: f64) {
        if !self.prefs.manual_save {
            if self.dirty_since.is_some_and(|t| time - t >= SAVE_DELAY) {
                self.flush();
            }
            // Autosave was turned back on with pages kept from before.
            if !self.drafts.is_empty() {
                self.save_all();
            }
        }
        if time - self.last_poll >= POLL_EVERY {
            self.last_poll = time;
            self.poll();
        }
        if time - self.last_scan >= SCAN_EVERY {
            self.last_scan = time;
            if let Some(diary) = &mut self.diary {
                // The first day of the week may have been changed on another computer. A settings
                // file that cannot be read now keeps the settings as they were.
                let _ = diary.reload_config();
                if let Err(e) = diary.scan() {
                    self.error = Some(self.lang().describe(&e));
                }
            }
            self.refresh_recent();
        }
        if let Some((_, since)) = &mut self.notice {
            if *since == 0.0 {
                *since = time;
            } else if time - *since > 6.0 {
                self.notice = None;
            }
        }
    }

    /// Adds the quick field's thought to today's speed list: into the page when today is on
    /// screen, into a kept page with autosave off, else straight to disk with the usual conflict
    /// rule. The field is emptied only once the thought is somewhere.
    fn capture(&mut self) {
        let Some(thought) = Thought::parse(&self.thought) else { return };
        let s = self.strings();
        let date = today();
        let added = |text: &str| speed_list::add(text, &thought);
        let number = if let Some(page) = self.page.as_mut().filter(|p| p.loaded.date == date) {
            if self.conflict.is_some() {
                self.error = Some(s.capture_locked.to_owned());
                return;
            }
            let Some((text, n)) = added(&page.text) else { return };
            page.text = text;
            // Saved at the next frame with autosave on, kept unsaved with it off.
            self.dirty_since = Some(0.0);
            n
        } else if let Some(draft) = self.drafts.get_mut(&date) {
            let Some((text, n)) = added(&draft.text) else { return };
            draft.text = text;
            n
        } else {
            let Some(diary) = &mut self.diary else { return };
            let loaded = match diary.read(date) {
                Ok(l) => l,
                Err(e) => {
                    self.error = Some(self.lang().describe(&e));
                    return;
                }
            };
            let Some((text, n)) = added(&loaded.text) else { return };
            if self.prefs.manual_save {
                let attachments = diary.attachments(date);
                self.drafts.insert(date, Page { loaded, text, attachments });
            } else {
                match diary.save(&loaded, &text, &computer_name(), &stamp()) {
                    Ok(Saved::Written { .. } | Saved::Skipped) => {}
                    Ok(Saved::Conflict { copy, .. }) => {
                        self.error = Some(format!("{}: {}", s.conflict_title, copy.display()));
                    }
                    Err(e) => {
                        self.error = Some(self.lang().describe(&e));
                        return;
                    }
                }
                self.refresh_recent();
            }
            n
        };
        self.thought.clear();
        self.notice = Some((format!("{} {number}", s.captured), 0.0));
    }

    fn run_search(&mut self) {
        let Some(diary) = &self.diary else { return };
        if self.query.trim().is_empty() {
            self.hits = None;
            return;
        }
        let mut all = diary.load_all();
        // The pages being written count with their unsaved text.
        for page in self.page.iter().chain(self.drafts.values()) {
            all.retain(|(d, _)| *d != page.loaded.date);
            all.push((page.loaded.date, page.text.clone()));
        }
        self.hits = Some(search(all.iter().map(|(d, t)| (*d, t.as_str())), &self.query));
    }

    fn attach(&mut self) {
        let lang = self.lang();
        let (Some(diary), Some(page)) = (&self.diary, &mut self.page) else { return };
        let Some(files) = rfd::FileDialog::new().pick_files() else { return };
        let date = page.loaded.date;
        for file in files {
            match diary.attach(date, &file) {
                Ok(target) => {
                    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    // A Markdown link relative to the entry, in angle brackets so spaces are allowed.
                    let link = format!("[{name}](<{}/{name}>)", date.format("%Y-%m-%d"));
                    if !page.text.is_empty() && !page.text.ends_with('\n') {
                        page.text.push('\n');
                    }
                    page.text.push_str(&link);
                    page.text.push('\n');
                    self.dirty_since = Some(0.0);
                }
                Err(e) => self.error = Some(lang.describe(&e)),
            }
        }
        page.attachments = diary.attachments(date);
        if !self.prefs.manual_save {
            self.flush();
        }
    }

    fn use_disk(&mut self) {
        let Some(conflict) = self.conflict.take() else { return };
        if let Some(page) = &mut self.page {
            page.text = conflict.disk_text;
            page.loaded = conflict.disk;
        }
        self.dirty_since = None;
    }

    fn keep_mine(&mut self) {
        let Some(conflict) = self.conflict.take() else { return };
        if let Some(page) = &mut self.page {
            // The editor's text goes over the version now on disk; that version stays readable in
            // the history of the sync client, and the editor's text is also in the conflict copy.
            page.loaded = conflict.disk;
            self.dirty_since = Some(0.0);
        }
        self.flush();
    }

    fn apply_actions(&mut self, ctx: &egui::Context) {
        for action in std::mem::take(&mut self.actions) {
            match action {
                Action::Select(date) => self.select(date),
                Action::ShowMonth(date) => self.month = diarycraft_engine::calendar::month_start(date),
                Action::OpenDialog => {
                    let mut dialog = rfd::FileDialog::new();
                    if let Some(start) = crate::folders::documents() {
                        dialog = dialog.set_directory(start);
                    }
                    if let Some(p) = dialog.pick_folder() {
                        self.leave(Leave::Open(p), ctx);
                    }
                }
                Action::Create(p) => match std::fs::create_dir_all(&p) {
                    Ok(()) => self.open(&p),
                    Err(e) => self.error = Some(format!("{}: {e}", p.display())),
                },
                Action::Open(p) => self.leave(Leave::Open(p), ctx),
                Action::CloseDiary => self.leave(Leave::CloseDiary, ctx),
                Action::Attach => self.attach(),
                Action::OpenPath(p) => open_with_system(&p),
                Action::UseDisk => self.use_disk(),
                Action::KeepMine => self.keep_mine(),
                Action::SetLang(lang) => self.prefs.lang = Some(lang),
                Action::SetTheme(theme) => {
                    self.prefs.theme = theme;
                    ctx.set_theme(theme.preference());
                }
                Action::SetWeekStart(start) => {
                    if let Some(diary) = &mut self.diary
                        && let Err(e) = diary.set_week_start(start)
                    {
                        self.error = Some(self.lang().describe(&e));
                    }
                }
                Action::Search => self.run_search(),
                Action::Capture => self.capture(),
                Action::Save => self.save_all(),
                Action::Quit => self.leave(Leave::Quit, ctx),
                Action::SaveAndLeave => {
                    self.save_all();
                    // Leave only if everything is on disk; an error stays on screen.
                    if !self.has_unsaved()
                        && let Some(leave) = self.leaving.take()
                    {
                        self.do_leave(leave, ctx);
                    }
                }
                Action::DiscardAndLeave => {
                    self.drafts.clear();
                    self.dirty_since = None;
                    if let Some(leave) = self.leaving.take() {
                        self.do_leave(leave, ctx);
                    }
                }
                Action::CancelLeave => self.leaving = None,
            }
        }
    }

    pub fn status(&self) -> (String, Option<String>) {
        let s = self.strings();
        let words = self.page.as_ref().map(|p| text::word_count(&p.text)).unwrap_or(0);
        let state = match (self.has_unsaved(), self.prefs.manual_save) {
            (false, _) => s.saved,
            (true, false) => s.pending,
            (true, true) => s.pending_manual,
        };
        let mut left = format!("{state} · {words} {}", s.words);
        if self.page.as_ref().is_some_and(|p| p.loaded.encoding == Encoding::Windows1252) {
            left = format!("{left} · {}", s.legacy_encoding);
        }
        let copies = self.diary.as_ref().map(|d| d.conflict_copies().len()).unwrap_or(0);
        let right = (copies > 0).then(|| format!("{copies} {}", s.conflict_copies));
        (left, right)
    }
}

/// Opens a file or folder with the program the system uses for it. Run as a program with the path
/// as its one argument, never through a shell.
fn open_with_system(path: &Path) {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();
    drop(result);
}

impl eframe::App for DiaryCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let time = ctx.input(|i| i.time);
        if ctx.input_mut(|i| i.consume_shortcut(&SAVE_SHORTCUT)) {
            self.actions.push(Action::Save);
        }
        // Closing the window with unsaved pages and autosave off asks first.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.prefs.manual_save
            && self.has_unsaved()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.leaving = Some(Leave::Quit);
        }
        self.tick(time);
        ctx.request_repaint_after(Duration::from_millis(500));

        egui::Panel::top("menu").show(ui, |ui| views::top_bar(ui, self));
        egui::Panel::bottom("status").show(ui, |ui| views::status_bar(ui, self));
        if self.diary.is_some() {
            egui::Panel::left("calendar").resizable(true).default_size(260.0).min_size(220.0).show(ui, |ui| {
                views::side(ui, self);
            });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            if self.diary.is_none() {
                views::empty(ui, self);
            } else {
                views::page(ui, self, time);
            }
        });
        views::compare_window(&ctx, self);
        views::unsaved_window(&ctx, self);
        self.apply_actions(&ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // eframe calls this every half minute too: with autosave off nothing is written here.
        if !self.prefs.manual_save {
            self.flush();
        }
        eframe::set_value(storage, PREFS_KEY, &self.prefs);
    }

    fn on_exit(&mut self) {
        // With autosave off the owner has already said whether to save or not.
        if !self.prefs.manual_save {
            self.flush();
        }
    }
}
