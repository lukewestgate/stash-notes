// CodeMirror 6 setup: plain markdown text with decorations for [[links]],
// ticket keys, URLs and checkboxes, plus the `[[` link picker.

import {
  autocompletion,
  completionKeymap,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { type Extension, type Range, StateEffect } from "@codemirror/state";
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
import { checkboxToggle, URL_RE, WIKI_RE, wikiTarget } from "./text";

export interface EditorHooks {
  onChange(): void;
  /** Follow the reference at `pos`; return true if there was one. */
  follow(pos: number): boolean;
  linkSearch(query: string): Promise<{ hits: SearchHit[]; exact: boolean }>;
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
const frontMatterLine = Decoration.line({ class: "cm-frontmatter" });

/** Last line number of a leading `---` front matter block, or 0. */
function frontMatterEnd(view: EditorView): number {
  const doc = view.state.doc;
  if (doc.lines < 2 || doc.line(1).text.trimEnd() !== "---") return 0;
  for (let n = 2; n <= Math.min(doc.lines, 200); n++) {
    const t = doc.line(n).text.trimEnd();
    if (t === "---" || t === "...") return n;
  }
  return 0;
}

function buildDecorations(view: EditorView): DecorationSet {
  const out: Range<Decoration>[] = [];
  const doc = view.state.doc;
  const fmEnd = frontMatterEnd(view);
  for (const { from, to } of view.visibleRanges) {
    let line = doc.lineAt(from);
    while (true) {
      const text = line.text;
      if (line.number <= fmEnd) out.push(frontMatterLine.range(line.from));
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
      if (u.docChanged || u.viewportChanged || u.transactions.some((tr) => tr.effects.some((e) => e.is(refresh)))) {
        this.decorations = buildDecorations(u.view);
      }
    }
  },
  { decorations: (v) => v.decorations },
);

const highlight = HighlightStyle.define([
  { tag: t.heading1, class: "cm-h cm-h1" },
  { tag: t.heading2, class: "cm-h cm-h2" },
  { tag: [t.heading3, t.heading4, t.heading5, t.heading6], class: "cm-h cm-h3" },
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
    const options: Completion[] = hits.map((h, i) => ({
      label: h.title,
      detail: [h.label, (seen.get(h.title.toLowerCase()) ?? 0) > 1 ? h.path : null, h.archived ? "archived" : null]
        .filter(Boolean)
        .join(" · "),
      info: h.snippet ?? undefined,
      type: h.kind,
      boost: -i,
      apply: insert(h.title),
    }));
    if (query.trim() && !exact) {
      options.push({ label: query.trim(), detail: "new note", type: "create", boost: -1000, apply: insert(query.trim()) });
    }
    return { from, options, filter: false };
  };
}

const layout = EditorView.theme({
  "&": { height: "100%", fontSize: "var(--font-size)" },
  ".cm-scroller": { fontFamily: "inherit", lineHeight: "1.65", padding: "10px 0 40px" },
  ".cm-content": { padding: "0 20px", maxWidth: "980px" },
});

export function editorExtensions(hooks: EditorHooks): Extension[] {
  return [
    layout,
    history(),
    drawSelection(),
    EditorView.lineWrapping,
    markdown(),
    syntaxHighlighting(highlight),
    decorations,
    autocompletion({
      override: [linkCompletions(hooks)],
      icons: false,
      activateOnTyping: true,
      maxRenderedOptions: 30,
    }),
    keymap.of([...completionKeymap, ...defaultKeymap, ...historyKeymap, indentWithTab]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) hooks.onChange();
    }),
    EditorView.domEventHandlers({
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
    EditorView.contentAttributes.of({ spellcheck: "false", autocorrect: "off", autocapitalize: "off" }),
  ];
}
