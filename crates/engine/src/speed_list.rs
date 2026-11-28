//! The speed list: thoughts caught in a second, numbered in a section of the day's entry.
//!
//! The section is plain Markdown, so the entry stays readable anywhere:
//!
//! ```markdown
//! ## Speed list
//!
//! 1. **new house** call the agency about the visit
//! 2. **book** reread the chapter on maps @writing
//! ```

/// Heading text of the section, matched ignoring case and heading level.
pub const HEADING: &str = "Speed list";

/// Longest hint: one to three words.
pub const MAX_HINT_WORDS: usize = 3;

/// A thought to add, or one read back from an entry.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Thought {
    pub hint: Option<String>,
    pub text: String,
    pub subject: Option<String>,
}

/// A thought of an entry with its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub number: u64,
    pub thought: Thought,
}

impl Thought {
    /// Reads what was typed in the quick field: an optional `hint:` of one to three words at the
    /// start, an optional `@subject` as the last word. `None` when nothing is left to write.
    pub fn parse(typed: &str) -> Option<Self> {
        let mut rest = one_line(typed);
        let mut hint = None;
        if let Some((head, tail)) = rest.split_once(':') {
            let words = head.split_whitespace().count();
            // A colon in a web address or a time is not a hint.
            if (1..=MAX_HINT_WORDS).contains(&words)
                && !tail.starts_with("//")
                && !tail.starts_with(|c: char| c.is_ascii_digit())
            {
                hint = Some(head.trim().to_owned());
                rest = tail.trim().to_owned();
            }
        }
        let mut subject = None;
        if let Some((before, last)) = rest.rsplit_once(char::is_whitespace)
            && let Some(name) = last.strip_prefix('@')
            && !name.is_empty()
        {
            subject = Some(name.to_owned());
            rest = before.trim_end().to_owned();
        }
        let thought = Self { hint, text: rest, subject };
        thought.cleaned()
    }

    /// The thought on one line each, with blanks dropped; `None` when there is no text.
    pub fn cleaned(&self) -> Option<Self> {
        let text = one_line(&self.text);
        if text.is_empty() {
            return None;
        }
        let word = |s: &Option<String>, max: usize| {
            s.as_deref().map(one_line).filter(|s| !s.is_empty()).map(|s| {
                let words: Vec<&str> = s.split_whitespace().take(max).collect();
                words.join(" ")
            })
        };
        // A subject is one word, so that `@subject` reads back as it was written.
        let subject = word(&self.subject, 1).map(|s| s.trim_start_matches('@').to_owned()).filter(|s| !s.is_empty());
        Some(Self { hint: word(&self.hint, MAX_HINT_WORDS), text, subject })
    }

    fn line(&self, number: u64) -> String {
        let mut line = format!("{number}. ");
        match &self.hint {
            Some(hint) => line.push_str(&format!("**{}** ", hint.replace('*', ""))),
            // Text that would read back as a hint is escaped; Markdown shows the stars as typed.
            None if self.text.starts_with("**") => line.push('\\'),
            None => {}
        }
        match self.text.rsplit_once(' ') {
            // Likewise a last word that would read back as the subject.
            Some((before, last)) if last.starts_with('@') && last.len() > 1 => {
                line.push_str(&format!("{before} \\{last}"));
            }
            _ => line.push_str(&self.text),
        }
        if let Some(subject) = &self.subject {
            line.push_str(&format!(" @{subject}"));
        }
        line
    }
}

/// Text on one line, with runs of blanks made one space.
fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Level of a Markdown heading line, and its text.
fn heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    // Indented four spaces or more, it is code.
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..).unwrap_or_default();
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim()))
}

/// Fenced code blocks as CommonMark reads them: a fence of three or more backticks or tildes,
/// indented at most three spaces, closed only by a fence of the same character, at least as long,
/// with nothing after it.
#[derive(Debug, Default)]
struct Fences {
    open: Option<(char, usize)>,
}

impl Fences {
    /// Whether the line is code: inside a block, or one of its fences.
    fn code(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start_matches(' ');
        let fence = (line.len() - trimmed.len() <= 3)
            .then(|| trimmed.chars().next().filter(|c| matches!(c, '`' | '~')))
            .flatten()
            .map(|c| (c, trimmed.chars().take_while(|&x| x == c).count()))
            .filter(|&(_, n)| n >= 3);
        match (self.open, fence) {
            (None, Some((c, n))) => {
                // A backtick fence's info string may not hold a backtick.
                let info = trimmed.get(n..).unwrap_or_default();
                if c == '~' || !info.contains('`') {
                    self.open = Some((c, n));
                    return true;
                }
                false
            }
            (Some((c, n)), Some((d, m)))
                if c == d && m >= n && trimmed.get(m..).is_some_and(|r| r.trim().is_empty()) =>
            {
                self.open = None;
                true
            }
            (Some(_), _) => true,
            (None, None) => false,
        }
    }
}

