//! Interface text in Italian and English.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    It,
    En,
}

impl Lang {
    /// The language of the system: the usual environment variables when set (so that
    /// `LANG=en_US.UTF-8` overrides the desktop), otherwise the language of the user interface as
    /// the system reports it (Windows and macOS do not set those variables). English otherwise.
    pub fn from_system() -> Self {
        let env =
            ["LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|k| std::env::var(k).ok()).find(|v| !v.is_empty());
        let tag = env.or_else(sys_locale::get_locale).unwrap_or_default();
        Self::from_tag(&tag)
    }

    /// The language for a locale name or language tag: `it`, `it-IT`, `it_CH.UTF-8`…
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag.split(['-', '_', '.', '@']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("it") { Lang::It } else { Lang::En }
    }

    /// An error of the diary folder, in this language.
    pub fn describe(self, e: &diarycraft_file::Error) -> String {
        use diarycraft_file::Error;
        let it = self == Lang::It;
        match e {
            Error::Io { path, source } => format!("{}: {source}", path.display()),
            Error::NotAFolder(p) if it => format!("{} non è una cartella", p.display()),
            Error::NotAFolder(p) => format!("{} is not a folder", p.display()),
            Error::Config { path, message } if it => {
                format!("{}: le impostazioni del diario non si possono leggere ({message})", path.display())
            }
            Error::Config { path, message } => {
                format!("{}: the diary settings cannot be read ({message})", path.display())
            }
            Error::TooNew { found } if it => format!(
                "Questo diario è stato scritto da una versione più recente di DiaryCraft (formato {found}); questa versione legge fino al formato {}.",
                diarycraft_model::FORMAT_VERSION
            ),
            Error::TooNew { found } => format!(
                "This diary was written by a newer DiaryCraft (format {found}); this version reads up to format {}.",
                diarycraft_model::FORMAT_VERSION
            ),
            Error::TooLarge { path } if it => format!("{}: la voce supera i 16 MB e non viene aperta", path.display()),
            Error::TooLarge { path } => format!("{}: the entry is larger than 16 MB and is not opened", path.display()),
        }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::It => &IT,
            Lang::En => &EN,
        }
    }
}

