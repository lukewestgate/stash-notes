// CodeMirror 6 setup: markdown with live formatting (see live.ts),
// decorations for [[links]], ticket keys, URLs and checkboxes, and the `[[`
// link picker.

import {
  autocompletion,
  completionKeymap,
  completionStatus,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { Compartment, EditorState, type Extension, Prec, type Range, StateEffect } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  drawSelection,
  EditorView,
  keymap,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import { api, type SearchHit } from "./api";
import { frontMatterField, frontMatterSyntax, liveConfig, liveDecorations, liveWikiLinks, previewText, toggleWrap } from "./live";
import { isPageLink, pageEmbeds, pageLinkSyntax } from "./page-links";
import { checkboxToggle, refAt, URL_RE, weekday, WIKI_RE, wikiTarget } from "./text";
import { noteName, showsWeekday } from "./daily.svelte";
import "./page-links.css";
import { imageEmbeds, imageInput } from "./images";

export interface EditorHooks {
  onChange(): void;
  /** Follow the reference at `pos`; return true if there was one. */
  follow(pos: number): boolean;
  /** The source note for vault-aware embedded links. */
  path(): string | null;
  openNote(path: string): void;
  linkSearch(query: string): Promise<{ hits: SearchHit[]; exact: boolean }>;
  /** The vault of the note being edited, and a vault's display name. */
  vault?(): string | null;
  vaultName?(id: string): string;
  /** Handle pasted text specially; return true if it was handled. */
  paste(text: string): boolean;
  imageError(message: string): void;
  /**
   * Send the line at `from` (its text was `text`) to a daily note: the day
   * its trailing `@today` / `@fri` names, or today when `now`.
   */
  sendLine?(from: number, text: string, now: boolean): void;
  /** A todo sent to or from a daily note was ticked or unticked (`text` is its line now). */
  todoToggled?(text: string): void;
  /** The days to offer after `@` at the end of a todo, for the words typed so far. */
  dateSuggestions?(typed: string): Promise<{ words: string; date: string }[]>;
  /** The text of a sent todo in the note at `path` was edited: its line `old` is now `now`. */
  todoEdited?(path: string, old: string, now: string): void;
}

/** A sent todo's line: indent, list mark and box; the box; its text; `·` or `→`; the link. */
const SENT_RE = /^(\s*(?:[-*+]|\d+[.)])\s+\[(.)\]\s+)(.*?)\s+(·|→)\s+\[\[([^\[\]\n]+)\]\]\s*$/;

/** `old` -> `now` is an edit inside a sent todo's text (its box and link stay). */
function sentTextEdit(old: string, now: string): boolean {
  const [a, b] = [SENT_RE.exec(old), SENT_RE.exec(now)];
  return !!a && !!b && a[2] === b[2] && a[4] === b[4] && a[5] === b[5];
}

/** A sent todo's text, for telling whether an edit changed it. */
const sentText = (line: string) => SENT_RE.exec(line)?.[3].trim() ?? "";

/** "Fri 30/10/2026", or "30/10/2026, Friday" when the format leaves out the day. */
function dayLabel(iso: string): string {
  const name = noteName({ kind: "daily", title: iso });
  return showsWeekday() ? name : `${name}, ${weekday(iso)}`;
}

/**
 * After ` @` at the end of a line: the days it could mean. Choosing one
 * sends the line to that day's daily note, as Enter would.
 */
function dateCompletions(hooks: EditorHooks) {
  return async (ctx: CompletionContext): Promise<CompletionResult | null> => {
    if (!hooks.dateSuggestions || !hooks.sendLine) return null;
    const line = ctx.state.doc.lineAt(ctx.pos);
    if (ctx.pos !== line.to) return null;
    const m = /\S\s+@([^@]*)$/.exec(line.text);
    if (!m) return null;
    const typed = m[1];
    const list = await hooks.dateSuggestions(typed);
    if (ctx.aborted || !list.length) return null;
    return {
      from: ctx.pos - typed.length,
      filter: false,
      options: list.map((s, i) => ({
        label: s.words,
        detail: dayLabel(s.date),
        type: "date",
        boost: -i,
        apply: (view: EditorView, _c: Completion, from: number, to: number) => {
          view.dispatch({ changes: { from, to, insert: s.words } });
          const l = view.state.doc.lineAt(from);
          hooks.sendLine?.(l.from, l.text, false);
        },
      })),
    };
  };
}

