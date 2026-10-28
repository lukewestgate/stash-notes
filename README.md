# Stash Notes

A keyboard-first, always-resident notepad for Windows. One global hotkey (`Alt+N`) brings it up instantly. You read or add a note, and one keypress (`Esc`) sends you back to what you were doing.

Notes are plain markdown files in folders you own. There's no account, no sync and no database. Stash Notes is built with [Tauri v2](https://v2.tauri.app/) (Rust) and a Svelte + CodeMirror UI.

- **Instant summon.** The window is created once at login and only ever shown or hidden.
- **Vaults.** Keep personal notes and each project's notes in their own folders, switch between them in a keystroke, and search them all at once.
- **Links, collected.** Paste a link into a note (a ticket, a review, a video, a page) and it's in the note's **Links** panel, with every vault's links one tab away. Optional link rules label a service's links and make a note from a link or key, such as one note per Jira ticket.
- **Statuses and tags.** Mark notes todo, open, blocked, backlog or closed, and tag them `#bug`, `#research` or anything else.
- **Live markdown.** Headings, bold, lists and checkboxes are styled as you type, and the file stays plain markdown.
- **Embedded images.** Paste a screenshot, drop image files into a note, or use **Insert image** in the note menu. Images display inline, including in pinned notes and expanded note previews.
- **Sidebar and Notes view.** See what you're working on at a glance, and scan, sort and filter every note.
- **One note menu.** Right-click any note, wherever it's listed, to set its status, tag, pin, move, archive or delete it.
- **Quick switcher pop-up** (`Alt+J`). Search, open or create a note from any app without bringing up the full window first.
- **Pinned notes.** Pop any note out into a small always-on-top window beside your code.
- **One fuzzy switcher** (`Ctrl+K`). Find notes by ticket key, title, date or text, filter with `@status`, `#tag` and `in:vault`, and create new ones, including from a pasted link.
- **Fully usable without the mouse.** Press `Ctrl+/` to see every shortcut.

---

## Contents

