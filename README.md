# DiaryCraft

DiaryCraft is a diary that keeps each day as a plain text file. You choose a folder; every entry is a Markdown file under `year/month/` named after its day (`2026/11/2026-11-24.md`), so the diary stays readable and editable with any text editor, today and in twenty years, with or without DiaryCraft.

It is a local desktop app written in Rust with egui. The folder can be on this computer, on a shared drive, or in a folder synced by OneDrive, Google Drive, Dropbox or Syncthing to write from several computers.

> **Status:** 0.3, in development. The plan is in [docs/plan.md](docs/plan.md) (in Italian).

## Install

Each [release](https://github.com/mmazzieri/DiaryCraft/releases) has these files, listed with their checksums in `SHA256SUMS.txt`:

| System | File |
|---|---|
| Windows 10 and 11, x64 | `diarycraft-<version>-windows-x64.msi` (installer) or `-windows-x64-portable.zip` |
| macOS 11 or newer, Intel and Apple silicon | `diarycraft-<version>-macos-universal.dmg` |
| Debian, Ubuntu and derivatives | `diarycraft-<version>-linux-<arch>.deb` |
| Fedora, openSUSE, RHEL | `diarycraft-<version>-linux-<arch>.rpm` |
| Any Linux with glibc 2.35 or newer | `diarycraft-<version>-linux-<arch>.tar.gz` |

`<arch>` is `x86_64` or `aarch64`. The MSI installs for the current user or for every user of the computer and adds DiaryCraft to the Start menu; the Linux packages add it to the applications menu. The Windows files are not signed yet, so SmartScreen may ask for a confirmation the first time.

On macOS, drag DiaryCraft to Applications. Until a release is signed with a Developer ID and notarized, macOS blocks it the first time because it cannot check it. On macOS 15 or newer, open it once, close the warning, then go to **System Settings › Privacy & Security** and choose **Open Anyway** next to DiaryCraft; on older versions, right-click it, choose **Open** and confirm. From Terminal, `xattr -dr com.apple.quarantine /Applications/DiaryCraft.app` does the same.

## Using it

- At the first start DiaryCraft suggests a `DiaryCraft` folder in your Documents folder (on Windows the Documents known folder, also when OneDrive redirects it); **Use this folder** creates it, **Choose another folder…** picks any other. **File › Open diary folder…** opens another diary later. An empty folder starts a new one; DiaryCraft adds a small `.diarycraft/config.json` with the format version and nothing else.
- The calendar on the left shows the month, with the days that have an entry highlighted. Click a day to read or write it; **Today** goes back to today.
- Under the calendar are the latest entries, or the results of the search box at the top: every word typed must appear in the entry, ignoring case.
- What you write is saved one second after you stop typing, when you move to another day and when you close the app. A day with nothing written creates no file. Untick **Autosave** at the top right to save only with **Save** (Ctrl+S, ⌘S on a Mac): changes stay in memory while the app is open, also when you move to another day (the calendar shows those days in italics), and closing the app, the diary or opening another one asks whether to save them.
- **Quick thought**, the field at the top, catches a thought without leaving what you are doing: type it and press Enter, and it goes into today's speed list, a numbered `## Speed list` section of today's entry (added at the end the first time). `house: call the agency` gives it a hint of one to three words, shown in bold; a final `@writing` gives it a subject. The field stays ready for the next thought.
- **Attach file…** copies files into the day's attachments folder (`2026/11/2026-11-24/`) and adds a link to them in the text.
- The editor colours the Markdown as you type (headings, bold, italic, code, links, lists, quotes, the front matter). **Preview** shows the entry formatted, with working links to its attachments; **Write** goes back to the text.
- Entries from the 2014 LogBook (`year/month/DD.txt` or `year/month/YYYYMMDD.txt`, also in Windows-1252) are read as they are. The first time you change one, it becomes a Markdown entry: the file is renamed to `year/month/YYYY-MM-DD.md` and saved in UTF-8. Entries you don't change stay as they are.
- **File › First day of the week** starts the calendar's weeks on Monday or Sunday. The choice is kept in the diary folder (`.diarycraft/config.json`), so every computer that opens the diary shows the same calendar; `diarycraft-cli --diary <diary folder> settings --week-start sunday` does the same from the command line.
- Italian and English: the language of the system at first (on Windows and macOS the language of the user interface, elsewhere `LC_ALL`, `LC_MESSAGES` or `LANG`), or the one chosen in **File › Language**.

## Your text is never lost

Every save goes to a temporary file that is then renamed over the entry, so a crash or a full disk never leaves half a page. Before saving, DiaryCraft checks that the file on disk is still the one it read. If another computer or another program changed it meanwhile, DiaryCraft does not overwrite it: it saves your text beside it as `2026-11-24 (conflict <computer> <time>).md`, shows a notice, and lets you compare the two versions and keep either. If the entry on screen has no unsaved changes, a version changed elsewhere is simply reloaded. DiaryCraft never deletes files. If a sync client brings back the `.txt` of an entry that became Markdown on another computer, it is listed with the conflict copies.

## Command line and agents

Every package also has `diarycraft-cli`, which works on the diary without a window:

```
diarycraft-cli --diary ~/Documents/DiaryCraft append today "Lunch with friends."
diarycraft-cli --diary ~/Documents/DiaryCraft capture --hint house "call the agency"
diarycraft-cli --diary ~/Documents/DiaryCraft search lunch
diarycraft-cli --help
```

`--json` prints the whole answer; `DIARYCRAFT_DIARY` replaces `--diary`. An existing entry is replaced only with the version that `read --json` gave, so a change made meanwhile is not overwritten.

`diarycraft-cli mcp` offers the same commands to an agent over the Model Context Protocol, on standard input and output, with no network:

```json
{"mcpServers": {"diarycraft": {"command": "diarycraft-cli", "args": [
  "--diary", "/home/me/Documents/DiaryCraft", "mcp",
  "--automation-read-root", "/home/me/Downloads",
  "--automation-write-root", "/home/me/Documents/DiaryCraft"]}}}
```

Both folders are required. The diary must be inside the write folder, and files to attach come only from the two folders. Links that lead outside are refused, and so are hard links except on Windows, where the check is not available. The folders must not be writable by programs you do not trust: a folder swapped during a command is not detected.

## Building

```
cargo run --release -p diarycraft -- path/to/diary
cargo xtask ci   # fmt, clippy -D warnings, tests
```

The packages are built by `packaging/linux/package.sh`, `packaging/macos/package.sh` and `packaging/windows/package.ps1`, and by the Release workflow for every pull request that touches them; a tag `vX.Y.Z` opens a draft release with all of them. The app icon's sizes are written by `tools/icons.py` (Pillow) from `assets/app-icon/diarycraft-source.png`.

## License

MIT OR Apache-2.0, at your option. See [ATTRIBUTION.md](ATTRIBUTION.md) for third-party material.