/** A line ending in `@` and a word or two: maybe a day to send it to. */
const SEND_RE = /\s@[\p{L}\d][^@]*$/u;

/** Put `next` in place of the line that read `old`, the one nearest `near`. */
export function replaceLine(view: EditorView, near: number, old: string, next: string): boolean {
  const doc = view.state.doc;
  let best: { from: number; to: number } | null = null;
  for (let n = 1; n <= doc.lines; n++) {
    const l = doc.line(n);
    if (l.text === old && (!best || Math.abs(l.from - near) < Math.abs(best.from - near))) best = l;
  }
  if (best) view.dispatch({ changes: { from: best.from, to: best.to, insert: next } });
  return !!best;
}

/** Lines whose box went between open and done in this update. */
function toggledLines(u: ViewUpdate): string[] {
  const out: string[] = [];
  u.changes.iterChanges((fromA, _toA, fromB, toB) => {
    const after = u.state.doc.lineAt(fromB);
    if (toB > after.to) return;
    const was = checkboxToggle(u.startState.doc.lineAt(fromA).text);
    const now = checkboxToggle(after.text);
    if (was && now && !was.moved && !now.moved && was.done !== now.done) out.push(after.text);
  });
  return out;
}

/** What the decorations need from settings and the index. */
export const decoConfig = {
  tickets: /\b(?:[A-Z][A-Z0-9]+-\d+)\b/g,
  archived: new Set<string>(),
  /** Open todos carried from earlier daily notes, by item text (lower case). */
  todoAges: new Map<string, { since: string; days: number }>(),
};

/** Todos carried along for this many days or more show how long. */
const AGE_FROM = 2;

/** `5d` after a todo carried from earlier daily notes. */
class AgeWidget extends WidgetType {
  constructor(
    readonly days: number,
    readonly since: string,
  ) {
    super();
  }
  eq(o: AgeWidget) {
    return o.days === this.days && o.since === this.since;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-todo-age";
    s.textContent = `${this.days}d`;
    s.title = `Open since ${noteName({ kind: "daily", title: this.since })} (${this.days} days)`;
    return s;
  }
}

const refresh = StateEffect.define<null>();

/** Re-run decorations after `decoConfig` changed. */
export function refreshDecorations(view: EditorView) {
  view.dispatch({ effects: refresh.of(null) });
}

const wikiMark = Decoration.mark({ class: "cm-wikilink" });
const wikiDim = Decoration.mark({ class: "cm-wikilink cm-archived" });
const ticketMark = Decoration.mark({ class: "cm-ticket" });
const ticketDim = Decoration.mark({ class: "cm-ticket cm-archived" });
const urlMark = Decoration.mark({ class: "cm-url" });
const boxOpen = Decoration.mark({ class: "cm-checkbox" });
const boxDone = Decoration.mark({ class: "cm-checkbox cm-checkbox-done" });
const boxMoved = Decoration.mark({ class: "cm-checkbox cm-checkbox-moved" });
const doneLine = Decoration.line({ class: "cm-done-line" });
const movedLine = Decoration.line({ class: "cm-moved-line" });

