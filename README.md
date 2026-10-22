# Stash Notes

A keyboard-first, always-resident notepad for work on Windows. One global hotkey (`Alt+N`) brings it up instantly. You read or add notes for the ticket you're on, and one keypress (`Esc`) sends you back to your code.

Notes are plain markdown files in a folder you own. There's no account, no sync and no database. Stash Notes is built with [Tauri v2](https://v2.tauri.app/) (Rust) and a Svelte + CodeMirror UI.

- **Instant summon.** The window is created once at login and only ever shown or hidden.
- **Ticket notes with a status.** One note per Jira key (`PROJ-123`), marked todo, open, blocked, backlog or closed, with Jira and Gerrit links a keystroke away.
- **Live markdown.** Headings, bold, lists and checkboxes are styled as you type, and the file stays plain markdown.
- **Sidebar and Notes view.** See what you're working on at a glance, and scan, sort and filter every note.
- **Quick switcher pop-up** (`Alt+J`). Search, open or create a note from any app without bringing up the full window first.
- **Pinned notes.** Pop any note out into a small always-on-top window beside your code.
- **One fuzzy switcher** (`Ctrl+K`). Find notes by ticket key, title, date, text or `@status`, and create new ones.
- **Fully usable without the mouse.** Press `Ctrl+/` to see every shortcut.

---

## Contents