/// Byte ranges of the section: from the line after its heading to the next heading of the same
/// level or higher, or the end. The first such section counts.
fn section(body: &str) -> Option<(usize, usize)> {
    let mut offset = 0;
    let mut start = None;
    let mut fences = Fences::default();
    for line in body.split_inclusive('\n') {
        if !fences.code(line)
            && let Some((level, text)) = heading(line)
        {
            match start {
                Some((lvl, from)) if level <= lvl => return Some((from, offset)),
                None if text.eq_ignore_ascii_case(HEADING) => start = Some((level, offset + line.len())),
                _ => {}
            }
        }
        offset += line.len();
    }
    start.map(|(_, from)| (from, body.len()))
}

/// The numbered list lines of a section, code blocks left out.
fn list_lines(inside: &str) -> impl Iterator<Item = (u64, &str)> {
    let mut fences = Fences::default();
    inside.lines().filter(move |l| !fences.code(l)).filter_map(numbered)
}

/// Number of a list line `N. …`, and what follows.
fn numbered(line: &str) -> Option<(u64, &str)> {
    let t = line.trim_start();
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = t.get(digits..)?.strip_prefix(['.', ')'])?;
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((t.get(..digits)?.parse().ok()?, rest.trim()))
}

/// The thoughts of an entry, in the order written.
pub fn items(text: &str) -> Vec<Item> {
    let (_, body) = crate::text::split_front_matter(text);
    let Some((from, to)) = section(body) else { return Vec::new() };
    list_lines(body.get(from..to).unwrap_or_default())
        .map(|(number, rest)| Item { number, thought: read_line(rest) })
        .collect()
}

fn read_line(rest: &str) -> Thought {
    let mut text = rest.to_owned();
    let mut hint = None;
    if let Some(escaped) = rest.strip_prefix("\\**") {
        text = format!("**{escaped}");
    } else if let Some(after) = rest.strip_prefix("**")
        && let Some((h, tail)) = after.split_once("**")
        && !h.trim().is_empty()
    {
        hint = Some(h.trim().to_owned());
        text = tail.trim().to_owned();
    }
    let mut subject = None;
    if let Some((before, last)) = text.rsplit_once(' ')
        && let Some(name) = last.strip_prefix('@')
        && !name.is_empty()
    {
        subject = Some(name.to_owned());
        text = before.trim_end().to_owned();
    }
    if let Some((before, last)) = text.rsplit_once(' ')
        && let Some(name) = last.strip_prefix("\\@")
    {
        text = format!("{before} @{name}");
    }
    Thought { hint, text, subject }
}

/// Adds a thought to the speed list of an entry, creating the section at the end if there is
/// none. Returns the new text and the thought's number; `None` when the thought has no text.
/// Everything outside the new line stays as it was.
pub fn add(text: &str, thought: &Thought) -> Option<(String, u64)> {
    let thought = thought.cleaned()?;
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let body = crate::text::split_front_matter(text).1;
    let head_len = text.len() - body.len();
    let Some((from, to)) = section(body) else {
        // At the end, after one blank line: only the line breaks that are missing are added.
        let sep = if text.trim().is_empty() || text.ends_with("\n\n") || text.ends_with("\n\r\n") {
            ""
        } else if text.ends_with('\n') {
            nl
        } else {
            "\n\n"
        };
        let sep = if sep == "\n\n" { format!("{nl}{nl}") } else { sep.to_owned() };
        return Some((format!("{text}{sep}## {HEADING}{nl}{nl}{}{nl}", thought.line(1)), 1));
    };
    let inside = body.get(from..to).unwrap_or_default();
    let number = list_lines(inside).map(|(n, _)| n).max().unwrap_or(0).saturating_add(1);
    let line = thought.line(number);
    // The end of the last line of the section with some text, before its line break.
    let mut last = None;
    let mut offset = 0;
    for l in inside.split_inclusive('\n') {
        if !l.trim().is_empty() {
            last = Some(offset + l.trim_end_matches(['\n', '\r']).len());
        }
        offset += l.len();
    }
    let Some(end) = last else {
        // The heading alone: the thought one blank line under it, and one before what follows.
        let at = head_len + from;
        let mut out = text.get(..at).unwrap_or_default().to_owned();
        if !out.ends_with('\n') {
            out.push_str(nl);
        }
        out.push_str(nl);
        out.push_str(&line);
        out.push_str(nl);
        let rest = text.get(at..).unwrap_or_default();
        if rest.lines().next().is_some_and(|l| !l.trim().is_empty()) {
            out.push_str(nl);
        }
        out.push_str(rest);
        return Some((out, number));
    };
    let at = head_len + from + end;
    let rest = text.get(at..).unwrap_or_default();
    let mut out = format!("{}{nl}{line}{rest}", text.get(..at).unwrap_or_default());
    if rest.is_empty() {
        out.push_str(nl);
    }
    Some((out, number))
}