pub struct Strings {
    pub file: &'static str,
    pub open_diary: &'static str,
    pub open_recent: &'static str,
    pub close_diary: &'static str,
    pub quit: &'static str,
    pub language: &'static str,
    pub theme: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub week_start: &'static str,
    pub week_start_hint: &'static str,
    pub monday: &'static str,
    pub sunday: &'static str,
    pub today: &'static str,
    pub search_hint: &'static str,
    pub capture_hint: &'static str,
    pub capture_tip: &'static str,
    pub captured: &'static str,
    pub capture_locked: &'static str,
    pub no_results: &'static str,
    pub recent_entries: &'static str,
    pub results: &'static str,
    pub empty_title: &'static str,
    pub empty_body: &'static str,
    pub use_suggested: &'static str,
    pub choose_other: &'static str,
    pub saved: &'static str,
    pub pending: &'static str,
    pub pending_manual: &'static str,
    pub autosave: &'static str,
    pub autosave_hint: &'static str,
    pub save: &'static str,
    pub discard: &'static str,
    pub cancel: &'static str,
    pub unsaved_title: &'static str,
    pub unsaved_body: &'static str,
    pub words: &'static str,
    pub attach: &'static str,
    pub attachments: &'static str,
    pub open_folder: &'static str,
    pub conflict_title: &'static str,
    pub conflict_body: &'static str,
    pub use_disk: &'static str,
    pub keep_mine: &'static str,
    pub compare: &'static str,
    pub disk_version: &'static str,
    pub my_version: &'static str,
    pub close: &'static str,
    pub conflict_copies: &'static str,
    pub reloaded: &'static str,
    pub legacy_encoding: &'static str,
    pub write: &'static str,
    pub preview: &'static str,
    /// Short dates in lists, as `chrono` formats them.
    pub short_date: &'static str,
    pub months: [&'static str; 12],
    pub weekdays: [&'static str; 7],
    pub weekdays_long: [&'static str; 7],
}

pub static IT: Strings = Strings {
    file: "File",
    open_diary: "Apri cartella del diario…",
    use_suggested: "Usa questa cartella",
    choose_other: "Scegli un'altra cartella…",
    open_recent: "Diari recenti",
    close_diary: "Chiudi diario",
    quit: "Esci",
    language: "Lingua",
    theme: "Tema",
    theme_system: "Come il sistema",
    theme_light: "Chiaro",
    theme_dark: "Scuro",
    week_start: "Primo giorno della settimana",
    week_start_hint: "Vale per questo diario, su ogni computer che lo apre",
    monday: "Lunedì",
    sunday: "Domenica",
    today: "Oggi",
    search_hint: "Cerca nel diario",
    capture_hint: "Pensiero veloce",
    capture_tip: "Invio lo aggiunge alla speed list di oggi. «spunto: testo» dà lo spunto (da una a tre parole), una @parola finale il soggetto.",
    captured: "Aggiunto alla speed list di oggi, numero",
    capture_locked: "Prima risolvi il conflitto di oggi: il pensiero è rimasto nel campo",
    no_results: "Nessuna voce contiene queste parole.",
    recent_entries: "Ultime voci",
    results: "Risultati",
    empty_title: "Nessun diario aperto",
    empty_body: "Un diario è una cartella: ogni giorno è un file di testo Markdown in anno/mese. DiaryCraft propone la cartella qui sotto; puoi sceglierne un'altra, vuota per cominciare o quella di un diario che hai già. Può stare in una cartella sincronizzata (OneDrive, Google Drive, Dropbox, Syncthing) per usarlo da più computer.",
    saved: "Salvato",
    pending: "Modifiche non ancora salvate",
    pending_manual: "Modifiche non salvate: premi Salva",
    autosave: "Salvataggio automatico",
    autosave_hint: "Acceso: la voce si salva un secondo dopo che smetti di scrivere. Spento: si salva solo con Salva (Ctrl+S, ⌘S sul Mac); le modifiche restano in memoria anche cambiando giorno, e se chiudi senza salvare DiaryCraft lo chiede.",
    save: "Salva",
    discard: "Non salvare",
    cancel: "Annulla",
    unsaved_title: "Modifiche non salvate",
    unsaved_body: "Queste voci hanno modifiche non salvate:",
    words: "parole",
    attach: "Allega file…",
    attachments: "Allegati",
    open_folder: "Apri la cartella",
    conflict_title: "Questa voce è stata cambiata altrove",
    conflict_body: "Mentre la scrivevi, il file è stato cambiato da un altro computer o programma. DiaryCraft non l'ha sovrascritto: il tuo testo è salvato in",
    use_disk: "Usa la versione sul disco",
    keep_mine: "Tieni la mia",
    compare: "Confronta",
    disk_version: "Sul disco",
    my_version: "La mia",
    close: "Chiudi",
    conflict_copies: "copie di conflitto nel diario",
    reloaded: "Voce ricaricata: era stata cambiata altrove",
    legacy_encoding: "File in codifica Windows-1252: alla prossima modifica sarà salvato in UTF-8.",
    write: "Scrivi",
    preview: "Anteprima",
    short_date: "%d/%m/%Y",
    months: [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ],
    weekdays: ["L", "M", "M", "G", "V", "S", "D"],
    weekdays_long: ["lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato", "domenica"],
};

pub static EN: Strings = Strings {
    file: "File",
    open_diary: "Open diary folder…",
    use_suggested: "Use this folder",
    choose_other: "Choose another folder…",
    open_recent: "Recent diaries",
    close_diary: "Close diary",
    quit: "Quit",
    language: "Language",
    theme: "Theme",
    theme_system: "Same as the system",
    theme_light: "Light",
    theme_dark: "Dark",
    week_start: "First day of the week",
    week_start_hint: "For this diary, on every computer that opens it",
    monday: "Monday",
    sunday: "Sunday",
    today: "Today",
    search_hint: "Search the diary",
    capture_hint: "Quick thought",
    capture_tip: "Enter adds it to today's speed list. \"hint: text\" gives the hint (one to three words), a final @word the subject.",
    captured: "Added to today's speed list, number",
    capture_locked: "Settle today's conflict first: the thought is still in the field",
    no_results: "No entry contains these words.",
    recent_entries: "Latest entries",
    results: "Results",
    empty_title: "No diary open",
    empty_body: "A diary is a folder: each day is a Markdown text file under year/month. DiaryCraft suggests the folder below; you can choose another one, empty to start or the folder of a diary you already have. It can be in a synced folder (OneDrive, Google Drive, Dropbox, Syncthing) to use it from several computers.",
    saved: "Saved",
    pending: "Changes not saved yet",
    pending_manual: "Changes not saved: press Save",
    autosave: "Autosave",
    autosave_hint: "On: the entry is saved one second after you stop typing. Off: it is saved only with Save (Ctrl+S, ⌘S on a Mac); changes stay in memory when you move to another day, and DiaryCraft asks before closing without saving.",
    save: "Save",
    discard: "Don't save",
    cancel: "Cancel",
    unsaved_title: "Unsaved changes",
    unsaved_body: "These entries have unsaved changes:",
    words: "words",
    attach: "Attach file…",
    attachments: "Attachments",
    open_folder: "Open the folder",
    conflict_title: "This entry was changed elsewhere",
    conflict_body: "While you were writing, the file was changed by another computer or program. DiaryCraft did not overwrite it: your text is saved in",
    use_disk: "Use the version on disk",
    keep_mine: "Keep mine",
    compare: "Compare",
    disk_version: "On disk",
    my_version: "Mine",
    close: "Close",
    conflict_copies: "conflict copies in the diary",
    reloaded: "Entry reloaded: it was changed elsewhere",
    legacy_encoding: "File in Windows-1252 encoding: it will be saved in UTF-8 at the next change.",
    write: "Write",
    preview: "Preview",
    short_date: "%Y-%m-%d",
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    weekdays: ["M", "T", "W", "T", "F", "S", "S"],
    weekdays_long: ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
};

impl Strings {
    pub fn month(&self, m: u32) -> &'static str {
        self.months.get(m.saturating_sub(1) as usize).copied().unwrap_or("")
    }

    /// "martedì 24 novembre 2026" / "Tuesday 24 November 2026".
    pub fn long_date(&self, date: chrono::NaiveDate) -> String {
        use chrono::Datelike;
        let wd = self.weekdays_long.get(date.weekday().num_days_from_monday() as usize).copied().unwrap_or("");
        format!("{wd} {} {} {}", date.day(), self.month(date.month()), date.year())
    }
}