- [What's new in 0.4](#whats-new-in-04)
- [Installation](#installation)
- [Getting started](#getting-started)
- [Using Stash Notes](#using-stash-notes)
  - [The window](#the-window)
  - [Note kinds and the notes folder](#note-kinds-and-the-notes-folder)
  - [Statuses](#statuses)
  - [The editor](#the-editor)
  - [Keyboard shortcuts](#keyboard-shortcuts)
  - [Quick switcher and search](#quick-switcher-and-search)
  - [Sidebar](#sidebar)
  - [Notes view](#notes-view)
  - [Links, backlinks and history](#links-backlinks-and-history)
  - [Jira, Gerrit and smart paste](#jira-gerrit-and-smart-paste)
  - [Quick switcher pop-up](#quick-switcher-pop-up)
  - [Pinned notes](#pinned-notes)
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

## What's new in 0.4

- **Real icons throughout.** Statuses, the header, the sidebar, search and Settings use one crisp icon set ([Lucide](https://lucide.dev)) instead of text symbols, so they look the same on every machine. Each status now has an **icon** you pick from a grid in Settings → Statuses. Your existing statuses are converted automatically.
- **Header action buttons** for status, Jira, Gerrit (or adding a Gerrit change), **pin** and **archive**. Each tooltip shows its shortcut, and the pin button shows when the note is pinned.
- **Know where you are.** The sidebar highlights the current place (Today, Notes, Settings or the open note). A note that isn't listed in the sidebar appears under **Open** while you're in it.
- **Highlighted search matches** in `Ctrl+K`, the `Alt+J` pop-up and the Notes view, so it's clear why each result appeared.
- **Comfortable editor width.** On wide or maximised windows the text sits in a centred column of about 80 characters (Settings → Editor → *Comfortable line width* turns it off).
- **Subtle animations.** Pop-ups fade in, toasts slide in, and the sidebar slides open. Settings → Appearance → *Animations* can follow Windows' "Animation effects" setting (the default), or be always on or off.
- **Better toasts.** Each has an icon and a close button, and pauses while the pointer is over it. Errors and warnings stay until you dismiss them.
- **Friendlier empty states**, and one consistent hover style and keyboard focus ring everywhere.

## What's new in 0.3

- **`Alt+J` is now a quick switcher pop-up.** A small search box with results (the same fuzzy search as `Ctrl+K`) appears over whatever you're doing. `Enter` opens the note in the app, and `Esc` closes the pop-up. It replaces 0.2's quick capture box, and your `Alt+J` key setting carries over.
- **Name a ticket as you create it.** Type or paste `PROJ-1234 Do this thing` to create the ticket with that title. A pasted Jira URL becomes `PROJ-1234 ` so you can type the name straight away.
- **Tickets are found by name everywhere**, including the `[[` link picker, which lists `PROJ-1234 Do this thing` and inserts `[[PROJ-1234]]`.

## What's new in 0.2

- **Statuses:** every ticket and general note can be todo, open, blocked, backlog or closed. Setting **closed** moves the note into the archive. `Ctrl+Shift+S` sets a status, and `@open`-style filters work in search.
- **Sidebar:** a thin rail that expands (hamburger or `Ctrl+\`) into **Active** and **Recent** groups.
- **Notes view** (`Ctrl+L`): a full-window list of every note with search, sorting and grouping by status.
- **Live markdown editor:** markers hide when the caret is elsewhere, headings are sized, checkboxes are clickable, and front matter shows as a compact properties row. The caret is now visible in dark mode.
- **Quick capture** (`Alt+J`; replaced by the quick switcher pop-up in 0.3) and **smart paste** of Jira and Gerrit links.
- **Pinned notes** (`Ctrl+E` then `P`): pop-out always-on-top windows that stay in sync with the main window.
- **Settings screen** (`Ctrl+,`): every option without editing JSON, including shortcut and hotkey recording.
- **Ticket names** now come from the note's first heading (`# PROJ-123 Rollout date question`), not front matter.
- **Opening Jira or Gerrit** keeps the app open (it used to hide).
- **Upgrading:** run the new installer over the old version. It replaces the app in place and keeps your notes and settings.

---

## Installation

Stash Notes targets **Windows 10 and Windows 11** (64-bit). It needs the Microsoft Edge **WebView2** runtime, which ships with Windows 11 and current Windows 10. If WebView2 is missing, the installer downloads it.

### Option 1: download from the Releases page (no login needed)

1. Go to **[Releases → latest](https://github.com/lukewestgate/stash-notes/releases/latest)**.
2. Download one of these:
   - **`Stash.Notes_x.y.z_x64-setup.exe`**, the installer (recommended). It installs for the current user and doesn't need admin rights.
   - **`stash-notes.exe`**, the app as a single file you can run without installing.
3. Run it.

> **Windows SmartScreen.** The builds are not code-signed, so Windows may show *"Windows protected your PC"*. Click **More info → Run anyway**.

### Upgrading to a new version

Download the newer installer and run it. It finds the existing install and replaces it in place. Close Stash Notes first, or let the installer close it. The install location, Start menu entry and login autostart stay the same. **Your notes folder and settings are never touched** by installing, upgrading or uninstalling. Stash Notes doesn't update itself.

### Other ways

- **GitHub Actions artifacts** (needs a GitHub login): every push builds the installer. Open **Actions → Build → Artifacts**.
- **Build it yourself:** see [Building from source](#building-from-source).

### Uninstalling

Use **Settings → Apps → Installed apps → Stash Notes → Uninstall**. Your notes stay in your notes folder. Settings and session data are in `%APPDATA%\com.lukewestgate.stashnotes\`, and the web view's cache is in `%LOCALAPPDATA%\com.lukewestgate.stashnotes\`, if you want to remove those too.

---

## Getting started

1. **Launch Stash Notes** from the Start menu. The window opens on today's daily note. A tray icon (blue note) appears in the notification area.
2. **Type something.** There's no save button. Notes autosave 300 ms after you stop typing, and again whenever the window hides.
3. **Press `Esc`.** The window hides and focus goes back to the app you were using.
4. **Press `Alt+N`** from anywhere. Stash Notes is back with the cursor where you left it. `Alt+N` again hides it.
5. **Press `Ctrl+,`** to open **Settings**. Under **Links**, set your Jira and Gerrit URL templates. Under **Notes**, check the ticket key pattern.
6. **Press `Ctrl+K`** and type a ticket key like `PROJ-123`, then `Enter`. That creates the ticket's note, with status *open* and the caret ready for you to type the ticket's name after `# PROJ-123`.
7. **Press `Ctrl+;`** to insert a timestamp such as `2026-10-22 14:05 `, then type your note.
8. **Press `Ctrl+J`** to open the ticket in Jira.
9. **Press `Alt+J`** from any app. Type a few letters of a note or ticket and press `Enter` to open it, or type `PROJ-456 Fix the login page` and press `Enter` to create that ticket.
10. **Press `Ctrl+/`** for the full list of shortcuts. That screen is the whole manual in brief.

Stash Notes starts automatically at login, hidden in the tray, unless you turn that off in Settings → General.

### A typical flow

`Alt+N` → `Ctrl+K` → type `123` → `Enter` → `Ctrl+;` → type a note → `Ctrl+J` to look at the ticket in the browser → back in the app, `Esc` to return to code.

---

## Using Stash Notes

### The window

```
+------+--------------------------------------------------------+
|  ≡   |  < >  PROJ-123  Rollout question  ◉ open  Jira  pin  arch|  header
|  📅  +--------------------------------------------------------+
|  ☰   |  ● open  Jira  Gerrit 2                     properties |
|  ──  |                                                        |
|  ●   |  PROJ-123 Rollout date question                        |  editor
|  ⊘   |  • 2026-10-22 14:05 asked about rollout, see Rollout…  |
|  ○   |                                                        |
|      |                                                        |
|  ⚙   +--------------------------------------------------------+
|      |  Mentioned in: 2026-10-22 (daily) · Rollout plan       |  backlinks
+------+--------------------------------------------------------+
 sidebar
```

- **Sidebar** (left): a thin rail by default. The hamburger (or `Ctrl+\`) expands it. See [Sidebar](#sidebar).
- **Header:** back and forward, the note's title, its status chip, **Jira** and **Gerrit** buttons when they apply (or a button to add a Gerrit change), then **pin** and **archive**. Hover over any button to see its shortcut. A dot shows unsaved changes.
- **Editor:** markdown that's styled as you type. See [The editor](#the-editor).
- **Backlinks strip:** notes that link to or mention this one.
- **Notes view** (`Ctrl+L`) and **Settings** (`Ctrl+,`) take over the main area. `Esc` goes back.

### Note kinds and the notes folder

Every note is one markdown file in a single **notes folder** (the "vault"). The default is `Documents\StashNotes`. The folder a file is in tells Stash Notes what kind of note it is:

| Kind | Path | Title | Created |
|---|---|---|---|
| Daily | `daily/2026-10-22.md` | The date | On first open each day (`Ctrl+D`) |
| Ticket | `tickets/PROJ-123.md` | The first heading: the key plus the ticket name | From the switcher or `Alt+J` pop-up, or by following `PROJ-123` / `[[PROJ-123]]` |
| Note | `notes/<slug>.md` | First `# heading`, else the file name | From the switcher, or by following a `[[link]]` |
| Archived | `archive/<original path>` | Same as before | `Ctrl+Shift+A`, or setting the status to *closed* |

Markdown files anywhere else in the folder are treated as regular notes. Folders starting with `.` (such as `.git`) and the `templates` folder are ignored.

**Ticket keys** match the regex `[A-Z][A-Z0-9]+-\d+` by default (`PROJ-123`, `OPS-7`). You can change this in Settings → Notes. A bare key anywhere in any note is highlighted and works as a link.

**Ticket note format.** All front matter fields are optional:

```markdown
---
status: open
jira: https://other.atlassian.net/browse/PROJ-123   # only to override the Jira URL template
gerrit:
  - 12345                             # bare change numbers expand via the Gerrit URL template
  - https://gerrit.example.com/c/proj/+/12399
---

# PROJ-123 Rollout date question

- 2026-10-22 14:05 asked about rollout date, see [[Rollout plan]]
- 2026-10-22 16:40 fix pushed, waiting on review
```

The ticket's **name** is whatever follows the key in the first heading. It's shown beside the key in the header, sidebar, switcher and Notes view. The file name stays `PROJ-123.md`, so editing the name never breaks links. Notes from 0.1 that still carry `title:` in front matter keep showing that name until you add one to the heading.

Any note can also list `aliases: [Plan B, launch]`. Links and search then match those names too.

### Statuses

Ticket and general notes can carry one status. Daily notes never do.

| Status | Icon | Meaning | Active |
|---|---|---|---|
| todo | empty circle | Planned, not started | yes |
| open | circle with a dot | Being worked on | yes |
| blocked | slashed circle | Waiting on something | yes |
| backlog | dashed circle | Parked for later | no |
| closed | circle with a check | Done | no |

- **Set it** with `Ctrl+Shift+S` (or by clicking the status chip). Type to filter or press `1`–`5`, then `Enter`. It works on the open note, the focused sidebar row or the selected Notes view row. It rewrites only the `status:` line.
- **Closed notes are archived.** Setting *closed* moves the note into `archive/` (links keep working). Setting any other status on a closed, archived note moves it back.
- **Where it shows:** a chip in the header, an icon in the status's color on every sidebar, switcher and Notes view row, and counts in the tray tooltip (for example *3 open · 1 blocked*).
- **Filters:** type `@todo`, `@open`, `@blocked`, `@backlog`, `@closed`, `@active` (todo, open and blocked) or `@none` in the switcher or Notes view. They combine with text: `@open rollout`.
- New ticket notes start as **open** (Settings → Notes → *Default status*). New general notes start with none.
- The list, icons, colors and which statuses count as *active* are editable in Settings → Statuses (click a status's icon to pick another). An unknown value in a file is kept and shown as a grey question mark.
- This is your personal status. It isn't read from or written to Jira.

### The editor

The editor styles markdown as you type, like Obsidian's live preview. Syntax markers hide when the caret is elsewhere and reappear on the line (or inline span) you're editing. The file on disk always stays plain markdown.

| You type | You see |
|---|---|
| `#` to `######` | Sized headings |
| `**bold**`, `*italic*`, `~~strike~~`, `` `code` `` | Styled text with the markers hidden. `Ctrl+B` / `Ctrl+I` wrap the selection |
| `- ` and `1. ` | Bullets and numbers. `Enter` continues a list, `Tab` / `Shift+Tab` indent |
| `- [ ]` | A clickable checkbox. Checked items are struck through and dimmed |
| `> quote` | A left bar with muted text |
| Fenced code | A monospace block with a background |
| `---` | A horizontal rule |
| `[[link]]`, `[text](url)`, ticket keys | Link-colored text with brackets and URL hidden. `Ctrl+Enter` follows |
| Front matter | A compact properties row (status, Jira, Gerrit). Move the caret into it to edit the YAML |

Tables, images, math and footnotes stay as plain text. To see raw markdown everywhere, turn off **Live formatting** in Settings → Editor.

### Keyboard shortcuts

Every in-app shortcut can be changed in Settings → Shortcuts. In-app shortcuts take priority over the editor's own bindings.

| Shortcut | Action | Notes |
|---|---|---|
| `Alt+N` (global) | Show or hide the app | Works from any app |
| `Alt+J` (global) | Quick switcher pop-up | See [Quick switcher pop-up](#quick-switcher-pop-up) |
| `Esc` | Close overlay / view, else hide window | Closes the switcher, pickers, backlinks list, Notes view and Settings first |
| `Ctrl+K` | Quick switcher and search | Also creates notes. With text selected, links the selection |
| `Ctrl+D` | Today's daily note | Creates it if missing |
| `Ctrl+L` | Notes view | |
| `Ctrl+Enter` | Follow link under cursor | Note link, ticket key or URL. On a checkbox line with no link, toggles it |
| `Alt+Left` / `Alt+Right` | Back / forward | |
| `Ctrl+J` | Open Jira page | The current ticket, or the key under the cursor, or the first key in the note |
| `Ctrl+G` | Open Gerrit change | Picker when there are several (`1`–`9` pick directly) |
| `Ctrl+Shift+G` | Add Gerrit link | Prefilled from the clipboard if it looks like one |
| `Ctrl+;` | Insert timestamp | Format set in Settings → Editor |
| `Ctrl+Shift+S` | Set status | On the note, focused sidebar row or selected Notes view row |
| `Ctrl+Shift+B` | Expand or collapse backlinks | Arrows and `Enter` open one |
| `Ctrl+Shift+A` | Archive or unarchive note | 5 s **Undo** toast |
| `Ctrl+\` | Expand or collapse the sidebar | |
| `Ctrl+0` | Focus the sidebar (and back) | |
| `Ctrl+E`, then `P` | Pin or unpin the note | `Ctrl+P` works too if you're still holding Ctrl |
| `Ctrl+,` | Settings | |
| `Ctrl+/` | Shortcut help | |

Also:

| Shortcut | Action |
|---|---|
| `[[` | Link picker: type to filter, `Enter` inserts `[[Title]]` |
| `Ctrl+B` / `Ctrl+I` | Bold / italic |
| `Ctrl+Shift+V` | Paste without converting Jira or Gerrit links |
| `Ctrl+Click` | Follow a link, key or URL with the mouse |
| `F2` | Rename the current note (links to it are updated) |
| `Ctrl+S` | Save now (not needed, since autosave runs anyway) |
| `Ctrl+Shift+Q` | Quit (or use **Quit** in the tray menu) |

### Quick switcher and search

`Ctrl+K` opens one overlay that handles navigation, search and note creation.

- **Empty query** lists recently opened notes, with active notes first. Closed and archived notes are hidden, so this is also your "open tickets" list.
- **Typing** updates the results on every keystroke. `↑`/`↓` move, `Enter` opens, `Esc` closes.
- **Create.** When nothing matches exactly, the last row reads `Create "<query>"`. A query that starts with a ticket key creates a ticket note, and any text after the key becomes its name: `PROJ-1234 Do this thing` creates `tickets/PROJ-1234.md` titled *PROJ-1234 Do this thing*. If that ticket already exists, it's the top result instead. Anything else creates a regular note.
- **`Tab`** includes closed and archived notes, shown dimmed.
- **`Alt+Enter`** *peeks* at a result without adding it to history.
- **Pasting a Jira URL** turns it into the ticket key.

What matches, from strongest to weakest:

| Field | Examples that match | Weight |
|---|---|---|
| Ticket key | `123`, `proj123`, `PROJ-123` | Highest |
| Title and aliases | `rollout`, `rlt pln` (fuzzy) | High |
| Date | `2026-10-22`, `oct 22`, `today`, `yesterday`, `monday` | High (matches the daily note) |
| Body text | Any word or words from a note, or a timestamp | Normal, with the matching line shown |

Typing **only digits** (for example `123`) matches every ticket key ending in those digits. Notes opened in the last 7 days get a boost. There are no regexes, boolean operators or saved searches. For those, search the notes folder with Everything, ripgrep or your editor.

### Sidebar

The sidebar starts as a thin rail. It shows **Today**, **Notes**, the status icons of your active notes and **Settings**. Hover over an icon to see the note's title. Click the hamburger (or press `Ctrl+\`) to expand it:

- **Today** and **Notes** rows, each with its shortcut as a hint.
- **Active:** notes with an active status, ordered open, blocked, then todo, and by last edit within each. Up to 8 rows, then a **+N more** row that opens the Notes view filtered to `@active`.
- **Recent:** the last 10 notes you opened, skipping daily notes and anything already listed.
- Where you are is always highlighted with an accent bar and tint: **Today** on today's note, **Notes** in the Notes view, **Settings** in Settings, or the open note's row. A note not otherwise listed shows under **Open** while it's in the editor. Right-click a row to set its status.
- Drag the right edge to resize it (180–360 px). Its width and expanded state are remembered.
- Below a 720 px window width, the expanded sidebar floats over the editor and closes after you open a note.
- **Keyboard:** `Ctrl+0` focuses it, `↑`/`↓` move, `Enter` opens (and puts the caret in the editor), `Ctrl+Shift+S` sets the row's status, and `Esc` goes back.

### Notes view

`Ctrl+L` (or the sidebar's **Notes** row) shows every note in the main area. It's a history entry, so `Alt+Left` comes back to it, and `Esc` leaves it.

- **Search** with the switcher's syntax, including `@status` filters.
- **Rows** show the status, key or title, a one-line snippet (the matching line while searching) and when it was last edited.
- **`Alt+S`** cycles the sort: last edited, last opened, status, title.
- **`Alt+G`** groups by status. `←`/`→` collapse or expand a group header.
- **`Alt+D`** shows or hides daily notes, and **`Tab`** shows or hides closed and archived notes (both hidden by default).
- **`Enter`** opens the selected note. **`Ctrl+Shift+S`** sets its status, and **`Ctrl+E`, `P`** pins it. A **Create** row appears when nothing matches.

Defaults for these are in Settings → Notes view.

### Links, backlinks and history

| Written as | Opens | If the target is missing |
|---|---|---|
| `[[Rollout plan]]` | The note titled "Rollout plan" (case-insensitive) | Creates `notes/rollout-plan.md` |
| `[[PROJ-123]]` or bare `PROJ-123` | `tickets/PROJ-123.md` | Creates it |
| `[[2026-10-22]]` | `daily/2026-10-22.md` | Creates it from the daily template |
| `https://...` or `[text](https://...)` | Your default browser | – |

`[[Target|shown text]]` and `[[Target#section]]` also work. **Resolution order:** exact file name, then title, then alias. If two notes share a title, the most recently edited one wins. Links to archived notes still work and are shown dimmed.

**Backlinks.** The strip at the bottom lists every note that links to the current one or mentions its ticket key. `Ctrl+Shift+B` expands it into a list.

**History** works like a browser. `Alt+Left` and `Alt+Right` restore each note's cursor and scroll position. It keeps the last 100 entries and is cleared when you quit.

### Jira, Gerrit and smart paste

- **`Ctrl+J`** opens the ticket using the Jira URL template (for example `https://yourco.atlassian.net/browse/{key}`), or the note's `jira:` override.
- **`Ctrl+Shift+G`** adds a Gerrit URL or change number to the note's `gerrit:` list. **`Ctrl+G`** opens it, or shows a picker if there are several.
- The app stays open behind the browser. If you'd rather it hide, turn on Settings → General → *Hide the window after opening a Jira or Gerrit link*.

**Smart paste** (on by default; `Ctrl+Shift+V` always pastes as typed):

| Where | You paste | Result |
|---|---|---|
| Switcher, `Alt+J` pop-up, Notes view search | A Jira URL (`…/browse/PROJ-123`, or a board link with `selectedIssue=PROJ-123`) | It becomes `PROJ-123 ` with the caret after it. If the ticket exists it's the top result. Otherwise keep typing its name, and **Create** makes `PROJ-123 <name>` |
| Editor, any note | A Jira URL | Inserts `PROJ-123`, which links |
| Editor, a ticket note | A Gerrit URL (same host as the Gerrit template) | Added to the `gerrit:` front matter, with an **Undo** toast |

### Quick switcher pop-up

Press **`Alt+J`** from any app. A small search box appears near the top of the screen your mouse is on. The main window stays wherever it is, maximised or hidden.

- It's the same switcher as `Ctrl+K`: fuzzy search by ticket key, ticket name, title, date, text or `@status`, with your recent and active notes listed when it's empty.
- **`Enter`** opens the selected note in the app.
- When nothing matches, the **Create** row creates the note and opens it. `PROJ-1234 Do this thing` creates a ticket with that name, and pasting a Jira URL gives you `PROJ-1234 ` to type the name after.
- **`Esc`** (or clicking anywhere else) closes the pop-up and returns you to what you were doing.
- `Alt+J` again while it's open closes it. You can change the key in Settings → General.

### Pinned notes

Press **`Ctrl+E` then `P`** to pop the current note out into a small always-on-top window (320 × 400). Do it again, inside the pinned window or on the same note, to close it. It also works on the selected sidebar or Notes view row.

- Both windows show the same live text. You can type in either, and the edits merge.
- Drag it by its title bar. The pin icon turns always-on-top off and on, and **×** closes it. `Esc` does nothing here, so it can't be closed by accident.
- `Ctrl+;`, `Ctrl+J`, `Ctrl+G`, `Ctrl+Shift+S` and `Ctrl+Enter` work inside it. Links open in the main window.
- Up to 5 notes can be pinned. Each window's position and size are remembered per note, and pinned windows come back at login (Settings → General).
- The tray menu has a **Pinned notes** submenu to focus or close each one.
- If the note is renamed or archived, the window follows it.

### Daily notes and todos

`Ctrl+D` opens today's note (`daily/YYYY-MM-DD.md`), creating it from a template with a `## Todo` section. The cursor lands at the end of the todo list, ready to type.

- `Enter` at the end of a `- [ ] item` continues the list with a new checkbox.
- `Ctrl+Enter` on a checkbox line (or clicking the box) toggles it.
- Unchecked items don't roll over to the next day.
- If the last note you had open was a **previous** day's daily note, summoning opens today's instead.

**Template:** edit it in Settings → Notes. It's saved as `templates/daily.md` in the notes folder. `{{date}}` and `{{weekday}}` are filled in.

### Archiving and renaming

- **`Ctrl+Shift+A`** moves the note into `archive/`, keeping its relative path, and moves it back if it's already archived. Archived notes are left out of search unless you press `Tab`, and links to them still work. **Undo** is available for 5 seconds.
- **`F2`** renames a regular note: the file, its `# heading` and every `[[link]]` to it. **Undo** is available for 5 seconds. Ticket names are edited in the heading after the key. Daily notes are named by their date.
- Stash Notes **never deletes** files. To delete a note, delete the file in Explorer.

### Window, tray and startup

- **Login:** Stash Notes starts with Windows and stays hidden in the tray, already loaded. Starting it from the Start menu shows the window.
- **`Alt+N`**: if the window is hidden, it shows and focuses the editor. If it's visible and focused, it hides. If it's behind another window, it comes to the front. It never touches pinned windows.
- **Close (X), Minimize and `Esc`** hide to the tray. Quit from the tray menu or with `Ctrl+Shift+Q`.
- **Tray icon:** left-click toggles the window. Right-click opens the menu: *Show, Today's note, Open notes folder, Settings, Pinned notes, Quit*. The tooltip counts your active notes.
- **Single instance:** launching Stash Notes again just shows the running window.
- **Window size and position** are remembered.

---

## Settings

Press **`Ctrl+,`** (or click the sidebar's gear, or use **Settings** in the tray menu). Changes apply immediately, with no Save button.

| Category | Options |
|---|---|
| General | Launch at login · Start hidden in tray · Restore pinned notes · What opens on summon · Show/hide hotkey and quick switcher hotkey (press the keys to set; a key another app owns is refused) · Hide after opening Jira/Gerrit |
| Window | Show in taskbar · Always on top · Sidebar expanded |
| Editor | Live formatting · Comfortable line width · Font and size · Line height · Spell check · Smart paste · Timestamp format (`yyyy MM dd HH hh mm ss a`) |
| Notes | Notes folder (with a folder picker) · Default status for new tickets · Ticket key pattern (with a test field) · Daily note template |
| Links | Jira and Gerrit URL templates, each with a live preview |
| Notes view | Show daily notes · Show closed notes · Default sort · Group by status |
| Statuses | Icon, name, color and *active* flag of each status · Reorder · Add · Remove |
| Shortcuts | Every in-app shortcut: click it and press new keys. Conflicts are refused. Reset per row or all |
| Appearance | Theme (follow Windows, light, dark) · Accent color · Animations (follow Windows, on, off) |

Each category has **Reset to defaults**. The footer opens `settings.json` and the notes folder.

**`settings.json`** (`%APPDATA%\com.lukewestgate.stashnotes\settings.json`) is still the source of truth, and hand edits show up in the screen live. Every key is optional. Keys the app doesn't know are kept. The main keys:

| Key | Default |
|---|---|
| `vaultPath` | `Documents\StashNotes` (`~` means your user folder) |
| `hotkey` / `switcherHotkey` | `Alt+N` / `Alt+J` |
| `onSummon` | `last` (or `daily`) |
| `startHidden`, `autostart`, `restorePinned` | `true` |
| `showInTaskbar`, `alwaysOnTop`, `hideAfterExternalLink` | `false` |
| `ticketPattern` | `[A-Z][A-Z0-9]+-\d+` (escape backslashes in JSON: `"\\d"`) |
| `jira.urlTemplate` | `https://yourco.atlassian.net/browse/{key}` |
| `gerrit.urlTemplate` | `https://gerrit.example.com/c/{n}` |
| `defaultStatus` | `open` |
| `statuses` | the five statuses above, each `{ "name", "icon", "color", "active" }`. `icon` is a [Lucide](https://lucide.dev/icons) name from the Settings grid, e.g. `circle-dot` |
| `liveFormatting`, `smartPaste`, `readableWidth` | `true` |
| `spellCheck` | `false` |
| `fontFamily`, `fontSize`, `lineHeight` | system UI font, `14`, `1.6` |
| `timestampFormat` | `yyyy-MM-dd HH:mm` |
| `theme`, `accent` | `system`, `blue` |
| `animations` | `system` (or `on`, `off`) |
| `notesView` | `{ "showDaily": false, "showClosed": false, "sort": "edited", "groupByStatus": false }` |
| `keys.*` | see [Keyboard shortcuts](#keyboard-shortcuts). `pinNote` is a two-key chord like `"Ctrl+E P"` |
| `schemaVersion` | managed by the app |

`session.json` in the same folder holds the last open note, recently opened notes, pinned windows and the sidebar state. It's safe to delete.

---

## Your data and safety

- **Plain files are the source of truth.** Notes are ordinary UTF-8 markdown files. You can edit them in any editor while Stash Notes runs, keep the folder in Git or OneDrive, and back it up any way you like. The search index lives only in memory and is rebuilt at every start.
- **One writer.** Rust holds the text of each open note, and every window (main and pinned) edits that one copy. Only that copy is written to disk, so two windows can never overwrite each other.
- **Autosave** runs 300 ms after the last change, and on hide, quit, navigate, archive and rename.
- **Crash-safe writes:** each save goes to a temporary file that's then renamed over the original. If another program holds the file open (for example a sync client), it retries briefly and then writes in place.
- **External changes:** if a note changes on disk while it has no unsaved edits, it reloads silently in every window. If it has unsaved edits, your text is kept and a bar offers **Reload from disk** or **Keep mine**.
- **Nothing is deleted.** Archiving and closing move files. Renaming rewrites links, and **Undo** puts everything back.
- **Upgrades are safe.** The installer only replaces the app. Notes live in your notes folder and settings in `%APPDATA%`, and neither is touched. When a new version changes what a setting means, it first saves the old file as `settings.json.bak`. Front matter keys the app doesn't know are left alone, and notes you didn't edit are never rewritten.
- **Locked-down UI:** the web UI has no direct file system access. Every read and write goes through Rust commands that only accept paths inside the notes folder. Only `http`/`https` URLs can be opened.

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
- `bundle\nsis\Stash Notes_0.2.0_x64-setup.exe` is the installer (per-user, no admin rights, upgrades in place).

### Run in development mode

```powershell
npm run tauri dev
```

This starts the Vite dev server with hot reload and a debug build of the app. Debug builds never register themselves for autostart.

### Linux and macOS

The app is designed for Windows, but it also builds and runs on Linux (tested in CI). On Debian/Ubuntu, first install `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev`. macOS should work but isn't tested.

---

## Development

### Project layout

```
stash-notes/
├── src/                        Svelte + TypeScript UI
│   ├── main.ts                 Picks the app for this window: main, pinned note or Alt+J pop-up
│   ├── App.svelte              Main window: navigation, views, keyboard handling
│   ├── PinnedApp.svelte        A pinned note window
│   ├── QuickApp.svelte         The Alt+J quick switcher pop-up
│   ├── components/             Header, Sidebar, Switcher, NotesView, SettingsView, Backlinks,
│   │                           StatusPicker, KeyRecorder, Help, Prompt, Picker, StatusIcon,
│   │                           Icon, Highlight, Toasts, EmptyState
│   └── lib/
│       ├── api.ts              Typed wrappers for the Rust commands and events
│       ├── buffer.ts           Syncs an editor with its note's Rust buffer (CodeMirror collab)
│       ├── editor.ts           CodeMirror setup: decorations, theme, [[ link picker
│       ├── live.ts             Live markdown formatting and the front matter row
│       ├── text.ts             Link/key detection, checkboxes, timestamps, smart paste
│       ├── keys.ts             Shortcut parsing, matching, chords and recording
│       ├── history.ts          Back/forward history
│       ├── icons.ts            The Lucide icons the app uses, by name
│       ├── highlight.ts        Which letters of a result to highlight
│       ├── motion.ts           Animation setting and the shared pop-in transitions
│       ├── toasts.svelte.ts    Toast queue (timers, pause on hover, sticky errors)
│       ├── status.ts / appearance.ts
│       └── mock.ts             In-memory backend for running the UI in a browser
├── src-tauri/                  Rust (Tauri v2) app
│   ├── src/
│   │   ├── lib.rs              App setup, plugins, window events
│   │   ├── commands.rs         Commands the UI calls
│   │   ├── buffers.rs          One authoritative text buffer per open note
│   │   ├── pins.rs             Pinned note windows
│   │   ├── quick.rs            The Alt+J pop-up window
│   │   ├── config.rs           settings.json (with migrations), indexing, file watching
│   │   ├── window.rs           Show/hide/focus (incl. the Windows foreground fix)
│   │   ├── tray.rs             Tray icon, menu and tooltip
│   │   ├── hotkey.rs           Global hotkeys
│   │   └── state.rs            Shared state and session persistence
│   ├── core/                   `stash-core` (no Tauri dependency; unit tested)
│   │   ├── src/                vault, index & search, notes, front matter, statuses,
│   │   │                       change sets, dates, settings
│   │   └── tests/perf.rs       5,000-note performance check
│   ├── capabilities/           Webview permissions (core events and window dragging only)
│   └── tauri.conf.json         Windows, bundle and security config
└── .github/workflows/          CI: tests on Linux, installer on Windows, releases
```

**Architecture.** Rust owns the files, the search index, the windows and the text of every open note. The web UI draws and sends commands. At startup, Rust scans the notes folder into an in-memory index in a background thread, then watches the folder (and `settings.json`) for changes. Each window opens a note with `buffer_open` and pushes its edits as CodeMirror change sets (`buffer_push`). Rust applies them to the one buffer, broadcasts them to every window showing the note (`buffer-updates`) and autosaves. Other commands cover search (`search`, `list_notes`, `sidebar`, `backlinks`), notes (`create_note`, `set_status`, `archive_note`, `rename_note`), pinned notes, the pop-up and settings. Fuzzy matching uses the [`nucleo`](https://github.com/helix-editor/nucleo) matcher.

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

In `npm run dev`, add `?pin=notes/rollout-plan.md` to the URL to see the pinned window, or `?quick` for the `Alt+J` pop-up.

### Continuous integration

`.github/workflows/build.yml` runs on every push and pull request:

- **Tests (Linux):** type check, UI tests, UI build, Rust tests, and the performance check (informational).
- **Windows installer:** `npm run tauri build`, then uploads the installer and the portable exe as artifacts.
- **Release:** on pushes to the default branch (and tags matching `v*`), publishes them to the GitHub Release `v<version>`, using the version in `tauri.conf.json`.

To publish a new version, bump `version` in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`, then push.

---

## Troubleshooting

**`Alt+N` or `Alt+J` does nothing.** Another app may already be using that hotkey. When this happens, Stash Notes shows a message on startup. Set a different one in Settings → General.

**The window appears but typing goes to the previous app.** Windows sometimes refuses focus to a window brought up by a background process. Stash Notes works around this. If it still happens, press the hotkey once more, and please open an issue.

**Jira/Gerrit open the wrong site.** Set the URL templates in Settings → Links.

**My ticket keys aren't highlighted.** Keys must match the ticket pattern (Settings → Notes has a test field). Lower-case keys typed in the switcher are upper-cased for you.

**A pinned window is off screen.** Close it from the tray menu (**Pinned notes**) and pin the note again.

**Where are my notes?** Use **Open notes folder** in the tray menu or in Settings.

**I want to start fresh.** Quit the app, then delete `%APPDATA%\com.lukewestgate.stashnotes`. Your notes folder is never touched.

---

## Design notes and decisions

Stash Notes implements the *Quick Notes (Tauri) — Product & Technical Spec*. Where the spec left questions open, these choices were made:

- **Name and folder.** The app is called *Stash Notes*, and its default notes folder is `Documents\StashNotes`.
- **Closed notes are archived.** Setting *closed* also moves the note into `archive/`, and reopening moves it back (owner's choice on the spec's open question).
- **Sidebar.** A thin rail with a hamburger that expands into the full sidebar (owner's choice). The expanded state is remembered.
- **No in-app updater.** The spec's auto-update section is left out at the owner's request. Installing a newer version replaces the old one in place, and only the per-user NSIS installer is shipped, so there's never a second copy. The update-related settings (*Check for updates*, *Install automatically*) and the pre-update vault snapshot are therefore not present.
- **Jira and Gerrit URLs** default to placeholders. Set yours in Settings → Links.
- **Daily todos don't roll over** (spec v1).
- **Shortcuts beyond the spec:** `F2` rename, `Ctrl+S` save now, `Ctrl+Click` follow, and right-click on a sidebar row to set its status.
- **Shortcut conflicts** are refused when recording, not just flagged, so two actions can never share a key.
- **Peek (`Alt+Enter`)** shows the chosen note without adding it to history.
- **`Alt+J` is a separate pop-up** rather than the main window with its switcher open, so a maximised app doesn't take over the screen just to search.
- **Animations.** The spec asked for an instant summon, and that's unchanged: showing and hiding the window never animates. Only small in-app transitions (pop-ups, toasts, the sidebar) do, they're short (under 200 ms), and they follow Windows' animation setting unless you choose otherwise.
- **Icons** are a bundled subset of [Lucide](https://lucide.dev) (ISC licence), drawn as inline SVG, so no icon font or network access is needed.
- **Ticket files are named by key only** (`PROJ-1234.md`), and the name lives in the heading. Rewording a ticket never breaks links, and tickets are still found by name everywhere.