function buildDecorations(view: EditorView): DecorationSet {
  const doc = view.state.doc;
  const live = liveConfig.enabled ? liveDecorations(view) : null;
  const out: Range<Decoration>[] = live ? live.decos : [];
  for (const { from, to } of view.visibleRanges) {
    let line = doc.lineAt(from);
    while (true) {
      const text = line.text;
      if (live?.codeLines.has(line.from)) {
        if (line.to >= to || line.number >= doc.lines) break;
        line = doc.line(line.number + 1);
        continue;
      }
      if (live) liveWikiLinks(view.state, line.from, text, out);
      const taken: [number, number][] = [];
      const free = (a: number, b: number) => !taken.some(([x, y]) => a < y && b > x);

      for (const m of text.matchAll(WIKI_RE)) {
        const a = m.index!;
        const b = a + m[0].length;
        taken.push([a, b]);
        if (!isPageLink(view.state, line.from + a + 2)) continue;
        const dim = decoConfig.archived.has(wikiTarget(m[1]).toLowerCase());
        out.push((dim ? wikiDim : wikiMark).range(line.from + a, line.from + b));
      }
      for (const m of text.matchAll(URL_RE)) {
        const url = m[0].replace(/[.,;:!?)]+$/, "");
        const a = m.index!;
        const b = a + url.length;
        if (!free(a, b)) continue;
        taken.push([a, b]);
        out.push(urlMark.range(line.from + a, line.from + b));
      }
      for (const m of text.matchAll(new RegExp(decoConfig.tickets.source, "g"))) {
        const a = m.index!;
        const b = a + m[0].length;
        if (!free(a, b)) continue;
        const dim = decoConfig.archived.has(m[0].toLowerCase());
        out.push((dim ? ticketDim : ticketMark).range(line.from + a, line.from + b));
      }
      const box = checkboxToggle(text);
      if (box) {
        out.push((box.done ? boxDone : box.moved ? boxMoved : boxOpen).range(line.from + box.at - 1, line.from + box.at + 2));
        if (box.done) out.push(doneLine.range(line.from));
        if (box.moved) out.push(movedLine.range(line.from));
        const age = !box.done && !box.moved ? decoConfig.todoAges.get(text.slice(box.at + 2).trim().toLowerCase()) : undefined;
        if (age && age.days >= AGE_FROM) {
          out.push(Decoration.widget({ widget: new AgeWidget(age.days, age.since), side: 1 }).range(line.to));
        }
      }
      if (line.to >= to || line.number >= doc.lines) break;
      line = doc.line(line.number + 1);
    }
  }
  return Decoration.set(out, true);
}

const decorations = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = buildDecorations(view);
    }
    update(u: ViewUpdate) {
      if (
        u.docChanged ||
        u.viewportChanged ||
        (liveConfig.enabled && u.selectionSet) ||
        u.transactions.some((tr) => tr.effects.some((e) => e.is(refresh)))
      ) {
        this.decorations = buildDecorations(u.view);
      }
    }
  },
  { decorations: (v) => v.decorations },
);

// Headings are only bolded here; live mode sizes them per line (live.ts).
const highlight = HighlightStyle.define([
  { tag: [t.heading1, t.heading2, t.heading3, t.heading4, t.heading5, t.heading6], class: "cm-h" },
  { tag: t.strong, fontWeight: "600" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strikethrough, textDecoration: "line-through" },
  { tag: t.monospace, class: "cm-code" },
  { tag: [t.processingInstruction, t.meta, t.contentSeparator], class: "cm-meta" },
  { tag: t.quote, class: "cm-quote" },
]);