- [What's new in 0.10](#whats-new-in-010)
- [What's new in 0.9](#whats-new-in-09)
- [What's new in 0.8](#whats-new-in-08)
- [What's new in 0.7](#whats-new-in-07)
- [What's new in 0.6](#whats-new-in-06)
- [What's new in 0.5](#whats-new-in-05)
- [Installation](#installation)
- [Getting started](#getting-started)
- [Using Stash Notes](#using-stash-notes)
  - [The window](#the-window)
  - [Note kinds and the notes folder](#note-kinds-and-the-notes-folder)
  - [Vaults](#vaults)
  - [Statuses](#statuses)
  - [Tags](#tags)
  - [The editor](#the-editor)
  - [Images](#images)
  - [Keyboard shortcuts](#keyboard-shortcuts)
  - [Quick switcher and search](#quick-switcher-and-search)
  - [Sidebar](#sidebar)
  - [Notes view](#notes-view)
  - [The note menu](#the-note-menu)
  - [Links, backlinks and history](#links-backlinks-and-history)
  - [Links and link rules](#links-and-link-rules)
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

## What's new in 0.10

More for the daily note: getting around it, adding to it, and finding days by name.

- **Add a todo from anywhere.** In the `Alt+J` pop-up (or `Ctrl+K`), type `+ chase Sam about the demo` and press `Enter`. It goes into today's Todo list without bringing up the app. See [Daily notes and todos](#daily-notes-and-todos).
- **Step through daily notes.** `Alt+PgUp` and `Alt+PgDn` go to the previous and next daily note, skipping days without one.
- **How long a todo has been open.** A carried todo shows a small `5d` badge after it once it's been open two days or more, counted from the daily note it was first written in. The file isn't changed.
- **More date words in search:** `last tuesday`, `tuesday last week`, `this week fri`, `next monday`, `3 days ago`, `a week ago`, `2 weeks ago monday`, `day before yesterday`. The switcher shows which day it read, and offers to create that day's note if there isn't one. See [Quick switcher and search](#quick-switcher-and-search).

## What's new in 0.9

0.9 is about the daily note you use every day.

- **Open todos move on to the next day.** When today's daily note is made, the unticked items from your last one (yesterday's, or Friday's on a Monday) come with it, with any lines nested under them. In the earlier note they become `- [>] item`, shown as a muted arrow box, so each todo is open in only one place. `Ctrl+Shift+D` pulls them in by hand. Settings → Notes can copy them instead, or turn it off. See [Daily notes and todos](#daily-notes-and-todos).
- **Daily notes in your date format.** Choose how daily notes are named on screen, such as `28/10/2026` or `Wed 28 Oct 2026`, in Settings → Notes. Files stay `2026-10-28.md`, so they still sort by year, month and day. Search understands dates typed your way: `28/10` finds the 28th of October.

## What's new in 0.8

0.8 tidies the window so each thing has one place, instead of the same buttons in two or three, and gives notes one place on disk too.

- **Every note in one `notes` folder.** Daily notes, tickets and the rest all go in `notes/`, and a note's name says what it is: `2026-10-22.md` is a daily note, `PROJ-123.md` a ticket. See [Note kinds and the notes folder](#note-kinds-and-the-notes-folder).
  - **Archiving marks a note** (`archived: true` in its properties) instead of moving it into `archive/`, so the file stays put.
  - **The app's own files** (the daily template and deleted notes' recovery copies) are in a hidden `.stash` folder. Images stay in `assets`.
  - **Upgrading:** each vault still using the older folders is offered a **tidy** after you start 0.8 (or in Settings → Vaults). It shows what will move first, never overwrites a file, fixes links that spell out an old path, and has **Undo**. Until then, notes in the older folders keep working.

- **A note menu where you right-click.** Right-clicking a note used to open a dialog in the middle of a dimmed window. Now a menu opens at the pointer with **Open**, **Pin**, **Status**, **Tags**, **Copy link**, **Rename**, **Move to**, **Archive** and **Delete** (and, on the open note, **Open** its primary link, **Links…** and **Insert image**).
  - **Everywhere a note is listed:** the sidebar, Notes view rows, backlinks and `Ctrl+K` results. The header's **…** opens it for the open note.
  - **From the keyboard:** `Shift+F10` (or the menu key) opens it on the selected row. Arrows move, `→` opens Status or Move to, `1`–`5` pick a status, and `Esc` closes.
  - **Move to** lists the other vaults directly, so the Move dialog is gone. **Delete** still asks first and still recommends Archive.
- **The properties row is the one place for a note's details:** status, tags, the primary link and the note's link count (which opens the Links panel).
  - **Every ticket and general note has the row**, even before it has any front matter.
  - **+ Add** adds tags or a link.
- **A simpler header:** back, forward and a breadcrumb (*Work / Tickets / PROJ-123 Rollout date question*), then **Pin** and **…**. The primary link and **Links (N)** moved to the properties row; **Archive** and **Insert image** moved into the note menu.
- **Popups open beside what you clicked.** The status chip, **+ Add** and the sidebar's vault row open their lists next to themselves, with nothing dimmed. Shortcuts still open them in the middle.
- **Sidebar:**
  - A **Search or create** box opens `Ctrl+K`.
  - The vault row is a dropdown, with **Open vault folder** and **Manage vaults**.
  - Notes in pinned windows are listed under **Pinned**.
  - A **?** button beside Settings opens the shortcuts.
  - The **Open** group is gone, because the header's breadcrumb shows which note you're in.
- **`Ctrl+K` and the Notes view are one search.** The vaults searched are tabs under the search box, and `Ctrl+Enter` opens the Notes view with the same query.
- **Every shortcut is unchanged.** Clicking the note's title no longer opens `Ctrl+K`; use the sidebar's search box or `Ctrl+K`.

## What's new in 0.7

- **Links, in one place, in every vault.** Every link in a note is collected without any setup: bare URLs, Markdown links, autolinks and links in front matter, but not code, comments or image sources. The header's **Links (N)** button (or `Ctrl+Shift+L`) opens the **Links panel** beside the editor, and it updates as you type.
  - **This note** lists each link once, with how often and where it appears: **Open**, **Copy**, **Go to source**, make it the note's **primary link**, or **Add link** for one that isn't in the text.
  - **All notes** (also the sidebar's **Links** row) lists the links of every vault with the notes that use them, filtered by text, vault and archive.
  - Pinned notes have the same panel.
- **Link rules replace per-vault ticket systems and attached link types.** Settings → **Links** holds one list for every vault. Rules are optional: a rule recognises a service's links (start from a preset and paste an example link), labels them (`PROJ-123`, `PR 42`), can format a pasted link as `[PROJ-123](https://…)`, can read bare keys, and can offer a **note template**.
- **Create a note from a link.** Paste a link, or type a key, into `Ctrl+K`, the `Alt+J` pop-up or the Notes view, then choose **Create note from link**. You see the vault, an editable title and the template's result first, and notes already about that link are offered instead. The note keeps the link as its `primary_link`.
- **One header button for the primary link** (**Open PROJ-123**) instead of a button per ticket system and link type. `Ctrl+J` opens the primary link, `Ctrl+G` any of the note's links, and `Ctrl+Shift+G` adds any link.
- **Upgrading from 0.6:**
  - Settings are backed up as `settings.json.bak`, then each vault's ticket systems and link types become link rules. Identical ones are merged; different services stay separate, named by vault (*Jira (Work)*).
  - Nothing in your notes is renamed, moved or rewritten. `tickets/KEY.md` notes, `jira:` overrides and `gerrit:`, `prs:`, `mrs:`, `docs:` or custom fields keep opening where they did, and show in the Links panel.
  - Your shortcuts keep their keys. If smart paste was on, pasted ticket links now go in as `[PROJ-123](link)` (the key, still linked) instead of the bare key.
  - **Pasting a Gerrit, PR or wiki link no longer moves it into the front matter.** It stays where you paste it and appears in the Links panel. Use **Add link** to attach one without putting it in the text.

## What's new in 0.6

- **Embedded pages.** Hover or Tab to a note link and click its arrow to expand the linked note beneath that line, read-only, in a rounded panel. Previews refresh when the file changes, nest (with circular links stopped), work across vaults and in pinned notes, and never create a missing note. Hover a preview for **Open note**.
- **Embedded images.** Paste a screenshot, drop image files, or click **Insert image** in the note header. Images display inline, including in pinned notes and expanded previews, and are copied into the vault's `assets/` folder.
- **File actions.** Click **…** beside a sidebar note (or right-click its row), or **…** in the note header, to **Rename**, **Move**, **Set status** or **Delete**.
  - **Move between vaults.** The note keeps its filename and folder, links to it are updated, its images come along, and open editors and pinned windows follow it. Existing files are never overwritten.
  - **Delete.** Stash Notes can now delete notes, which earlier versions never did. Deleting always asks first and recommends **Archive** instead. **Undo** is available for 5 seconds, and a recovery copy stays in the vault's hidden `.trash/` folder after that.
  - **Created a note by mistake?** The creation notice has a **Delete** button for 10 seconds.
- **Popups close the way you'd expect.** Click outside any popup or confirmation to dismiss it, and press `Ctrl+K` (or your switcher shortcut) again to close the switcher. Focus returns to where you were.

## What's new in 0.5

- **Vaults.** Keep several folders of notes, for example *Personal* and one per project, each with its own name and color.
  - **Switching:** use the vault row at the top of the sidebar, `Ctrl+Shift+O`, or the tray menu. Opening a note from another vault switches with it.
  - **Per vault:** the sidebar, Notes view and Today (each vault keeps its own daily notes) show one vault at a time.
  - **Search** (`Ctrl+K`, `Alt+J`, the `[[` picker) covers every vault, with the current one first and a vault tag on each result. Narrow it with `in:work`, or press `Tab` to cycle *All vaults / this vault / each vault*. Closed and archived notes moved to `Alt+A`.
  - **Links** look in their own vault first, and `[[work:Some note]]` names a vault.
  - **Upgrading:** your notes folder becomes the first vault, *Personal*. Rename it in Settings → Vaults if it's really your work vault.
- **Tags.**
  - **Adding them:** tag ticket and general notes (`tags: [bug, research]` in front matter) with `Ctrl+Shift+T`, the header's tag button or the `+` in the properties row.
  - **Searching:** `#bug` filters anywhere, nested tags like `#bug/ui` match their parent, and tags are suggested as you type (`Tab` completes).
  - **Notes view:** `Alt+G` now cycles grouping: none, status, tag.
  - **Settings → Tags** gives tags colors, and renames or removes a tag across all notes, with Undo.
- **Links to other systems, per vault.** Jira and Gerrit are now just two presets.
  - A vault can have **ticket systems**, which give a note per key: Jira, GitHub issues, GitLab issues or custom.
  - It can also have **attached link types**, which a note collects in its front matter: Gerrit, GitHub PRs, GitLab MRs, Confluence/wiki pages or custom.
  - A vault with none is a plain notes app: no ticket highlighting, no extra buttons, and URLs paste as they are.
  - `Ctrl+J` / `Ctrl+G` / `Ctrl+Shift+G` now open the note's ticket, open an attached link, or attach one.
  - **Upgrading:** your Jira and Gerrit settings move to your first vault, and the shortcuts keep their keys.

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
5. **Press `Ctrl+K`**, type a title like `Shopping list` and press `Enter` to create a note. Press `Ctrl+;` to insert a timestamp such as `2026-10-22 14:05 `.
6. **Paste a link** into the note, any link. The properties row above the text now says **1 link**; click it to see the Links panel.
7. **Set up a work vault and Jira (optional).** Press `Ctrl+,`:
   - **Vaults → Add a vault** named *Work* with its own folder.
   - **Links → Add a rule → Jira**, and paste a link to one of your Jira issues. The rule works in every vault.
8. **Press `Ctrl+K`**, type `PROJ-123 Rollout date` (or paste the issue's link) and choose **Create note from PROJ-123**. Check the preview and press `Enter`. **`Ctrl+J`** opens the issue in Jira.
9. **Press `Ctrl+Shift+O`** to switch vault, and **`Ctrl+Shift+T`** to tag the note.
10. **Press `Alt+J`** from any app. Type a few letters of any note in any vault and press `Enter` to open it, or paste a link to create a note about it.
11. **Press `Ctrl+/`** for the full list of shortcuts. That screen is the whole manual in brief.

Stash Notes starts automatically at login, hidden in the tray, unless you turn that off in Settings → General.

### A typical flow

`Alt+N` → `Ctrl+K` → type `123` → `Enter` → `Ctrl+;` → type a note → `Ctrl+J` to look at the ticket in the browser → back in the app, `Esc` to return to code. Or from anywhere: `Alt+J` → `rollout` → `Enter`.

---

## Using Stash Notes

### The window

```
+------+--------------------------------------------------------+
|  ▯   | < >  Work / Tickets / PROJ-123 Rollout question  📌 …  |  header
|  🔍  +--------------------------------------------------------+
|  📅  |  ● open  #bug | PROJ-123  3 links  + Add               |  properties
|  ☰   |                                                        |
|  ──  |  PROJ-123 Rollout date question                        |  editor
|  ●   |  • 2026-10-22 14:05 asked about rollout, see Rollout…  |
|  ⊘   |                                                        |
|  ⚙   +--------------------------------------------------------+
|      |  Mentioned in: 2026-10-22 (daily) · Rollout plan       |  backlinks
+------+--------------------------------------------------------+
 sidebar
```

- **Sidebar** (left): a thin rail by default. The panel button (or `Ctrl+\`) expands it. See [Sidebar](#sidebar).
- **Header:** where you are and what you can do with the note. Back and forward, a breadcrumb (the vault when you have several, the kind of note, then its title), **Pin**, and **…** for the [note menu](#the-note-menu). A dot shows unsaved changes.
- **Properties row:** the note's status, tags, primary link (**PROJ-123**, when it has one), its link count for the [Links panel](#links-and-link-rules), and **+ Add**. Every ticket and general note has one; daily notes don't. See [The editor](#the-editor).
- **Links panel** (right, when open): the note's links, or every note's.
- **Editor:** markdown that's styled as you type. See [The editor](#the-editor).
- **Backlinks strip:** notes that link to or mention this one.
- **Notes view** (`Ctrl+L`) and **Settings** (`Ctrl+,`) take over the main area. `Esc` goes back.

### Note kinds and the notes folder

Every note is one markdown file in a **vault**: a folder of notes (see [Vaults](#vaults)). The first vault's default folder is `Documents\StashNotes`. Inside it:

```
StashNotes\
├── notes\           every note
│   ├── 2026-10-22.md     a daily note: named by its date
│   ├── PROJ-123.md       a ticket note: named by its key
│   └── rollout-plan.md   any other note: named after its title
├── assets\          images pasted or dropped into notes
└── .stash\          the app's own files (hidden)
    ├── templates\daily.md
    └── trash\           recovery copies of deleted notes
```

A note's **file name** says what kind it is, wherever it is:

| Kind | File name | Title | Created |
|---|---|---|---|
| Daily | A date: `2026-10-22.md` | The date | On first open each day (`Ctrl+D`) |
| Ticket | A key a [link rule](#links-and-link-rules) reads, exactly: `PROJ-123.md` | The first heading: the key plus the ticket name | From a link rule whose template names the file `{key}` (rules made from 0.6 ticket systems do), or by following `[[PROJ-123]]` |
| Note | Anything else: `rollout-plan.md` | First `# heading`, else the file name | From the switcher, or by following a `[[link]]` |

**Archived** notes stay where they are, marked `archived: true` in their properties (`Ctrl+Shift+A`, or setting the status to *closed*).

Markdown files in folders of your own (say `projects\`) are read like any other note; the app just creates new ones in `notes\`. Folders starting with `.` (such as `.git` and `.stash`) are ignored.

**Vaults from before 0.8** kept daily notes in `daily\`, tickets in `tickets\`, archived notes in `archive\`, the template in `templates\` and deleted notes in `.trash\`. Those notes keep working as they are. To move them into the layout above, use the **tidy** offered after upgrading, or **Tidy folders…** in Settings → Vaults:

- It shows what will move before anything does: how many notes, which archived ones will be marked, and which names are taken (those get `-2`).
- Notes keep their file names, so links by title, name or key keep working. Links that spell out an old path (`[[work:tickets/PROJ-1]]`) are updated, in every vault, as are image paths written relative to a note.
- Nothing is overwritten. If anything goes wrong part-way, everything is put back. **Undo** puts it all back too (keeping edits made since), and `.stash\tidy-<time>.json` records what moved where.
- Files in the old folders that aren't notes stay where they are.

**Keys** come from [link rules](#links-and-link-rules) that read bare keys: Jira's pattern `[A-Z][A-Z0-9]+-\d+` matches `PROJ-123` and `OPS-7`. A bare key anywhere in a note is then highlighted, works as a link, and shows in the Links panel. A note about a link doesn't need to be a ticket note: a rule's template can put it in `notes/` or anywhere else.

**Note format.** All front matter fields are optional:

```markdown
---
status: open
tags: [bug, rollout]
primary_link: https://acme.atlassian.net/browse/PROJ-123   # what the note is about (Ctrl+J)
links:                                                     # links not in the text (Add link)
  - https://gerrit.example.com/c/proj/+/12399
---

# PROJ-123 Rollout date question

- 2026-10-22 14:05 asked about rollout date, see [[Rollout plan]]
- 2026-10-22 16:40 fix pushed, waiting on review
```

Notes from 0.6 may instead have `jira:` (where the ticket opens) and link lists such as `gerrit: [12345]`. They keep working as they are, and bare entries open where 0.6 sent them.

The ticket's **name** is whatever follows the key in the first heading. It's shown beside the key in the header, sidebar, switcher and Notes view. The file name stays `PROJ-123.md`, so editing the name never breaks links. Notes from 0.1 that still carry `title:` in front matter keep showing that name until you add one to the heading.

Any note can also list `aliases: [Plan B, launch]`. Links and search then match those names too.

### Vaults

A **vault** is a folder of notes with a name and a color, for example *Personal* and one vault per project. Set them up in Settings → Vaults. Each vault has:
- **its own folder** (any folder, including one in OneDrive or a Git repository);
- **its own daily notes and template**.

[Link rules](#links-and-link-rules) are the same in every vault.

- **One vault is current.** The sidebar, the Notes view, Today (`Ctrl+D`) and new notes belong to it.
  - It shows at the top of the sidebar as a colored badge with the vault's initial.
  - Switch with that row, **`Ctrl+Shift+O`**, or the tray menu's **Vault** submenu.
  - Each vault remembers the note you last had open in it.
- **Opening a note from another vault switches to that vault.** You're always in the vault of the note on screen. Back and forward move across vaults too.
- **Search covers every vault.**
  - **`Ctrl+K`, the `Alt+J` pop-up and the `[[` link picker** all search everywhere, list the current vault's notes first, and tag each result with its vault.
  - **To narrow it**, type `in:work` (a vault's id or the start of its name), or pick a vault tab in the switcher (**`Tab`** moves along them: *All vaults → this vault → each other vault*).
  - **The Notes view** lists the current vault unless you type `in:`.
- **Links between vaults:**
  - **Same vault first:** `[[Plan]]` looks in the linking note's own vault, then in the others.
  - **Naming a vault:** `[[work:Plan]]` looks only in *work*, and picking a note from another vault in the `[[` picker writes it that way.
  - **Ticket keys** resolve in whichever vault has the ticket.
  - **New notes from a `[[link]]`** go in the current vault.
- **Notes created from a link or key** go in the current vault, or the one you pick in the create form.
- **Removing a vault** in Settings only forgets it; its folder and notes are untouched. Each vault's id (`in:work`) is fixed when it's created; renaming changes only the name.

### Statuses

Ticket and general notes can carry one status. Daily notes never do.

| Status | Icon | Meaning | Active |
|---|---|---|---|
| todo | empty circle | Planned, not started | yes |
| open | circle with a dot | Being worked on | yes |
| blocked | slashed circle | Waiting on something | yes |
| backlog | dashed circle | Parked for later | no |
| closed | circle with a check | Done | no |

- **Set it** with `Ctrl+Shift+S`, by clicking the status chip in the properties row, or from **Status** in the [note menu](#the-note-menu). Type to filter or press `1`–`5`, then `Enter`. It works on the open note, the focused sidebar row or the selected Notes view row. It rewrites only the `status:` line.
- **Closed notes are archived.** Setting *closed* also marks the note archived (links keep working). Setting any other status on a closed, archived note unarchives it.
- **Where it shows:** a chip in the properties row, an icon in the status's color on every sidebar, switcher and Notes view row, and counts in the tray tooltip (for example *3 open · 1 blocked*).
- **Filters:** type `@todo`, `@open`, `@blocked`, `@backlog`, `@closed`, `@active` (todo, open and blocked) or `@none` in the switcher or Notes view. They combine with text: `@open rollout`.
- New ticket notes start as **open** (Settings → Notes → *Default status*). New general notes start with none.
- The list, icons, colors and which statuses count as *active* are editable in Settings → Statuses (click a status's icon to pick another). An unknown value in a file is kept and shown as a grey question mark.
- This is your personal status. It isn't read from or written to Jira.

### Tags

Ticket and general notes can carry any number of tags; daily notes can't. Tags live in front matter as `tags: [bug, research]`, the same format other markdown apps use.

- **Add or remove** with **`Ctrl+Shift+T`** (or **+ Add** in the properties row, or **Tags…** in the note menu). Type to filter your tags, `Enter` adds or removes the highlighted one (or creates the tag you typed), `Backspace` removes the last, and `Esc` closes. Changes save as you go.
- **Names** are lower case letters, digits, `-`, `_` and `/` for nesting: `bug/ui` is a `bug` too. A tag can't be only digits.
- **Search:** `#bug` filters in `Ctrl+K`, `Alt+J` and the Notes view. Several tags must all match, and they combine with everything else: `#bug @open in:work login`. While you type `#`, matching tags are suggested; `Tab` completes the first.
- **Creating with tags:** `Create "Login timeout #bug"` makes the note *Login timeout* tagged `bug`.
- **Where they show:** chips in the properties row and on switcher and Notes view rows, and in sidebar tooltips. Click a chip to see all notes with that tag.
- **Settings → Tags** lists every tag with how many notes use it. Give a tag a color (nested tags use their parent's unless they have their own), or rename or remove it on every note at once, with **Undo**.
- Tags are shared across vaults: `#bug` finds bugs everywhere, `#bug in:work` only in *work*.

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
| Front matter | A compact properties row (status, tags, ticket, attached links, **+ Add**). Move the caret into it to edit the YAML. Ticket and general notes show the row even before they have front matter |
| `![description](path)` | An inline image preview. Click it to edit the Markdown |

Tables, math and footnotes stay as plain text. To see raw markdown everywhere, turn off **Live formatting** in Settings → Editor.

### Images

Paste an image with `Ctrl+V`, drop files into the editor, or choose **Insert image** in the note menu (the header's **…**; a pinned note has its own button). PNG, JPEG, GIF, WebP and BMP are supported, up to 20 MB each. Images also work in pinned notes and expanded note previews. Click an image to edit its `![description](path)` Markdown; move the caret away to see it again.

Imported images are copied into the note's vault under `assets/`, with unique names and vault-root links (`/assets/image-….png`) so renaming or archiving the note keeps them working. Include `assets/` when backing up or sharing a vault. Deleting an image from a note or undoing insertion leaves the file on disk, so other notes and redo can still use it. Markdown viewers outside Stash Notes need to resolve these paths from the vault root.

Existing Markdown images can use a note-relative path, a vault-root path, an HTTP(S) URL, or a raster image data URL. Local images must be inside the vault. Remote images require a connection. Turning off **Live formatting** shows the image Markdown. Browser development mode keeps imported images in memory for that session.

### Keyboard shortcuts

Every in-app shortcut can be changed in Settings → Shortcuts. In-app shortcuts take priority over the editor's own bindings.

| Shortcut | Action | Notes |
|---|---|---|
| `Alt+N` (global) | Show or hide the app | Works from any app |
| `Alt+J` (global) | Quick switcher pop-up | See [Quick switcher pop-up](#quick-switcher-pop-up) |
| `Esc` | Close overlay / view, else hide window | Closes the switcher, pickers, Links panel, backlinks list, Notes view and Settings first |
| `Ctrl+K` | Quick switcher and search | Also creates notes. With text selected, links the selection |
| `Ctrl+D` | Today's daily note | Creates it if missing, bringing the previous one's open todos |
| `Ctrl+Shift+D` | Bring open todos from the previous daily note | In a daily note; skips ones it already has |
| `Alt+PgUp` / `Alt+PgDn` | Previous / next daily note | Skips days without one. From another note, `Alt+PgUp` opens the latest |
| `Ctrl+L` | Notes view | The current vault's notes |
| `Ctrl+Shift+O` | Switch vault | Also from the sidebar and the tray |
| `Ctrl+Enter` | Follow link under cursor | Note link, key or URL. A key without a note offers to create one. On a checkbox line with no link, toggles it |
| `Alt+Left` / `Alt+Right` | Back / forward | |
| `Ctrl+J` | Open the note's primary link | A 0.6 ticket note opens its ticket. Without a primary link: the only link, or a picker |
| `Ctrl+G` | Open one of the note's links | A picker when there are several (`1`–`9` pick directly) |
| `Ctrl+Shift+G` | Add a link | Any link, prefilled from the clipboard; it goes in the front matter's `links` |
| `Ctrl+Shift+L` | Links panel | This note's links; again to close |
| `Ctrl+;` | Insert timestamp | Format set in Settings → Editor |
| `Ctrl+Shift+S` | Set status | On the note, focused sidebar row or selected Notes view row |
| `Ctrl+Shift+T` | Add or remove tags | Same targets as status |
| `Ctrl+Shift+B` | Expand or collapse backlinks | Arrows and `Enter` open one |
| `Ctrl+Shift+A` | Archive or unarchive note | 5 s **Undo** toast |
| `Ctrl+\` | Expand or collapse the sidebar | |
| `Ctrl+0` | Focus the sidebar (and back) | |
| `Ctrl+E`, then `P` | Pin or unpin the note | `Ctrl+P` works too if you're still holding Ctrl |
| `Shift+F10` or the menu key | Note menu | On the selected sidebar, Notes view, backlink or `Ctrl+K` row. Right-click works too |
| `Ctrl+,` | Settings | |
| `Ctrl+/` | Shortcut help | |

Also:

| Shortcut | Action |
|---|---|
| `[[` | Link picker: type to filter, `Enter` inserts `[[Title]]` |
| `Ctrl+B` / `Ctrl+I` | Bold / italic |
| `Ctrl+Shift+V` | Paste as typed, without a link rule's formatting |
| `Ctrl+Click` | Follow a link, key or URL with the mouse |
| `F2` | Rename the current note (links to it are updated) |
| `Ctrl+S` | Save now (not needed, since autosave runs anyway) |
| `Ctrl+Shift+Q` | Quit (or use **Quit** in the tray menu) |

### Quick switcher and search

`Ctrl+K` opens one overlay that handles navigation, search and note creation.

- **Empty query** lists recently opened notes, with active notes first. Closed and archived notes are hidden, so this is also your "open tickets" list.
- **Closing:** click outside a popup or press `Esc`. Pressing `Ctrl+K` again also closes the switcher (or your configured switcher shortcut).
- **Typing** updates the results on every keystroke. `↑`/`↓` move, `Enter` opens, `Esc` closes.
- **Create.** When nothing matches exactly, the last row reads `Create "<query>"`.
  - **Links and keys:** a query that is a link, or starts with a key a link rule reads, offers **Create note from link** (any text after it is the title). See [Creating a note from a link](#creating-a-note-from-a-link). If the ticket's note already exists, it's the top result instead.
  - **Anything else** creates a regular note in the current vault, or in the vault you narrowed to.
  - **Filter words** (`#tag`, `@status`, `in:`) are left out of the title, and `#tags` are given to the new note.
- **Every vault** is searched, the current one first; each result shows its vault. The tabs under the search box pick which vaults: *All vaults*, this vault, or each other vault. **`Tab`** moves along them, or completes a `#tag` while you're typing one.
- **`Ctrl+Enter`** (or **all in Notes view** in the footer) opens the [Notes view](#notes-view) with the same query and vaults, to sort, group and scan every result.
- **`Shift+F10`**, the menu key or a right-click on a result opens its [note menu](#the-note-menu).
- **`Alt+A`** includes closed and archived notes, shown dimmed.
- **`Alt+Enter`** *peeks* at a result without adding it to history.
- **Pasting a link** keeps it as you pasted it.

What matches, from strongest to weakest:

| Field | Examples that match | Weight |
|---|---|---|
| Ticket key | `123`, `proj123`, `PROJ-123` | Highest |
| Title and aliases | `rollout`, `rlt pln` (fuzzy) | High |
| Filters | `@open`, `@active`, `#bug`, `in:work` | Narrow the results; combine freely |
| Date | `2026-10-22`, `22/10`, `22/10/2026`, `oct 22`, `today`, `yesterday`, `monday`, `last tuesday`, `tuesday last week`, `3 days ago` | High (matches the daily note). See [Date words](#date-words) |
| Body text | Any word or words from a note, or a timestamp | Normal, with the matching line shown |

#### Date words

When the query reads as a day, the switcher shows that day under the search box, and the day's daily note comes first. If there isn't one, the **Create** row creates it. Weeks start on Monday. Taking Thursday 22 October 2026 as today:

| You type | Day |
|---|---|
| `today`, `yesterday`, `tomorrow`, `day before yesterday` | 22, 21, 23, 20 October |
| `monday`, `thu` | The latest one, today included: 19, 22 October |
| `last monday`, `last thu` | The latest one before today: 19, 15 October |
| `next mon` | The next one after today: 26 October |
| `this week fri`, `friday this week`, `this fri` | Friday of this week: 23 October |
| `last week tuesday`, `tuesday last week` | Tuesday of last week: 13 October |
| `monday next week` | 26 October |
| `3 days ago`, `a week ago`, `2 weeks ago` | 19, 15, 8 October |
| `2 weeks ago monday` | Monday two weeks back: 5 October |
| `oct 22`, `22 october`, `22/10`, `22/10/2026`, `2026-10-22` | 22 October (`22/10` follows your daily note date format's order) |

Typing **only digits** (for example `123`) matches every ticket key ending in those digits. Notes opened in the last 7 days get a boost. There are no regexes, boolean operators or saved searches. For those, search the notes folder with Everything, ripgrep or your editor.

### Sidebar

The sidebar starts as a thin rail. It shows search, **Today**, **All notes**, your pinned and active notes and **Settings**. With more than one vault, a colored badge shows which vault you're in; click it to switch. Hover over an icon to see the note's title. Click the panel button (or press `Ctrl+\`) to expand it:

- **The vault** at the top, when you have several. Click it for a dropdown: each vault (the current one checked), **Open vault folder** and **Manage vaults**. `Ctrl+Shift+O` opens the same list in the middle of the window.
- **Search or create** opens [`Ctrl+K`](#quick-switcher-and-search).
- **Today**, **All notes** and **Links** rows. **Links** opens the Links panel on *All notes*.
- **Pinned:** notes open in [pinned windows](#pinned-notes), from any vault.
- **Active:** notes with an active status, ordered open, blocked, then todo, and by last edit within each. Up to 8 rows, then a **+N more active** row that opens the Notes view filtered to `@active`.
- **Recent:** the last 10 notes you opened, skipping daily notes and anything already listed.
- **Settings**, and **?** for the keyboard shortcuts.
- Where you are is highlighted with an accent bar and tint: **Today** on today's note, **All notes** in the Notes view, **Settings** in Settings, or the open note's row.
- Right-click a note (or click its **…**) for the [note menu](#the-note-menu).
- Drag the right edge to resize it (180–360 px). Its width and expanded state are remembered.
- Below a 720 px window width, the expanded sidebar floats over the editor and closes after you open a note.
- **Keyboard:** `Ctrl+0` focuses it, `↑`/`↓` move, `Enter` opens (and puts the caret in the editor), `Ctrl+Shift+S` sets the row's status, `Shift+F10` opens its note menu, and `Esc` goes back.

### Notes view

`Ctrl+L` (or the sidebar's **All notes** row) shows every note in the current vault in the main area. It's a history entry, so `Alt+Left` comes back to it, and `Esc` leaves it.

- **Search** with the switcher's syntax, including `@status`, `#tag` and `in:vault` filters (`in:` lists another vault, or several).
- **Rows** show the status, key or title, tags, a one-line snippet (the matching line while searching) and when it was last edited.
- **`Alt+S`** cycles the sort: last edited, last opened, status, title.
- **`Alt+G`** cycles grouping: none, by status, by tag (a note with several tags appears under each). `←`/`→` collapse or expand a group header.
- **`Alt+D`** shows or hides daily notes, and **`Tab`** shows or hides closed and archived notes (both hidden by default).
- **`Enter`** opens the selected note. **`Ctrl+Shift+S`** sets its status, and **`Ctrl+E`, `P`** pins it. A **Create** row appears when nothing matches.

Defaults for these are in Settings → Notes view.

### The note menu

Right-click a note wherever it's listed (the sidebar, a Notes view row, a backlink or a `Ctrl+K` result), or click **…** beside a sidebar note or in the header. A menu opens where you clicked, with nothing dimmed:

| Item | Does |
|---|---|
| **Open** | Opens the note (not shown for the note you're in) |
| **Pin beside your work** / **Unpin** | Pops it out into a [pinned window](#pinned-notes), or closes it |
| **Status ›** | Sets its [status](#statuses); `1`–`5` pick directly |
| **Tags…** | Opens the tag picker beside the menu |
| **Open PROJ-123** | Opens the open note's primary link (`Ctrl+J`), when it has one |
| **Links…** | Shows the open note's links in the [Links panel](#links-and-link-rules) |
| **Copy link** | Copies `[[Title]]` (or `[[PROJ-123]]`, or `[[work:Title]]` for another vault's note) |
| **Insert image…** | Picks image files to insert (the open note only) |
| **Rename…** | Renames a regular note; see [Archiving and renaming](#archiving-and-renaming) |
| **Move to ›** | Moves it to another vault |
| **Archive** / **Unarchive** | Marks it archived (or not); the file stays where it is |
| **Delete…** | Asks first, recommending Archive |

Status and Tags don't appear on daily notes. **From the keyboard**, `Shift+F10` or the menu key opens the menu on the selected row. `↑`/`↓` move, `→` or `Enter` opens a submenu, `←` closes it, and `Esc` closes the submenu, then the menu.

### Links, backlinks and history

| Written as | Opens | If the target is missing |
|---|---|---|
| `[[Rollout plan]]` | The note titled "Rollout plan" (case-insensitive), in this note's vault first | Creates `notes/rollout-plan.md` in the current vault |
| `[[work:Rollout plan]]` | That note in the *work* vault | Creates it there |
| `[[PROJ-123]]` | The ticket note `PROJ-123.md`, in whichever vault has it | Creates `notes/PROJ-123.md` in the current vault |
| Bare `PROJ-123` (a key a link rule reads) | Its note, if there is one | Offers to create one from the key's link |
| `[[2026-10-22]]` | The daily note `2026-10-22.md` | Creates it in `notes/` from the daily template |
| `https://...` or `[text](https://...)` | Your default browser | – |

`[[Target|shown text]]` and `[[Target#section]]` also work. **Resolution order:** the linking note's vault first (exact file name, then title, then alias), then the other vaults in order. If two notes share a title, the most recently edited one wins. Links to archived notes still work and are shown dimmed.

**Embedded pages.** Note links have a subtle rounded tint, fitted to the text. Hover a link or Tab to it to reveal a centered arrow; the link widens to make room for it. Click the arrow to expand the linked note beneath that line in a rounded panel with a soft background and full border; click again to collapse it. The arrow stays visible while expanded. This works for links in lists, several links on the same line, and links to other vaults. You can also press Enter or Space on an arrow. The preview shows formatted, read-only content without YAML properties; hover or focus it to reveal **Open note** for editing (in the main window when viewing a pinned note). Nested panels alternate backgrounds to stay visibly separate.

Previews refresh when the linked files change. You can expand further links inside a preview; circular links and deeply nested chains show an **Open note** action instead of expanding indefinitely. A missing note shows an error with **Retry**; expanding it never creates a file. Code examples and front matter do not get disclosure arrows. Expansion lasts until you leave the containing note and does not change its markdown. A `#section` link previews the whole note, matching the existing link navigation behavior.

**Backlinks.** The strip at the bottom lists every note that links to the current one or mentions its ticket key. Links from other vaults count when they name the vault (`[[work:Plan]]`). `Ctrl+Shift+B` expands it into a list.

**History** works like a browser. `Alt+Left` and `Alt+Right` restore each note's cursor and scroll position. It keeps the last 100 entries and is cleared when you quit.

### Links and link rules

**Every link is collected, with no setup.** A link in a note is anything Markdown reads as one: a bare `https://…` URL, `[text](https://…)`, `<https://…>`, a used `[text][ref]` reference, the front matter's `primary_link` and `links`, and a key a link rule reads (`PROJ-123`). Links in code, comments and image sources are left out, and so are internal `[[note links]]`, which work as before.

**The Links panel** opens beside the editor from the properties row's link count, `Ctrl+Shift+L`, **Links…** in the note menu, or the sidebar's **Links** row.

- **This note** shows the open note's links as you type, before anything is saved. Each row has a readable label (the Markdown label, else the rule's label or key, else the URL), the site and the full URL:
  - **Open** in your browser, **Copy** the link, and **Go to source** (selects it in the text). An attached-only link has **Edit** instead.
  - **…** makes it the **primary link** (what `Ctrl+J` opens), removes an attached link, or goes to any of the places it appears.
  - The same URL appearing several times is one row, with a count. Case in the site name and default ports don't make a link different; anything else in the URL does.
  - **Add link** (`Ctrl+Shift+G`) attaches any link to the note's front matter. Making another link primary keeps the old one attached unless the note still mentions it. These are edits to the note, so `Ctrl+Z` undoes them.
- **All notes** shows the links of every vault and the notes that use them, most recently edited first. Filter by text and vault, and **Include archived** to see archived notes' links too. Click a note to open it.
- `↑`/`↓` move between rows and `Esc` closes the panel, returning to the editor.

**Link rules** (Settings → Links) are optional and work in every vault. Each rule recognises one service's links:

| Preset | Recognises | Label | Bare keys |
|---|---|---|---|
| Jira | `https://yourco.atlassian.net/browse/{key}`, and boards with `selectedIssue=` | `PROJ-123` | `[A-Z][A-Z0-9]+-\d+` |
| GitHub issues / pull requests | `…/OWNER/REPO/issues/{n}`, `…/pull/{n}` | `GH-42`, `PR 42` | `GH-\d+` (issues) |
| GitLab issues / merge requests | `…/-/issues/{n}`, `…/-/merge_requests/{n}` | `GL-42`, `MR 42` | `GL-\d+` (issues) |
| Gerrit | `https://gerrit.example.com/c/{n}`, including `/c/project/+/{n}` | `Gerrit 12345` | – |
| Confluence / wiki | page links with a number | `Page 123` | – |
| YouTube (optional) | `youtube.com/watch?v=…`, `youtu.be/…` | `Video …` | – |
| Custom | your link format | yours | optional |

- **Setting one up:** choose a preset and paste an example link from your site; the rule adapts its link format (host, owner and repository) and shows what it recognises in **Try it**. `{key}` is the identifier and `{n}` its number. Regular expressions are under **Advanced**.
- **More specific rules win**, so a pull request rule beats a rule for all of GitHub; ties go to the rule higher in the list. A rule never claims every page of a site just because the host matches.
- **Bare keys** work when a rule has a key pattern. If a key could belong to two rules (say two Jira sites), you're asked which; nothing is guessed.
- **Format pasted links** (off for new rules): pasting just one matching link inserts `[PROJ-123](https://…/PROJ-123)`. Text with several links, or anything pasted with `Ctrl+Shift+V`, goes in exactly as copied. The master switch is at the top of Settings → Links.
- **Note template** (optional): a title, file name, Markdown body, status and tags (the note goes in `notes/` like every other), using `{url}`, `{key}`, `{title}` and `{domain}`. Problems with the placeholders are shown before you save or create.
- Editing or turning off a rule never changes links stored in notes.

#### Creating a note from a link

In `Ctrl+K`, the `Alt+J` pop-up or the Notes view, paste a link or type a key, optionally followed by a title (`PROJ-123 Investigate startup delay`), and choose **Create note from link**. Nothing is created until you confirm. The form shows:

- the **vault** it goes in (the current one, or the one you narrowed to; change it here),
- an editable **title** (from what you typed, else the key, else a name from the URL; no web requests are made),
- the **template's result**: the heading, the file and the body. Without a template it's a plain note,
- notes, in any vault, **already about this link** (with it as their primary link). `Enter` opens the first; **Create another note** (`Ctrl+Enter`) makes a new one anyway.

The new note always keeps the link as its `primary_link`, and existing files are never overwritten (`PROJ-123-2.md`). A template is used only when creating a note: pasting a link into an existing note never changes its title, status or tags.

The app stays open behind the browser when you open a link. If you'd rather it hide, turn on Settings → General → *Hide the window after opening a link*.

### Quick switcher pop-up

Press **`Alt+J`** from any app. A small search box appears near the top of the screen your mouse is on. The main window stays wherever it is, maximised or hidden.

- It's the same switcher as `Ctrl+K`: fuzzy search across every vault by ticket key, ticket name, title, date or text, with `@status`, `#tag` and `in:vault` filters, and your recent and active notes listed when it's empty. The vault you last used comes first, and `Tab` narrows to a vault.
- **`Enter`** opens the selected note in the app.
- When nothing matches, the **Create** row creates the note (in the vault you last used, or the one you narrowed to) and opens it. A pasted link, or `PROJ-1234 Do this thing`, offers [Create note from link](#creating-a-note-from-a-link) in the pop-up first.
- **`+ text`** adds `- [ ] text` to today's daily note (in the vault you last used, or the one you narrowed to with `Tab`), creating the note if needed, and closes the pop-up. It works in `Ctrl+K` too.
- **`Esc`** (or clicking anywhere else) closes the pop-up and returns you to what you were doing.
- `Alt+J` again while it's open closes it. You can change the key in Settings → General.

### Pinned notes

Press **`Ctrl+E` then `P`** to pop the current note out into a small always-on-top window (320 × 400). Do it again, inside the pinned window or on the same note, to close it. It also works on the selected sidebar or Notes view row.

- Both windows show the same live text. You can type in either, and the edits merge.
- Drag it by its title bar. The pin icon turns always-on-top off and on, and **×** closes it. `Esc` does nothing here, so it can't be closed by accident.
- `Ctrl+;`, `Ctrl+J`, `Ctrl+G`, `Ctrl+Shift+G`, `Ctrl+Shift+S`, `Ctrl+Shift+T` and `Ctrl+Enter` work inside it. Note links open in the main window.
- The link button in its title bar (or `Ctrl+Shift+L`) shows the note's Links panel over the text; `Esc` closes it.
- Up to 5 notes can be pinned. Each window's position and size are remembered per note, and pinned windows come back at login (Settings → General).
- Pinned notes are listed under **Pinned** in the sidebar.
- The tray menu has a **Pinned notes** submenu to focus or close each one.
- If the note is renamed or archived, the window follows it.

### Daily notes and todos

`Ctrl+D` opens today's note in the current vault (`notes/YYYY-MM-DD.md`), creating it from a template with a `## Todo` section. Each vault has its own daily notes. The cursor lands at the end of the todo list, ready to type.

- `Enter` at the end of a `- [ ] item` continues the list with a new checkbox.
- `Ctrl+Enter` on a checkbox line (or clicking the box) toggles it.
- If the last note you had open was a **previous** day's daily note, summoning opens today's instead.

**Open todos carry forward.** When today's note is created, the open todos of the vault's most recent earlier daily note come with it. That's yesterday's, or the last day you wrote one, so weekends and holidays don't lose anything.

- An open todo is an unticked `- [ ] item` with some text, anywhere in the note except code blocks. The lines nested under it (sub-bullets, ticked steps, notes) come with it.
- They go into today's `## Todo` list (or `## To do`, `## Todos`), above the empty `- [ ] ` the template leaves, so the cursor still lands ready for a new item. A note without a todo heading gets them in its first list of todos, or under its title.
- In the earlier note each one becomes `- [>] item`: a muted box with an arrow, meaning "moved on". Clicking it (or `Ctrl+Enter`) reopens it. Done items, `[>]` items and empty checkboxes stay where they are.
- An open todo nested under another open todo moves with its parent. One nested under a ticked or plain bullet moves on its own.
- It happens once, when the note is created. Deleting a carried item from today's note doesn't bring it back.
- **`Ctrl+Shift+D`** in a daily note brings the previous daily note's open todos in by hand, skipping any it already has (same text). Use it if you added todos to yesterday's note after today's was made.
- **Adding a todo from anywhere:** `Alt+J`, then `+ chase Sam about the demo` and `Enter`. It goes into today's Todo list the same way carried todos do, above the empty checkbox.
- **How long it's been open:** once a carried todo has been open two days or more, a muted badge after it says how long (`5d`), counted from the daily note it was first written in. Hover it to see that day. It's worked out from your earlier daily notes (following the item back while each day still has it, open or `[>]`), so the file isn't changed. Ticking it, or changing its text, drops the badge.
- **`Alt+PgUp` / `Alt+PgDn`** go to the previous and next daily note in the vault, skipping days without one.
- **Settings → Notes → Open todos in a new daily note:** *move* (the default), *copy* (the earlier note is left as it was), or *leave them*. `Ctrl+Shift+D` copies when the setting says copy, and moves otherwise.

**Date format:** Settings → Notes → **Daily note date** picks how daily notes are named in the header, sidebar, switcher, Notes view, backlinks and pinned windows: `2026-10-22`, `22/10/2026`, `22 Oct 2026`, `Thu 22 Oct 2026`, `Thursday 22 October 2026` or `10/22/2026`. Files are always named `2026-10-22.md`, so a folder listing sorts them by date and `[[2026-10-22]]` links keep working. Search reads typed dates in the same order: with a day-first format, `3/10` is 3 October.

**Template:** edit it in Settings → Notes (pick the vault when you have several). It's saved as `.stash/templates/daily.md` in that vault's folder (a `templates/daily.md` from before 0.8 is used until you next save it). `{{date}}` (in the daily note date format) and `{{weekday}}` are filled in.

### Archiving and renaming

- **`Ctrl+Shift+A`** marks the note archived (`archived: true` in its properties), or unarchives it. The file doesn't move. A note still in a pre-0.8 `archive/` folder moves into `notes/` when unarchived. Archived notes are left out of search unless you press `Tab`, and links to them still work. **Undo** is available for 5 seconds.
- **`F2`** renames a regular note: the file, its `# heading` and every `[[link]]` to it. **Undo** is available for 5 seconds. Ticket names are edited in the heading after the key. Daily notes are named by their date.
- **File actions** are in the [note menu](#the-note-menu): right-click a note anywhere it's listed, or click **…** in the header. `F2` also renames the focused sidebar or Notes view note.
- **Move:** choose a vault from the note menu's **Move to**. The note keeps its filename and relative folder, including when the vaults are on different drives. Existing files are never overwritten. Links to the note are updated, local images are copied into the destination vault, and open editors and pinned windows follow the move. Moving the open note switches its sidebar to the destination vault.
- **Delete:** the confirmation offers **Cancel**, **Archive** (recommended) and **Delete**. An already archived note offers **Keep archived**. After deleting, **Undo** is available for 5 seconds. A recovery copy remains in the original vault's hidden `.stash/trash/` folder, with its original relative path. Restore it manually from there if the Undo notice has expired. Shared image files are kept. Nothing empties `.stash/trash/` automatically, so delete its contents in Explorer when you no longer need them.
- **Created a note by mistake?** The creation notice has a **Delete** button for 10 seconds. It discards that new note immediately, with **Undo** available afterward. This works for notes created in the switcher, Notes view, quick pop-up, or by following a missing link. Opening an existing note never offers this shortcut.

### Window, tray and startup

- **Login:** Stash Notes starts with Windows and stays hidden in the tray, already loaded. Starting it from the Start menu shows the window.
- **`Alt+N`**: if the window is hidden, it shows and focuses the editor. If it's visible and focused, it hides. If it's behind another window, it comes to the front. It never touches pinned windows.
- **Close (X), Minimize and `Esc`** hide to the tray. Quit from the tray menu or with `Ctrl+Shift+Q`.
- **Tray icon:** left-click toggles the window. Right-click opens the menu: *Show, Vault (with more than one), Today's note, Open notes folder (the current vault's), Settings, Pinned notes, Quit*. The tooltip counts your active notes.
- **Single instance:** launching Stash Notes again just shows the running window.
- **Window size and position** are remembered.

---

## Settings

Press **`Ctrl+,`** (or click the sidebar's gear, or use **Settings** in the tray menu). Changes apply immediately, with no Save button.

| Category | Options |
|---|---|
| General | Launch at login · Start hidden in tray · Restore pinned notes · What opens on summon · Show/hide hotkey and quick switcher hotkey (press the keys to set; a key another app owns is refused) · Hide after opening a link |
| Vaults | Each vault's name, color and folder · Open its folder · Reorder · Remove · Add a vault |
| Links | Format pasted links · Link rules for every vault: preset, name, icon, on/off, link format with **Try it**, bare keys, paste format and label, note template, advanced match pattern · Reorder · Remove (with Undo) |
| Window | Show in taskbar · Always on top · Sidebar expanded |
| Editor | Live formatting · Comfortable line width · Font and size · Line height · Spell check · Timestamp format (`yyyy MM dd HH hh mm ss a`) |
| Notes | Default status for new key notes (from a `[[KEY]]` link) · Daily note date format · Open todos in a new daily note (move, copy or leave) · Daily note template (per vault) |
| Notes view | Show daily notes · Show closed notes · Default sort · Group by (none, status, tag) |
| Statuses | Icon, name, color and *active* flag of each status · Reorder · Add · Remove |
| Tags | Every tag with its note count · Color · Rename or remove on every note (with Undo) |
| Shortcuts | Every in-app shortcut: click it and press new keys. Conflicts are refused. Reset per row or all |
| Appearance | Theme (follow Windows, light, dark) · Accent color · Animations (follow Windows, on, off) |

Most categories have **Reset to defaults**. The footer opens `settings.json` and the current vault's folder.

**`settings.json`** (`%APPDATA%\com.lukewestgate.stashnotes\settings.json`) is still the source of truth, and hand edits show up in the screen live. Every key is optional. Keys the app doesn't know are kept. The main keys:

| Key | Default |
|---|---|
| `vaults` | `[{ "id": "personal", "name": "Personal", "path": "", "color": "blue" }]`. `path` empty means `Documents\StashNotes`, and `~` is your user folder |
| `linkRules` | `[]`. Each rule is `{ "id", "name", "icon", "enabled", "url", "pattern", "keyPattern", "label", "pasteFormat", "template": { "enabled", "title", "folder", "filename", "body", "status", "tags" } }` |
| `legacyLinks` | Written by the 0.7 upgrade: each vault's 0.6 ticket systems and link types, kept so old links keep their destinations. Leave it alone |
| `hotkey` / `switcherHotkey` | `Alt+N` / `Alt+J` |
| `onSummon` | `last` (or `daily`) |
| `startHidden`, `autostart`, `restorePinned` | `true` |
| `showInTaskbar`, `alwaysOnTop`, `hideAfterExternalLink` | `false` |
| `defaultStatus` | `open` |
| `dailyDateFormat` | `yyyy-MM-dd`. Tokens: `yyyy` `yy` year, `MMMM` `MMM` `MM` `M` month, `EEEE` `EEE` weekday, `dd` `d` day; anything else is kept. Any format works here, not only the ones Settings lists |
| `carryTodos` | `move` (or `copy`, `off`) |
| `tagColors` | `{}`, e.g. `{ "bug": "red" }` |
| `statuses` | the five statuses above, each `{ "name", "icon", "color", "active" }`. `icon` is a [Lucide](https://lucide.dev/icons) name from the Settings grid, e.g. `circle-dot` |
| `liveFormatting`, `smartPaste`, `readableWidth` | `true` |
| `spellCheck` | `false` |
| `fontFamily`, `fontSize`, `lineHeight` | system UI font, `14`, `1.6` |
| `timestampFormat` | `yyyy-MM-dd HH:mm` |
| `theme`, `accent` | `system`, `blue` |
| `animations` | `system` (or `on`, `off`) |
| `notesView` | `{ "showDaily": false, "showClosed": false, "sort": "edited", "groupBy": "none" }` |
| `keys.*` | see [Keyboard shortcuts](#keyboard-shortcuts). `pinNote` is a two-key chord like `"Ctrl+E P"` |
| `schemaVersion` | managed by the app |

Key and match patterns are regular expressions: escape backslashes in JSON (`"[A-Z]+-\\d+"`).

`session.json` in the same folder holds the current vault, the last open note (overall and per vault), recently opened notes, pinned windows and the sidebar state. It's safe to delete.

---

## Your data and safety

- **Plain files are the source of truth.** Notes are ordinary UTF-8 markdown files. You can edit them in any editor while Stash Notes runs, keep vault folders in Git or OneDrive, and back them up any way you like. The search index lives only in memory and is rebuilt at every start.
- **One writer.** Rust holds the text of each open note, and every window (main and pinned) edits that one copy. Only that copy is written to disk, so two windows can never overwrite each other.
- **Autosave** runs 300 ms after the last change, and on hide, quit, navigate, archive and rename.
- **Crash-safe writes:** each save goes to a temporary file that's then renamed over the original. If another program holds the file open (for example a sync client), it retries briefly and then writes in place.
- **External changes:** if a note changes on disk while it has no unsaved edits, it reloads silently in every window. If it has unsaved edits, your text is kept and a bar offers **Reload from disk** or **Keep mine**.
- **Recoverable file actions.** Archiving and closing only mark a note. Renaming rewrites links and offers **Undo**. Deleting asks for confirmation, offers **Undo**, and keeps a recovery copy in the vault's hidden `.stash/trash/` folder. Tidying a vault into 0.8's layout previews first, never overwrites, and offers **Undo**.
- **Upgrades are safe.** The installer only replaces the app. Notes live in your vault folders and settings in `%APPDATA%`, and neither is touched. When a new version changes what a setting means, it first saves the old file as `settings.json.bak`. Front matter keys the app doesn't know are left alone, and notes you didn't edit are never rewritten (looking at a note's links never changes it). The one exception you choose: moving todos to a new daily note marks them `[>]` in the earlier one (Settings → Notes can copy them instead).
- **Locked-down UI:** the web UI has no direct file system access. Every read and write goes through Rust commands that only accept paths inside a vault folder. Only `http`/`https` URLs can be opened.

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
│   ├── components/             Header, Sidebar, Switcher, NotesView, SettingsView, Backlinks, LinksPanel, LinkCreate,
│   │                           Menu, StatusPicker, KeyRecorder, Help, Prompt, Picker, FileDialog, TidyDialog,
│   │                           StatusIcon, Icon, Highlight, Toasts, EmptyState, TagPicker, TagChip, VaultTag
│   └── lib/
│       ├── api.ts              Typed wrappers for the Rust commands and events
│       ├── buffer.ts           Syncs an editor with its note's Rust buffer (CodeMirror collab)
│       ├── editor.ts           CodeMirror setup: decorations, theme, [[ link picker
│       ├── live.ts             Live markdown formatting and the front matter row
│       ├── page-links.ts/.css  Collapsible linked-page previews and their presentation
│       ├── text.ts             Link/key under the cursor, checkboxes, timestamps
│       ├── keys.ts             Shortcut parsing, matching, chords and recording
│       ├── history.ts          Back/forward history
│       ├── icons.ts            The Lucide icons the app uses, by name
│       ├── highlight.ts        Which letters of a result to highlight
│       ├── links.ts            Link rule presets, example-link adaptation, keys to highlight
│       ├── motion.ts           Animation setting and the shared pop-in transitions
│       ├── popup.ts            Popups: outside-click dismissal and placing them beside what opened them
│       ├── toasts.svelte.ts    Toast queue (timers, pause on hover, sticky errors)
│       ├── status.ts / appearance.ts
│       └── mock.ts             In-memory backend for running the UI in a browser
├── src-tauri/                  Rust (Tauri v2) app
│   ├── src/
│   │   ├── lib.rs              App setup, plugins, window events
│   │   ├── commands.rs         Commands the UI calls
│   │   ├── tidy.rs             Tidying a vault into 0.8's layout: preview, apply, undo
│   │   ├── buffers.rs          One authoritative text buffer per open note
│   │   ├── pins.rs             Pinned note windows
│   │   ├── quick.rs            The Alt+J pop-up window
│   │   ├── config.rs           settings.json (with migrations), indexing, file watching
│   │   ├── window.rs           Show/hide/focus (incl. the Windows foreground fix)
│   │   ├── tray.rs             Tray icon, menu and tooltip
│   │   ├── hotkey.rs           Global hotkeys
│   │   └── state.rs            Shared state and session persistence
│   ├── core/                   `stash-core` (no Tauri dependency; unit tested)
│   │   ├── src/                vault, index & search, query filters (@ # in:), notes, links
│   │   │                       (extraction, rules, templates), front matter, statuses,
│   │   │                       change sets, dates, settings (with migrations), tidying a vault
│   │   └── tests/perf.rs       5,000-note performance check
│   ├── capabilities/           Webview permissions (core events and window dragging only)
│   └── tauri.conf.json         Windows, bundle and security config
└── .github/workflows/          CI: tests on Linux, installer on Windows, releases
```

**Architecture.** Rust owns the files, the search index, the windows and the text of every open note. The web UI draws and sends commands. At startup, Rust scans the notes folder into an in-memory index in a background thread, then watches the folder (and `settings.json`) for changes. Each window opens a note with `buffer_open` and pushes its edits as CodeMirror change sets (`buffer_push`). Rust applies them to the one buffer, broadcasts them to every window showing the note (`buffer-updates`) and autosaves. Other commands cover search (`search`, `list_notes`, `sidebar`, `backlinks`), notes (`create_note`, `set_status`, `archive_note`, `rename_note`), tidying (`tidy_needed`, `tidy_preview`, `tidy_vault`, `tidy_undo`), links (`note_links`, `all_links`, `link_meta_edit`, `link_preview`, `create_from_link`), pinned notes, the pop-up and settings. Links are read from the Markdown with [`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark). Fuzzy matching uses the [`nucleo`](https://github.com/helix-editor/nucleo) matcher.

### Useful commands

```bash
npm run dev            # UI only, in a browser, backed by an in-memory mock (http://localhost:1420)
npm run tauri dev      # the real app with hot reload
npm run check          # Svelte/TypeScript type check
npm test               # UI unit tests (Vitest)
npm run test:e2e        # Editor interaction tests (Playwright; run npx playwright install chromium first)
cd src-tauri && cargo test --workspace                     # Rust unit tests
cd src-tauri && cargo test --release -p stash-core --test perf -- --ignored --nocapture
                                                           # performance check on 5,000 synthetic notes
```

In `npm run dev`, add `?pin=notes/rollout-plan.md` to the URL to see the pinned window, or `?quick` for the `Alt+J` pop-up.

On Windows, use an installed Edge browser for the interaction tests with `$env:PLAYWRIGHT_CHANNEL='msedge'; npm run test:e2e` in PowerShell.

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

**A key or link opens the wrong site.** Check the rule's link format in Settings → Links; **Try it** shows where a key or link goes. Links written into notes as full URLs always open exactly as written.

**My keys aren't highlighted, or `Ctrl+J` does nothing.** Keys need a link rule with **Recognise bare keys** on (Settings → Links). `Ctrl+J` opens the note's primary link, else its only link; a note with none says so.

**My link isn't in the Links panel.** Links in code (`` `…` `` or fenced blocks), comments and image sources are left out on purpose, and a URL needs `https://` or `http://`.

**A note I know exists isn't in the sidebar or Notes view.** They show the current vault. Search (`Ctrl+K`) covers every vault; opening the note switches to its vault.

**My notes disappeared after upgrading to 0.5.** They haven't: your notes folder is now the first vault. Check Settings → Vaults, and **Open notes folder** in the tray.

**A pinned window is off screen.** Close it from the tray menu (**Pinned notes**) and pin the note again.

**Where are my notes?** Use **Open notes folder** in the tray menu (the current vault's), or the folder button on each vault in Settings → Vaults.

**I want to start fresh.** Quit the app, then delete `%APPDATA%\com.lukewestgate.stashnotes`. Your notes folder is never touched.

---

## Design notes and decisions

Stash Notes implements the *Quick Notes (Tauri) — Product & Technical Spec*. Where the spec left questions open, these choices were made:

- **Name and folder.** The app is called *Stash Notes*, and its default notes folder is `Documents\StashNotes`.
- **Closed notes are archived.** Setting *closed* also marks the note archived, and reopening unmarks it (owner's choice on the spec's open question; until 0.8 this moved the file into `archive/`).
- **One notes folder** (0.8, owner's request): every note goes in `notes/` and is typed by its file name; archiving is a property, not a folder; the app's own files are in `.stash/`. Vaults keep their separate folders. Older vaults are tidied only when you agree, with a preview and Undo.
- **Sidebar.** A thin rail with a button that expands it into the full sidebar (owner's choice). The expanded state is remembered.
- **One place per action** (0.8, owner's request): a note's status, tags and links live in its properties row; the header only says where you are and holds Pin and the note menu; every per-note action is in the one note menu, which opens wherever a note is listed. Popups opened from an element open beside it without dimming the window; ones opened with a key stay centered. Only Delete keeps a confirmation dialog.
- **No in-app updater.** The spec's auto-update section is left out at the owner's request. Installing a newer version replaces the old one in place, and only the per-user NSIS installer is shipped, so there's never a second copy. The update-related settings (*Check for updates*, *Install automatically*) and the pre-update vault snapshot are therefore not present.
- **Links are collected, not configured** (0.7 spec): any link works without setup, and link rules (global, in Settings → Links) only add labels, paste formatting and templates. Jira and the rest are presets with placeholder URLs; an example link adapts them to your site. The 0.6 per-vault configuration is kept read-only so old links never change destination.
- **Vaults** (0.5, owner's design): one current vault drives the sidebar, Notes view, Today and new notes; opening a note from another vault switches with it; search covers all vaults (current first) and narrows with `in:` or `Tab`. Daily notes are per vault. Notes can be moved between vaults through file actions, and statuses are shared by all vaults.
- **Vault ids** are fixed when a vault is created so that `in:work`, `[[work:Note]]` and saved note paths keep working after a rename.
- **Switching vault is `Ctrl+Shift+O`**, not `Ctrl+Shift+V`, which is plain paste.
- **Tags** live only in front matter (not `#tags` in the text), and daily notes don't take tags, like statuses.
- **Daily todos don't roll over** (spec v1).
- **Shortcuts beyond the spec:** `F2` rename, `Ctrl+S` save now, `Ctrl+Click` follow, and right-click (or `Shift+F10`) on any listed note for the note menu.
- **Shortcut conflicts** are refused when recording, not just flagged, so two actions can never share a key.
- **Peek (`Alt+Enter`)** shows the chosen note without adding it to history.
- **`Alt+J` is a separate pop-up** rather than the main window with its switcher open, so a maximised app doesn't take over the screen just to search.
- **Animations.** The spec asked for an instant summon, and that's unchanged: showing and hiding the window never animates. Only small in-app transitions (pop-ups, toasts, the sidebar) do, they're short (under 200 ms), and they follow Windows' animation setting unless you choose otherwise.
- **Icons** are a bundled subset of [Lucide](https://lucide.dev) (ISC licence), drawn as inline SVG, so no icon font or network access is needed.
- **Ticket files are named by key only** (`PROJ-1234.md`), and the name lives in the heading. Rewording a ticket never breaks links, and tickets are still found by name everywhere.
