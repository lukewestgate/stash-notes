# Stash Notes

A keyboard-first, always-resident notepad for work on Windows. One global hotkey (`Alt+N`) brings it up instantly. You read or add notes for the ticket you're on, and one keypress (`Esc`) sends you back to your code.

Notes are plain markdown files in a folder you own. There's no account, no sync and no database. Stash Notes is built with [Tauri v2](https://v2.tauri.app/) (Rust) and a Svelte + CodeMirror UI.

- **Instant summon.** The window is created once at login and only ever shown or hidden.
- **Opens where you left off,** or on today's daily note.
- **Ticket notes:** one note per Jira key (`PROJ-123`), with Jira and Gerrit links a keystroke away.
- **Links between notes** (`[[Rollout plan]]`, bare `PROJ-123`, `[[2026-10-22]]`) with browser-style back and forward.
- **One fuzzy switcher** (`Ctrl+K`) finds notes by ticket key, title, date or text, and creates new ones.
- **Fully usable without the mouse.** Press `Ctrl+/` to see every shortcut.

---

## Contents

- [Installation](#installation)
- [Getting started](#getting-started)
- [Using Stash Notes](#using-stash-notes)
  - [Note kinds and the notes folder](#note-kinds-and-the-notes-folder)
  - [Keyboard shortcuts](#keyboard-shortcuts)
  - [Quick switcher and search](#quick-switcher-and-search)
  - [Links, backlinks and history](#links-backlinks-and-history)
  - [Jira and Gerrit](#jira-and-gerrit)
  - [Daily notes and todos](#daily-notes-and-todos)
  - [Archiving and renaming](#archiving-and-renaming)
  - [Window, tray and startup](#window-tray-and-startup)
- [Settings](#settings)
- [Your data and safety](#your-data-and-safety)
- [Building from source](#building-from-source)
- [Development](#development)
- [Troubleshooting](#troubleshooting)
- [Design notes and decisions](#design-notes-and-decisions)

---

## Installation

Stash Notes targets **Windows 10 and Windows 11** (64-bit). It needs the Microsoft Edge **WebView2** runtime, which ships with Windows 11 and current Windows 10. If WebView2 is missing, the installer downloads it.

### Option 1: download a build from GitHub (no tools needed)

Every push to this repository builds the Windows installers with GitHub Actions.

1. Open the repository on GitHub and go to the **Actions** tab.
2. Click the latest successful **Build** run, preferably on the `main` branch or the branch you want to try.
3. Under **Artifacts**, download one of these:
   - **`stash-notes-windows`** has the installers: `Stash Notes_x.y.z_x64-setup.exe` (NSIS, recommended, installs for the current user without admin rights) and `Stash Notes_x.y.z_x64_en-US.msi`.
   - **`stash-notes-portable-exe`** is a single `stash-notes.exe` you can run without installing.
4. Unzip the download and run the installer (or the exe).

**No GitHub login?** Artifacts need a login, but releases don't. Every build of the default branch also publishes the installers and the portable exe to the [**Releases**](https://github.com/lukewestgate/stash-notes/releases/latest) page, under the version in `src-tauri/tauri.conf.json` (for example `v0.1.0`). Pushing a tag such as `v0.2.0` publishes that tag too.

> **Windows SmartScreen.** The builds are not code-signed, so Windows may show *"Windows protected your PC"*. Click **More info → Run anyway**. Code signing needs a certificate, which this project doesn't have yet.

### Option 2: build it yourself

See [Building from source](#building-from-source).

### Uninstalling

Use **Settings → Apps → Installed apps → Stash Notes → Uninstall**. Your notes are never touched. They stay in your notes folder. Settings are in `%APPDATA%\com.lukewestgate.stashnotes\` and the web view's cache is in `%LOCALAPPDATA%\com.lukewestgate.stashnotes\` if you want to remove those too.

---

## Getting started

1. **Launch Stash Notes** from the Start menu. The window opens on today's daily note. A tray icon (blue note) appears in the notification area.
2. **Type something.** There's no save button. Notes autosave 300 ms after you stop typing, and again whenever the window hides.
3. **Press `Esc`.** The window hides and focus goes back to the app you were using.
4. **Press `Alt+N`** from anywhere. Stash Notes is back with the cursor where you left it. `Alt+N` again hides it.
5. **Press `Ctrl+K`** and type a ticket key like `PROJ-123`, then `Enter`. That creates (or opens) the ticket's note.
6. **Press `Ctrl+;`** to insert a timestamp such as `2026-10-22 14:05 `, then type your note.
7. **Press `Ctrl+J`** to open the ticket in Jira. Stash Notes hides itself so the browser takes focus.
8. **Press `Ctrl+/`** for the full list of shortcuts. That screen is the whole manual in brief.

Before using Jira or Gerrit links, set your own URLs. Press `Ctrl+,` to open `settings.json` and change `jira.urlTemplate` and `gerrit.urlTemplate` (see [Settings](#settings)). The change takes effect as soon as you save the file.

Stash Notes starts automatically at login (to the tray, hidden) unless you turn `autostart` off.

### A typical flow

`Alt+N` → `Ctrl+K` → type `123` → `Enter` → `Ctrl+;` → type a note → `Ctrl+J` to look at the ticket. Opening Jira hides the app, so you're already back in the browser. Next time, `Alt+N` brings you right back to the same note.

---

## Using Stash Notes

### Note kinds and the notes folder

Every note is one markdown file in a single **notes folder** (the "vault"). The default is `Documents\StashNotes`. The folder a file is in tells Stash Notes what kind of note it is:

| Kind | Path | Title | Created |
|---|---|---|---|
| Daily | `daily/2026-10-22.md` | The date | On first open each day (`Ctrl+D`) |
| Ticket | `tickets/PROJ-123.md` | The ticket key | From the switcher, or by following `PROJ-123` / `[[PROJ-123]]` |
| Note | `notes/<slug>.md` | First `# heading`, else the file name | From the switcher, or by following a `[[link]]` |
| Archived | `archive/<original path>` | Same as before | `Ctrl+Shift+A` |

Markdown files anywhere else in the folder are treated as regular notes. Folders starting with `.` (such as `.git`) and the `templates` folder are ignored.

**Ticket keys** match the regex `[A-Z][A-Z0-9]+-\d+` by default (`PROJ-123`, `OPS-7`). You can change this with `ticketPattern`. A bare key anywhere in any note is highlighted and works as a link.

**Ticket note format.** Ticket notes start with a small front matter block. All fields are optional:

```markdown
---
title: Rollout date question          # shown next to the key in the header and switcher
jira: https://other.atlassian.net/browse/PROJ-123   # only to override jira.urlTemplate
gerrit:
  - 12345                             # bare change numbers expand via gerrit.urlTemplate
  - https://gerrit.example.com/c/proj/+/12399
---

- 2026-10-22 14:05 asked about rollout date, see [[Rollout plan]]
- 2026-10-22 16:40 fix pushed, waiting on review
```

Any note can also list `aliases: [Plan B, launch]`. Links and search then match those names too.

### Keyboard shortcuts

All in-app shortcuts can be changed in `settings.json` (`keys` section). In-app shortcuts take priority over the editor's own bindings.

| Shortcut | Action | Notes |
|---|---|---|
| `Alt+N` (global) | Show or hide the app | Works from any app (`hotkey` setting) |
| `Esc` | Close overlay, else hide window | Closes the switcher, picker, prompts and backlinks list first |
| `Ctrl+K` | Quick switcher and search | Also creates notes. With text selected, links the selection |
| `Ctrl+D` | Go to today's daily note | Creates it if missing |
| `Ctrl+Enter` | Follow link under cursor | Note link, ticket key or URL. On a checkbox line with no link, toggles the checkbox |
| `Alt+Left` | Back | Previous note in history |
| `Alt+Right` | Forward | Undoes a Back |
| `Ctrl+J` | Open Jira page | The current ticket, or the key under the cursor, or the first key in the note |
| `Ctrl+G` | Open Gerrit change | Shows a picker when there are several (`1`–`9` pick directly) |
| `Ctrl+Shift+G` | Add Gerrit link | Prefilled from the clipboard if it looks like a Gerrit URL or change number |
| `Ctrl+;` | Insert timestamp | `2026-10-22 14:05 ` |
| `Ctrl+Shift+B` | Expand or collapse backlinks | Use the arrow keys and `Enter` to open one |
| `Ctrl+Shift+A` | Archive or unarchive note | Doesn't ask first. The toast has **Undo** for 5 s |
| `Ctrl+/` | Shortcut help | |

These extras aren't counted in the 14 above:

| Shortcut | Action |
|---|---|
| `[[` | Link picker: type to filter, `Enter` inserts `[[Title]]` |
| `Ctrl+Click` | Follow a link, key or URL with the mouse |
| Click on `[ ]` | Toggle a checkbox |
| `F2` | Rename the current note (links to it are updated) |
| `Ctrl+,` | Open `settings.json` in your default editor |
| `Ctrl+S` | Save now (not needed, since autosave runs anyway) |
| `Ctrl+Shift+Q` | Quit (or use **Quit** in the tray menu) |

### Quick switcher and search

`Ctrl+K` opens one overlay that handles navigation, search and note creation.

- **Empty query** lists recently opened notes, newest first. Archived notes are hidden, so this is also your "open tickets" list.
- **Typing** updates the results on every keystroke. `↑`/`↓` move, `Enter` opens, `Esc` closes.
- **Create.** When nothing matches exactly, the last row reads `Create "<query>"`. A query that looks like a ticket key creates a ticket note. Anything else creates a regular note.
- **`Tab`** includes archived notes, which are shown dimmed with an `archived` tag.
- **`Alt+Enter`** *peeks* at a result without adding it to history. Back returns you to where you were.

What matches, from strongest to weakest:

| Field | Examples that match | Weight |
|---|---|---|
| Ticket key | `123`, `proj123`, `PROJ-123` | Highest |
| Title and aliases | `rollout`, `rlt pln` (fuzzy) | High |
| Date | `2026-10-22`, `oct 22`, `22 oct`, `today`, `yesterday`, `monday` | High (matches the daily note) |
| Body text | Any word or words from a note, or a `2026-10-22 14:05` timestamp | Normal, with the matching line shown |

Typing **only digits** (for example `123`) matches every ticket key ending in those digits, so you rarely need the project prefix. Notes you opened in the last 7 days get a boost, and archived notes get a small penalty.

Search is deliberately simple: there are no regexes, boolean operators or saved searches. For those, search the notes folder with Everything, ripgrep or your editor.

### Links, backlinks and history

| Written as | Opens | If the target is missing |
|---|---|---|
| `[[Rollout plan]]` | The note titled "Rollout plan" (case-insensitive) | Creates `notes/rollout-plan.md` |
| `[[PROJ-123]]` or bare `PROJ-123` | `tickets/PROJ-123.md` | Creates it with a front matter stub |
| `[[2026-10-22]]` | `daily/2026-10-22.md` | Creates it from the daily template |
| `https://...` | Your default browser | – |

`[[Target|shown text]]` and `[[Target#section]]` also work. Only the part before `|` or `#` is used to find the note.

**Resolution order:** exact file name, then title, then alias. If two notes share a title, the most recently edited one wins. Links to archived notes still work and are shown dimmed.

**Making links quickly:**

- Type `[[` to open the inline link picker.
- Select some text and press `Ctrl+K`, then pick a note. The selection becomes `[[Title|selected text]]`.
- Bare ticket keys need no brackets. Typing `PROJ-123` in a daily note already links to it and shows up in the ticket's backlinks.

**Backlinks.** The one-line strip at the bottom shows *"Mentioned in: …"*, listing every note that links to the current note or mentions its ticket key. `Ctrl+Shift+B` expands it into a list: use the arrows and `Enter` to open, `Esc` to collapse.

**History** works like a browser. Every note opened by a link, the switcher or `Ctrl+D` is pushed onto a stack. `Alt+Left` and `Alt+Right` move through it and restore each note's cursor and scroll position. Opening a new note after going back drops the forward entries. History survives hiding and showing the window, keeps the last 100 entries, and is cleared when you quit.

### Jira and Gerrit

- **`Ctrl+J`** builds the URL from `jira.urlTemplate` (for example `https://yourco.atlassian.net/browse/{key}`) and opens it. In a ticket note it uses that note's key, or the front matter `jira:` override. In any other note it uses the key under the cursor, else the first key in the note. If there's no key, a message says so.
- **`Ctrl+Shift+G`** asks for a Gerrit URL or change number and appends it to the note's `gerrit:` list. It creates the front matter if needed.
- **`Ctrl+G`** opens the note's only Gerrit link, or shows a picker if there are several. Bare numbers expand through `gerrit.urlTemplate` (for example `https://gerrit.example.com/c/{n}`).
- The header shows **Jira** and **Gerrit N** buttons only when they're relevant. Hover over them to see their shortcuts.
- After opening an external link, the app hides so the browser keeps focus. Set `hideAfterExternalLink` to `false` to turn this off.

Only `http://` and `https://` links can be opened.

### Daily notes and todos

`Ctrl+D` opens today's note (`daily/YYYY-MM-DD.md`), creating it from a template with a `## Todo` section. The cursor lands at the end of the todo list, ready to type.

- `Enter` at the end of a `- [ ] item` continues the list with a new checkbox.
- `Ctrl+Enter` on a checkbox line (or clicking the `[ ]`) toggles it. Done items are dimmed.
- Unchecked items don't roll over to the next day. They stay where you wrote them, and the switcher still finds them.
- With `onSummon: "last"`, if the last note you had open was a **previous** day's daily note, Stash Notes opens today's instead, so you don't land on yesterday's list.

**Custom template:** create `templates/daily.md` inside the notes folder. `{{date}}` (`2026-10-22`) and `{{weekday}}` (`Thursday`) are filled in. The default template is:

```markdown
# {{date}}

## Todo

- [ ] 

## Notes

```

### Archiving and renaming

- **`Ctrl+Shift+A`** moves the note into `archive/`, keeping its relative path (`tickets/PROJ-123.md` → `archive/tickets/PROJ-123.md`). Press it again on an archived note to move it back. Archived notes are left out of search unless you press `Tab` in the switcher, and links to them still work. A toast offers **Undo** for 5 seconds.
- **`F2`** renames a regular note. It changes the file name and the `# heading`, and rewrites every `[[link]]` to it across the notes folder. A toast reports how many links were updated and offers **Undo** for 5 seconds. Ticket and daily notes are named by their key or date, so they can't be renamed. For tickets, set `title:` in the front matter instead.
- Stash Notes **never deletes** files. To delete a note, delete the file in Explorer.

### Window, tray and startup

- **Login:** Stash Notes starts with Windows (`autostart`) and stays hidden in the tray, already loaded. Set `startHidden` to `false` to show the window at login. Starting it yourself from the Start menu always shows the window.
- **Global hotkey** (`Alt+N`): if the window is hidden, it shows and focuses the editor. If it's visible and focused, it hides. If it's visible but behind another window, it comes to the front.
- **Close (X), Minimize and `Esc`** all hide to the tray. The process only exits through **Quit** in the tray menu or `Ctrl+Shift+Q`.
- **Hiding gives focus back** to the window you were in before, usually your editor or terminal.
- **Tray icon:** left-click toggles the window. Right-click opens the menu: *Show, Today's note, Open notes folder, Settings, Quit*.
- **Single instance:** launching Stash Notes again just shows the running window.
- **Window size and position** are remembered. The default is a centered window of about 900 × 650.
- **Theme** follows the Windows light or dark setting. You can force one with `theme`.

---

## Settings

Settings live in a hand-editable JSON file:

```
%APPDATA%\com.lukewestgate.stashnotes\settings.json
```

Press `Ctrl+,` (or use **Settings** in the tray menu) to open it in your default editor. Notepad is used if `.json` has no associated app. Changes are picked up as soon as you save, with no restart needed. Every key is optional, and anything you leave out uses its default. If the file has a JSON error, a message tells you so. The previous settings stay in effect while the app is running, and the defaults are used if it starts with a broken file.

| Key | Default | Purpose |
|---|---|---|
| `vaultPath` | `Documents\StashNotes` | The notes folder. `~` means your user folder. |
| `hotkey` | `Alt+N` | Global show/hide hotkey (for example `Ctrl+Alt+N` or `Ctrl+Alt+Space`) |
| `onSummon` | `last` | `last`: the note you were in. `daily`: always today's note |
| `startHidden` | `true` | When started at login, stay in the tray |
| `autostart` | `true` | Start with Windows |
| `showInTaskbar` | `false` | Keep a taskbar button while the window is shown |
| `alwaysOnTop` | `false` | Float above other windows while visible |
| `hideAfterExternalLink` | `true` | Hide after opening Jira, Gerrit or any URL |
| `ticketPattern` | `[A-Z][A-Z0-9]+-\d+` | Regex for ticket keys. In JSON, escape the backslash: `"[A-Z][A-Z0-9]+-\\d+"` |
| `jira.urlTemplate` | `https://yourco.atlassian.net/browse/{key}` | Jira link builder |
| `gerrit.urlTemplate` | `https://gerrit.example.com/c/{n}` | Gerrit link builder for bare change numbers |
| `theme` | `system` | `system`, `light` or `dark` |
| `fontSize` | `14` | Editor and UI font size in px |
| `keys.*` | see [shortcuts](#keyboard-shortcuts) | In-app shortcuts: `hide`, `quickSwitcher`, `dailyNote`, `followLink`, `back`, `forward`, `openJira`, `openGerrit`, `addGerrit`, `timestamp`, `backlinks`, `archive`, `help`, `quit`, `rename`, `openSettings` |

Example:

```json
{
  "vaultPath": "C:\\Users\\me\\OneDrive\\Notes",
  "hotkey": "Ctrl+Alt+N",
  "ticketPattern": "(?:PROJ|OPS|INFRA)-\\d+",
  "jira": { "urlTemplate": "https://mycompany.atlassian.net/browse/{key}" },
  "gerrit": { "urlTemplate": "https://review.mycompany.com/c/{n}" },
  "keys": { "rename": "Ctrl+R" }
}
```

Shortcut strings combine `Ctrl`, `Alt`, `Shift` and `Win`/`Super`/`Meta` with a key: a letter, digit, symbol (`;`, `/`, `,`), `Enter`, `Esc`, `Tab`, `Space`, `Left`/`Right`/`Up`/`Down`, `Home`, `End` or `F1`–`F12`. Changing `vaultPath` or `ticketPattern` re-indexes the notes folder right away.

The same folder also holds `session.json` (last open note, cursor position and the recently opened list) and the saved window position. Both are safe to delete.

---

## Your data and safety

- **Plain files are the source of truth.** Notes are ordinary UTF-8 markdown files. You can edit them in any editor while Stash Notes is running, put the folder in Git, OneDrive or Dropbox, and back it up any way you like. Stash Notes keeps only an in-memory search index, rebuilt at every start, so there's no database to corrupt.
- **Autosave** runs 300 ms after the last keystroke, when the window hides, before navigating, and on quit.
- **Crash-safe writes:** each save goes to a temporary file that is then renamed over the original, so a crash or power cut can't leave a half-written note. If another program holds the file open (for example a sync client), it retries briefly and then writes in place.
- **External changes:** if a note changes on disk while you have no unsaved edits, it reloads silently. If you *do* have unsaved edits, your text is kept and a bar offers **Reload from disk** or **Keep mine**.
- **Nothing is deleted.** Archiving moves files. Renaming moves the file and rewrites links, and **Undo** puts everything back.
- **Locked-down UI:** the web UI has no direct file system access. Every read and write goes through a small set of Rust commands that only accept paths inside the notes folder. Only `http`/`https` URLs can be opened.

---

## Building from source

### Prerequisites (Windows)

1. **Microsoft C++ Build Tools.** Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **"Desktop development with C++"** workload.
2. **Rust** from [rustup.rs](https://rustup.rs/) (stable, MSVC toolchain, which is the default on Windows).
3. **Node.js 20 or newer** (22 LTS recommended) from [nodejs.org](https://nodejs.org/).
4. **WebView2**: already installed on Windows 11 and current Windows 10.

These are the standard [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

### Build

```powershell
git clone https://github.com/lukewestgate/stash-notes.git
cd stash-notes
npm ci
npm run tauri build
```

The output is written to `src-tauri\target\release\`:

- `stash-notes.exe` is the app itself, which you can run directly.
- `bundle\nsis\Stash Notes_0.1.0_x64-setup.exe` is the installer (per-user, no admin rights).
- `bundle\msi\Stash Notes_0.1.0_x64_en-US.msi` is the MSI installer.

### Run in development mode

```powershell
npm run tauri dev
```

This starts the Vite dev server with hot reload and a debug build of the app. Debug builds never register themselves for autostart.

### Linux and macOS

The app is designed for Windows, but it also builds and runs on Linux (it is tested there in CI). On Debian/Ubuntu, first install `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`. macOS should work but isn't tested.

---

## Development

### Project layout

```
stash-notes/
├── src/                      Svelte + TypeScript UI
│   ├── App.svelte            App state, navigation, saving, keyboard handling
│   ├── components/           Header, Switcher, Backlinks, Help, Prompt, Picker
│   └── lib/
│       ├── api.ts            Typed wrappers for the Rust commands and events
│       ├── editor.ts         CodeMirror setup: decorations, [[ link picker
│       ├── text.ts           Link/key detection, checkboxes, timestamps, URLs
│       ├── keys.ts           Shortcut parsing and matching
│       ├── history.ts        Back/forward history
│       └── mock.ts           In-memory backend for running the UI in a browser
├── src-tauri/                Rust (Tauri v2) app
│   ├── src/
│   │   ├── lib.rs            App setup, plugins, window events
│   │   ├── commands.rs       Commands the UI calls (open/save/search/...)
│   │   ├── config.rs         settings.json, indexing, file watching
│   │   ├── window.rs         Show/hide/focus (incl. the Windows foreground fix)
│   │   ├── tray.rs           Tray icon and menu
│   │   ├── hotkey.rs         Global hotkey
│   │   └── state.rs          Shared state and session persistence
│   ├── core/                 `stash-core`: vault, index, search, parsing
│   │   ├── src/              (no Tauri dependency; unit tested)
│   │   └── tests/perf.rs     5,000-note performance check
│   ├── capabilities/         Webview permissions (core events only)
│   └── tauri.conf.json       Window, bundle and security config
└── .github/workflows/        CI: tests on Linux, installers on Windows
```

**Architecture.** Rust owns the files, the search index and the window. The web UI only draws and sends commands. At startup, Rust scans the notes folder into an in-memory index in a background thread, then watches the folder (and `settings.json`) for changes. The UI talks to Rust through these commands: `open_note`, `save_note`, `create_note`, `archive_note`, `rename_note`, `search`, `backlinks`, `get_settings`, `set_settings` and a few helpers. Rust pushes events back: `vault-changed`, `settings-changed`, `summoned`, `toast`. Fuzzy matching uses the [`nucleo`](https://github.com/helix-editor/nucleo) matcher, the engine behind the Helix editor's picker.

### Useful commands

```bash
npm run dev            # UI only, in a browser, backed by an in-memory mock (http://localhost:1420)
npm run tauri dev      # the real app with hot reload
npm run check          # Svelte/TypeScript type check
npm test               # UI unit tests (Vitest)
cd src-tauri && cargo test --workspace                     # Rust unit tests
cd src-tauri && cargo test --release -p stash-core --test perf -- --ignored --nocapture
                                                           # performance check on 5,000 synthetic notes
```

`npm run dev` on its own is handy for UI work. It fills in sample notes, and nothing is saved.

### Continuous integration

`.github/workflows/build.yml` runs on every push and pull request:

- **Tests (Linux):** type check, UI tests, UI build, Rust tests, and the performance check (informational).
- **Windows installer:** `npm run tauri build`, then uploads the installers and the portable exe as artifacts.
- **Release:** on pushes to the default branch, and on tags matching `v*`, attaches the installers and the portable exe to a GitHub Release. Branch builds publish to `v<version>` from `tauri.conf.json`, and later builds of the same version replace its files.

To publish a new version, bump `version` in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` and push (or push a tag such as `v0.2.0`).

---

## Troubleshooting

**`Alt+N` does nothing.** Another app may already be using that hotkey. When this happens, Stash Notes shows a message on startup. Pick a different `hotkey` in `settings.json`, such as `Ctrl+Alt+N`. It is re-registered as soon as you save the file.

**The window appears but typing goes to the previous app.** Windows sometimes refuses to give focus to a window brought up by a background process. Stash Notes works around this by briefly attaching to the foreground thread's input queue. If it still happens, press the hotkey once more (a visible but unfocused window gets focused), and please open an issue describing your setup.

**Jira/Gerrit open the wrong site.** Set `jira.urlTemplate` and `gerrit.urlTemplate` in `settings.json`.

**My ticket keys aren't highlighted.** Keys must match `ticketPattern`, which is upper-case letters, a dash and digits by default. Lower-case keys typed in the switcher are upper-cased for you.

**Where are my notes?** Use **Open notes folder** in the tray menu. The path is also shown at the bottom of the `Ctrl+/` help screen.

**I want to start fresh.** Quit the app, then delete `%APPDATA%\com.lukewestgate.stashnotes`. That resets settings, the session and the window position. Your notes folder is never touched.

---

## Design notes and decisions

Stash Notes implements the *Quick Notes (Tauri) — Product & Technical Spec*. Where the spec left open questions or room for interpretation, these choices were made. All of them can be changed in settings where noted.

- **Name and folder.** The app is called *Stash Notes*, and its default notes folder is `Documents\StashNotes` (the spec's working name was *Quick Notes*).
- **Jira and Gerrit URLs** default to placeholders (`yourco.atlassian.net`, `gerrit.example.com`). Set your real ones in `settings.json`.
- **Default hotkey** is `Alt+N`. If it's taken on your machine, change `hotkey`.
- **Daily todos don't roll over.** Unchecked items stay where they were written, as the spec specifies for v1.
- **Windows 10 and 11** are both supported. The installer bootstraps WebView2 if it's missing.
- **The notes folder lives in Documents** by default. Syncing it with OneDrive or Git works because saves are atomic and external changes are picked up live. Point `vaultPath` elsewhere if you prefer.
- **Shortcuts beyond the spec's 14:** `F2` rename (the spec describes renaming but assigns no key), `Ctrl+,` open settings (suggested in the spec), `Ctrl+S` save now, and `Ctrl+Click` to follow links.
- **Peek (`Alt+Enter`)** shows the chosen note without adding it to history. `Alt+Left` returns to the note you were on, and forward history is kept.
- **Link picker** is CodeMirror autocomplete on `[[`. It uses the same search as the switcher. When two notes share a title, both are listed with their paths.
- **Links to archived notes** are dimmed in the editor. Search shows archived notes only after pressing `Tab`.
- **The editor font** is Segoe UI with tabular digits, so timestamps line up in a neat column. Inline code uses Cascadia Mono.