function linkCompletions(hooks: EditorHooks) {
  return async (ctx: CompletionContext): Promise<CompletionResult | null> => {
    const m = ctx.matchBefore(/\[\[[^\[\]\n|#]*$/);
    if (!m) return null;
    const query = m.text.slice(2);
    const { hits, exact } = await hooks.linkSearch(query);
    if (ctx.aborted) return null;
    const from = m.from + 2;
    const closed = ctx.state.sliceDoc(ctx.pos, ctx.pos + 2) === "]]";
    const insert = (text: string): Completion["apply"] => (view, _c, a, b) => {
      const end = closed ? b + 2 : b;
      view.dispatch({ changes: { from: a, to: end, insert: `${text}]]` }, selection: { anchor: a + text.length + 2 } });
    };
    const seen = new Map<string, number>();
    for (const h of hits) seen.set(h.title.toLowerCase(), (seen.get(h.title.toLowerCase()) ?? 0) + 1);
    const here = hooks.vault?.() ?? null;
    const options: Completion[] = hits.map((h, i) => {
      // A note in another vault is linked as [[vault:Title]], so it can't be
      // mistaken for a same-named note here. Ticket keys resolve anywhere.
      const elsewhere = here !== null && h.vault !== here;
      return {
        // Tickets show their name too ("PROJ-1234 Do this thing"); the link stays [[PROJ-1234]].
        label: h.label ? `${h.title} ${h.label}` : noteName(h),
        detail: [
          elsewhere ? (hooks.vaultName?.(h.vault) ?? h.vault) : null,
          (seen.get(h.title.toLowerCase()) ?? 0) > 1 ? h.path.slice(h.path.indexOf(":") + 1) : null,
          h.archived ? "archived" : null,
        ]
          .filter(Boolean)
          .join(" · "),
        info: h.snippet ?? undefined,
        type: h.kind,
        boost: -i,
        apply: insert(elsewhere && h.kind !== "ticket" ? `${h.vault}:${h.title}` : h.title),
      };
    });
    if (query.trim() && !exact) {
      options.push({ label: query.trim(), detail: "new note", type: "create", boost: -1000, apply: insert(query.trim()) });
    }
    return { from, options, filter: false };
  };
}

const layout = EditorView.theme({
  "&": { height: "100%", fontSize: "var(--font-size)" },
  ".cm-scroller": { fontFamily: "var(--editor-font)", lineHeight: "var(--line-height)", padding: "18px 0 40px" },
  // A centred column (Settings > Editor > Comfortable line width).
  ".cm-content": { padding: "0 24px", maxWidth: "var(--content-max, none)", margin: "0 auto", width: "100%" },
});

/**
 * Caret and selection colors come from theme variables, and the theme is
 * flagged dark when dark so CodeMirror's own defaults flip too. (Without
 * this, the default caret is black, which vanishes on a dark background.)
 */
function colorTheme(dark: boolean) {
  return EditorView.theme(
    {
      ".cm-content": { caretColor: "var(--fg)" },
      ".cm-cursor, .cm-dropCursor": { borderLeft: "2px solid var(--fg)" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "color-mix(in srgb, var(--accent) 30%, transparent) !important" },
    },
    { dark },
  );
}

const colors = new Compartment();
const spell = new Compartment();

/** Switch the editor between light and dark. */
export function setEditorDark(view: EditorView, dark: boolean) {
  view.dispatch({ effects: colors.reconfigure(colorTheme(dark)) });
}

/** Turn live formatting on or off. */
export function setLiveFormatting(view: EditorView, on: boolean) {
  liveConfig.enabled = on;
  refreshDecorations(view);
}

/** Turn the browser spell checker on or off. */
export function setSpellCheck(view: EditorView, on: boolean) {
  view.dispatch({ effects: spell.reconfigure(spellAttrs(on)) });
}

function spellAttrs(on: boolean) {
  return EditorView.contentAttributes.of({ spellcheck: String(on), autocorrect: "off", autocapitalize: "off" });
}

export const editorState = { dark: false, spellCheck: false };

function embeddedPages(path: () => string | null, open: (path: string) => void, ancestors?: string[]): Extension[] {
  return pageEmbeds({
    path,
    open,
    render(parent, text, notePath, parents) {
      // Properties belong to the containing note. Hide the preview's YAML
      // rather than displaying interactive status/tag controls for the parent.
      const body = previewText(text);
      return new EditorView({ parent, doc: body, extensions: [
        EditorState.readOnly.of(true),
        EditorView.editable.of(false),
        EditorView.lineWrapping,
        markdown({ base: markdownLanguage, extensions: [frontMatterSyntax, pageLinkSyntax] }),
        syntaxHighlighting(highlight),
        decorations,
        imageEmbeds(() => notePath),
        embeddedPages(() => notePath, open, parents),
        EditorView.domEventHandlers({
          mousedown(event, view) {
            if (!event.ctrlKey && !event.metaKey) return false;
            const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
            if (pos == null) return false;
            const line = view.state.doc.lineAt(pos);
            const ref = refAt(line.text, pos - line.from, decoConfig.tickets);
            if (!ref) return false;
            event.preventDefault();
            if (ref.kind === "url") api.openUrl(ref.target).catch(() => {});
            else api.resolveLink(ref.target, notePath).then(open, () => {});
            return true;
          },
        }),
      ] });
    },
  }, ancestors);
}

export function editorExtensions(hooks: EditorHooks): Extension[] {
  // Edits to a sent todo's text, passed on once typing pauses (or another
  // line is edited), so the other end isn't rewritten on every key.
  let pending: { path: string; old: string; now: string } | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const flushEdit = () => {
    clearTimeout(timer);
    if (pending && sentText(pending.now) && sentText(pending.old) !== sentText(pending.now)) hooks.todoEdited?.(pending.path, pending.old, pending.now);
    pending = null;
  };
  const trackTextEdits = (u: ViewUpdate) => {
    u.changes.iterChanges((fromA, _toA, fromB, toB) => {
      const after = u.state.doc.lineAt(fromB);
      if (toB > after.to) return;
      const before = u.startState.doc.lineAt(fromA).text;
      if (pending && pending.now === before && SENT_RE.exec(after.text)) {
        pending.now = after.text;
      } else if (sentTextEdit(before, after.text)) {
        const path = hooks.path();
        if (!path) return;
        flushEdit();
        pending = { path, old: before, now: after.text };
      } else return;
      clearTimeout(timer);
      timer = setTimeout(flushEdit, 1000);
    });
  };
  return [
    layout,
    colors.of(colorTheme(editorState.dark)),
    history(),
    drawSelection(),
    EditorView.lineWrapping,
    markdown({ base: markdownLanguage, extensions: [frontMatterSyntax, pageLinkSyntax] }),
    syntaxHighlighting(highlight),
    decorations,
    embeddedPages(hooks.path, hooks.openNote),
    imageEmbeds(hooks.path),
    imageInput(hooks),
    frontMatterField,
    autocompletion({
      override: [linkCompletions(hooks), dateCompletions(hooks)],
      icons: false,
      activateOnTyping: true,
      maxRenderedOptions: 30,
    }),
    // Enter at the end of `… @today`: send the line, then carry on as usual
    // (ahead of the markdown keymap, which continues lists).
    Prec.highest(
      keymap.of([
        {
          key: "Enter",
          run: (view) => {
            const sel = view.state.selection.main;
            const line = view.state.doc.lineAt(sel.head);
            // With suggestions open, Enter picks one (and that sends the line).
            if (completionStatus(view.state) === "active") return false;
            if (hooks.sendLine && sel.empty && sel.head === line.to && SEND_RE.test(line.text)) hooks.sendLine(line.from, line.text, false);
            return false;
          },
        },
      ]),
    ),
    keymap.of([
      ...completionKeymap,
      { key: "Mod-b", run: toggleWrap("**") },
      { key: "Mod-i", run: toggleWrap("*") },
      ...defaultKeymap,
      ...historyKeymap,
      indentWithTab,
    ]),
    EditorView.updateListener.of((u) => {
      if (!u.docChanged) return;
      hooks.onChange();
      if (hooks.todoToggled) for (const line of toggledLines(u)) if (/ (·|→) \[\[[^\]]+\]\]\s*$/.test(line)) hooks.todoToggled(line);
      if (hooks.todoEdited) trackTextEdits(u);
    }),
    EditorView.domEventHandlers({
      paste(e) {
        const text = e.clipboardData?.getData("text/plain");
        if (text && hooks.paste(text)) {
          e.preventDefault();
          return true;
        }
        return false;
      },
      mousedown(e, view) {
        const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
        if (pos == null) return false;
        if (e.ctrlKey || e.metaKey) {
          if (hooks.follow(pos)) {
            e.preventDefault();
            return true;
          }
          return false;
        }
        const target = e.target as HTMLElement;
        if (target.closest(".cm-checkbox")) {
          const line = view.state.doc.lineAt(pos);
          const box = checkboxToggle(line.text);
          if (box) {
            view.dispatch({ changes: { from: line.from + box.at, to: line.from + box.at + 1, insert: box.insert } });
            e.preventDefault();
            return true;
          }
        }
        return false;
      },
    }),
    spell.of(spellAttrs(editorState.spellCheck)),
  ];
}
