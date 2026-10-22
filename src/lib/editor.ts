// CodeMirror 6 setup: markdown with live formatting (see live.ts),
// decorations for [[links]], ticket keys, URLs and checkboxes, and the `[[`
// link picker.

import {
  autocompletion,
  completionKeymap,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { Compartment, type Extension, type Range, StateEffect } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  drawSelection,
  EditorView,
  keymap,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";
import { tags as t } from "@lezer/highlight";
import type { SearchHit } from "./api";
import { frontMatterField, frontMatterSyntax, liveConfig, liveDecorations, liveWikiLinks, toggleWrap } from "./live";
import { checkboxToggle, URL_RE, WIKI_RE, wikiTarget } from "./text";

export interface EditorHooks {
  onChange(): void;
  /** Follow the reference at `pos`; return true if there was one. */
  follow(pos: number): boolean;
  linkSearch(query: string): Promise<{ hits: SearchHit[]; exact: boolean }>;
  /** The vault of the note being edited, and a vault's display name. */
  vault?(): string | null;
  vaultName?(id: string): string;
  /** Handle pasted text specially; return true if it was handled. */
  paste(text: string): boolean;
}

/** What the decorations need from settings and the index. */
export const decoConfig = {
  tickets: /\b(?:[A-Z][A-Z0-9]+-\d+)\b/g,
  archived: new Set<string>(),
};

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
const doneLine = Decoration.line({ class: "cm-done-line" });

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
        const done = box.insert === " ";
        out.push((done ? boxDone : boxOpen).range(line.from + box.at - 1, line.from + box.at + 2));
        if (done) out.push(doneLine.range(line.from));
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
        label: h.label ? `${h.title} ${h.label}` : h.title,
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

export function editorExtensions(hooks: EditorHooks): Extension[] {
  return [
    layout,
    colors.of(colorTheme(editorState.dark)),
    history(),
    drawSelection(),
    EditorView.lineWrapping,
    markdown({ base: markdownLanguage, extensions: [frontMatterSyntax] }),
    syntaxHighlighting(highlight),
    decorations,
    frontMatterField,
    autocompletion({
      override: [linkCompletions(hooks)],
      icons: false,
      activateOnTyping: true,
      maxRenderedOptions: 30,
    }),
    keymap.of([
      ...completionKeymap,
      { key: "Mod-b", run: toggleWrap("**") },
      { key: "Mod-i", run: toggleWrap("*") },
      ...defaultKeymap,
      ...historyKeymap,
      indentWithTab,
    ]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) hooks.onChange();
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
