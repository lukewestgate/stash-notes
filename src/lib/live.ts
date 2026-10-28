// Live markdown formatting (spec section 11): markers such as `#`, `**` and
// `[[ ]]` hide when the caret is elsewhere and reappear when it moves into
// them. The document itself always stays plain markdown.

import { syntaxTree } from "@codemirror/language";
import { EditorSelection, EditorState, type Range, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import type { MarkdownConfig } from "@lezer/markdown";
import { checkboxToggle, normalizeTag, WIKI_RE } from "./text";
import { iconElement } from "./icons";
import { isPageLink } from "./page-links";
import { anchorOf, type Anchor } from "./popup";

/** Parse a leading `---` block as front matter, so YAML isn't read as markdown. */
export const frontMatterSyntax: MarkdownConfig = {
  defineNodes: [{ name: "FrontMatter", block: true }],
  parseBlock: [
    {
      name: "FrontMatter",
      before: "HorizontalRule",
      parse(cx, line) {
        if (cx.lineStart !== 0 || line.text.trimEnd() !== "---") return false;
        const start = cx.lineStart;
        let end = start + line.text.length;
        while (cx.nextLine()) {
          end = cx.lineStart + line.text.length;
          if (/^(---|\.\.\.)\s*$/.test(line.text)) {
            cx.nextLine();
            break;
          }
        }
        cx.addElement(cx.elt("FrontMatter", start, end));
        return true;
      },
    },
  ],
};

/** Range of the front matter block (both fences included), if any. */
export function frontMatterRange(state: EditorState): { from: number; to: number } | null {
  const doc = state.doc;
  if (doc.lines < 2 || doc.line(1).text.trimEnd() !== "---") return null;
  for (let n = 2; n <= Math.min(doc.lines, 200); n++) {
    const t = doc.line(n).text.trimEnd();
    if (t === "---" || t === "...") return { from: 0, to: doc.line(n).to };
  }
  return null;
}

/** Preview the body without YAML, using normalized editor offsets for CRLF files. */
export function previewText(text: string): string {
  const state = EditorState.create({ doc: text });
  const fm = frontMatterRange(state);
  return fm ? state.sliceDoc(fm.to).replace(/^(?:[ \t]*\n)+/, "") : state.doc.toString();
}

/** Does the selection touch [from, to]? */
function touches(state: EditorState, from: number, to: number): boolean {
  if (state.readOnly) return false;
  return state.selection.ranges.some((r) => r.from <= to && r.to >= from);
}

function lineTouched(state: EditorState, pos: number): boolean {
  const line = state.doc.lineAt(pos);
  return touches(state, line.from, line.to);
}

class BulletWidget extends WidgetType {
  eq() {
    return true;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-bullet";
    s.textContent = "•";
    return s;
  }
}

/** A todo's box: open, done, or moved to a later daily note (`[>]`). */
type TaskState = "open" | "done" | "moved";

class CheckboxWidget extends WidgetType {
  constructor(
    readonly state: TaskState,
    readonly markPos: number,
  ) {
    super();
  }
  eq(o: CheckboxWidget) {
    return o.state === this.state && o.markPos === this.markPos;
  }
  toDOM(view: EditorView) {
    const box = document.createElement("span");
    box.className = "cm-task" + (this.state === "open" ? "" : ` cm-task-${this.state}`);
    box.setAttribute("role", "checkbox");
    box.setAttribute("aria-checked", this.state === "done" ? "true" : this.state === "moved" ? "mixed" : "false");
    if (this.state === "moved") box.title = "Moved to a later daily note";
    box.addEventListener("mousedown", (e) => {
      e.preventDefault();
      if (view.state.readOnly) return;
      view.dispatch({ changes: { from: this.markPos, to: this.markPos + 1, insert: this.state === "open" ? "x" : " " } });
    });
    return box;
  }
  ignoreEvent() {
    return true;
  }
}

class RuleWidget extends WidgetType {
  eq() {
    return true;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-rule";
    return s;
  }
}

export interface FrontMatterInfo {
  status: string | null;
  /** How many entries each list-like key has (`gerrit: [1, 2]` -> 2). */
  counts: Record<string, number>;
  aliases: string[];
  tags: string[];
}

/** The pieces of front matter the properties row shows. */
export function readFrontMatter(text: string): FrontMatterInfo {
  const status = /^status:[ \t]*([^\s#]+)/m.exec(text)?.[1] ?? null;
  const counts: Record<string, number> = {};
  let key: string | null = null;
  for (const line of text.split("\n")) {
    const item = /^[ \t]+-[ \t]*(\S.*)?$/.exec(line);
    if (item && key) {
      if (item[1]) counts[key] = (counts[key] ?? 0) + 1;
      continue;
    }
    const kv = /^([A-Za-z0-9_-]+):[ \t]*(.*?)[ \t]*(?:#.*)?$/.exec(line);
    if (!kv) {
      key = null;
      continue;
    }
    key = kv[1];
    const v = kv[2];
    counts[key] = v.startsWith("[") ? v.slice(1, -1).split(",").filter((x) => x.trim()).length : v ? 1 : 0;
  }
  const aliasLine = /^aliases:[ \t]*\[(.*)\]/m.exec(text)?.[1];
  const aliases = aliasLine ? aliasLine.split(",").map((x) => x.trim().replace(/^["']|["']$/g, "")).filter(Boolean) : [];
  const tagBlock = /^tags:[ \t]*(\[.*\]|[^\n]*)\n?((?:[ \t]+-[^\n]*\n?)*)/m.exec(text);
  const rawTags = tagBlock
    ? [...tagBlock[1].replace(/^\[|\]$/g, "").split(","), ...tagBlock[2].split("\n").map((l) => l.replace(/^\s*-\s*/, ""))]
    : [];
  const tags = [...new Set(rawTags.map((t) => normalizeTag(t.replace(/^["']|["']$/g, ""))).filter((t): t is string => !!t))];
  return { status: status?.toLowerCase() ?? null, counts, aliases, tags };
}

/** What the front matter row needs from the app. */
export interface PropertiesHooks {
  statusView(status: string): { icon: string; color: string; name: string };
  /** Open the status picker beside `anchor` (the chip that was clicked). */
  onStatus(anchor: Anchor): void;
  /** The row's + Add: tags, or a link (beside `anchor`). */
  onAdd(anchor: Anchor): void;
  /** Show the notes with this tag. */
  onTag(tag: string): void;
  tagColor(tag: string): string | null;
  /** Open the note's primary link / its links. */
  onPrimary(): void;
  onLinks(): void;
  /** How many links the note has, and its primary one. */
  links(): { count: number; primary: { label: string; icon: string } | null };
  /** Show the row on a note with no front matter yet (not on daily notes). */
  showEmpty(): boolean;
}

export const propertiesHooks: { current: PropertiesHooks | null } = { current: null };

class PropertiesWidget extends WidgetType {
  /** What the row shows: the front matter, plus the note's links. */
  readonly sig: string;
  constructor(readonly info: FrontMatterInfo) {
    super();
    const hooks = propertiesHooks.current;
    this.sig = JSON.stringify([info, hooks?.links()]);
  }
  eq(o: PropertiesWidget) {
    return o.sig === this.sig;
  }
  toDOM() {
    const hooks = propertiesHooks.current;
    const row = document.createElement("div");
    row.className = "cm-properties";
    const button = (cls: string, text: string, title: string, run?: (anchor: Anchor) => void, icon?: string) => {
      const b = document.createElement("button");
      b.className = cls;
      if (icon) b.append(iconElement(icon, 13));
      b.append(text);
      b.title = title;
      b.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        run?.(anchorOf(b));
      });
      row.appendChild(b);
      return b;
    };
    const st = this.info.status ? hooks?.statusView(this.info.status) : null;
    const chip = button(
      "cm-prop cm-prop-status",
      st ? st.name : "status",
      "Set status",
      (a) => hooks?.onStatus(a),
      st?.icon ?? "circle-dashed",
    );
    if (st) chip.style.color = `var(--c-${st.color})`;
    for (const t of this.info.tags) {
      const b = button("cm-prop cm-prop-tag", `#${t}`, `Notes tagged #${t}`, () => hooks?.onTag(t));
      const c = hooks?.tagColor(t);
      if (c) b.style.setProperty("--c", `var(--c-${c})`);
    }
    // Where the note points: its primary link, and all its links (the Links panel).
    const links = hooks?.links();
    if (links?.primary || links?.count) {
      const divider = document.createElement("span");
      divider.className = "cm-prop-divider";
      row.appendChild(divider);
    }
    if (links?.primary) {
      const b = button("cm-prop", links.primary.label, `Open ${links.primary.label}`, () => hooks?.onPrimary(), links.primary.icon);
      b.setAttribute("aria-label", `Open ${links.primary.label}`);
    }
    if (links?.count) button("cm-prop", `${links.count} ${links.count === 1 ? "link" : "links"}`, "This note's links", () => hooks?.onLinks(), "link");
    button("cm-prop cm-prop-add", "Add", "Add a tag or a link", (a) => hooks?.onAdd(a), "plus");
    if (this.info.aliases.length) {
      const a = document.createElement("span");
      a.className = "cm-prop-text";
      a.textContent = `aka ${this.info.aliases.join(", ")}`;
      row.appendChild(a);
    }
    return row;
  }
  ignoreEvent() {
    return false;
  }
}

/** Live mode on/off; read by the decoration builders. */
export const liveConfig = { enabled: true };

/** The front matter row is a block replacement, so it has to live in a state field. */
export const frontMatterField = StateField.define<DecorationSet>({
  create: (state) => buildFrontMatter(state),
  update(deco, tr) {
    if (tr.docChanged || tr.selection || tr.effects.length) return buildFrontMatter(tr.state);
    return deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});

const frontMatterLine = Decoration.line({ class: "cm-frontmatter" });

const NO_FRONT_MATTER: FrontMatterInfo = { status: null, counts: {}, aliases: [], tags: [] };

function buildFrontMatter(state: EditorState): DecorationSet {
  const fm = frontMatterRange(state);
  if (!fm) {
    // Status, tags and links live only in this row, so a note gets it before it has any.
    if (!liveConfig.enabled || !propertiesHooks.current?.showEmpty()) return Decoration.none;
    return Decoration.set([Decoration.widget({ widget: new PropertiesWidget(NO_FRONT_MATTER), block: true, side: -1 }).range(0)]);
  }
  if (!liveConfig.enabled || touches(state, fm.from, fm.to)) {
    const lines: Range<Decoration>[] = [];
    for (let n = 1; n <= state.doc.lineAt(fm.to).number; n++) lines.push(frontMatterLine.range(state.doc.line(n).from));
    return Decoration.set(lines);
  }
  const info = readFrontMatter(state.sliceDoc(fm.from, fm.to));
  return Decoration.set([Decoration.replace({ widget: new PropertiesWidget(info), block: true }).range(fm.from, fm.to)]);
}

const hide = Decoration.replace({});
const bullet = Decoration.replace({ widget: new BulletWidget() });
const rule = Decoration.replace({ widget: new RuleWidget() });
const headingLine = [1, 2, 3, 4, 5, 6].map((n) => Decoration.line({ class: `cm-hl cm-hl${n}` }));
const quoteLine = Decoration.line({ class: "cm-quote-line" });
const codeLine = Decoration.line({ class: "cm-codeblock" });
const fenceLine = Decoration.line({ class: "cm-codeblock cm-codefence" });
const linkText = Decoration.mark({ class: "cm-mdlink" });
const indentMark = Decoration.mark({ class: "cm-indent" });

/** Hide [from, to) plus one following space, if there is one. */
function hideWithSpace(state: EditorState, out: Range<Decoration>[], from: number, to: number) {
  const end = state.sliceDoc(to, to + 1) === " " ? to + 1 : to;
  if (end > from) out.push(hide.range(from, end));
}

/**
 * Live-formatting decorations for the visible ranges. Returns the
 * decorations and the line starts that belong to code blocks (where link
 * and key styling should not apply).
 */
export function liveDecorations(view: EditorView): { decos: Range<Decoration>[]; codeLines: Set<number> } {
  const state = view.state;
  const out: Range<Decoration>[] = [];
  const codeLines = new Set<number>();
  const fm = frontMatterRange(state);
  const tree = syntaxTree(state);
  for (const { from, to } of view.visibleRanges) {
    tree.iterate({
      from,
      to,
      enter(node) {
        if (fm && node.from <= fm.to && node.to >= fm.from && node.name !== "Document") return false;
        const name = node.name;
        const m = /^ATXHeading(\d)$/.exec(name) ?? /^SetextHeading(\d)$/.exec(name);
        if (m) {
          out.push(headingLine[Number(m[1]) - 1].range(state.doc.lineAt(node.from).from));
          return;
        }
        switch (name) {
          case "HeaderMark":
            if (!lineTouched(state, node.from) && state.doc.lineAt(node.from).from === node.from)
              hideWithSpace(state, out, node.from, node.to);
            else if (!lineTouched(state, node.from)) out.push(hide.range(node.from, node.to));
            return;
          case "Emphasis":
          case "StrongEmphasis":
          case "InlineCode":
          case "Strikethrough": {
            if (touches(state, node.from, node.to)) return false;
            const c = node.node.cursor();
            if (c.firstChild())
              do {
                if (/Mark$/.test(c.name)) out.push(hide.range(c.from, c.to));
              } while (c.nextSibling());
            return false;
          }
          case "Link": {
            if (touches(state, node.from, node.to)) return false;
            const c = node.node.cursor();
            let textFrom = -1;
            let textTo = -1;
            if (c.firstChild())
              do {
                if (c.name === "LinkMark") {
                  if (textFrom < 0) textFrom = c.to;
                  else if (textTo < 0) textTo = c.from;
                }
              } while (c.nextSibling());
            if (textFrom < 0 || textTo < 0 || textTo <= textFrom) return false;
            out.push(hide.range(node.from, textFrom));
            out.push(linkText.range(textFrom, textTo));
            out.push(hide.range(textTo, node.to));
            return false;
          }
          case "Blockquote": {
            for (let pos = node.from; pos <= node.to; ) {
              const line = state.doc.lineAt(pos);
              out.push(quoteLine.range(line.from));
              pos = line.to + 1;
            }
            return;
          }
          case "QuoteMark":
            if (!lineTouched(state, node.from)) hideWithSpace(state, out, node.from, node.to);
            return;
          case "FencedCode":
          case "CodeBlock": {
            const first = state.doc.lineAt(node.from).number;
            const last = state.doc.lineAt(node.to).number;
            for (let n = first; n <= last; n++) {
              const line = state.doc.line(n);
              codeLines.add(line.from);
              const fence = name === "FencedCode" && (n === first || n === last);
              out.push((fence ? fenceLine : codeLine).range(line.from));
            }
            return false;
          }
          case "HorizontalRule":
            if (!lineTouched(state, node.from)) out.push(rule.range(node.from, node.to));
            return;
          case "ListMark": {
            const line = state.doc.lineAt(node.from);
            if (line.from < node.from) out.push(indentMark.range(line.from, node.from));
            const task = checkboxToggle(line.text);
            if (task) {
              const done: TaskState = task.done ? "done" : task.moved ? "moved" : "open";
              if (!lineTouched(state, node.from)) {
                const markPos = line.from + task.at;
                const end = markPos + 2 + (state.sliceDoc(markPos + 2, markPos + 3) === " " ? 1 : 0);
                out.push(
                  Decoration.replace({ widget: new CheckboxWidget(done, markPos) }).range(node.from, end),
                );
              }
              return;
            }
            const ordered = node.node.parent?.parent?.name === "OrderedList";
            if (!ordered && !lineTouched(state, node.from)) out.push(bullet.range(node.from, node.to));
            return;
          }
        }
      },
    });
  }
  return { decos: out, codeLines };
}

/** Hide `[[` and `]]` (and `target|` when a label is given) of wiki links off the caret. */
export function liveWikiLinks(state: EditorState, lineFrom: number, text: string, out: Range<Decoration>[]) {
  for (const m of text.matchAll(WIKI_RE)) {
    const a = lineFrom + m.index!;
    const b = a + m[0].length;
    if (!isPageLink(state, a + 2)) continue;
    if (touches(state, a, b)) continue;
    const bar = m[1].indexOf("|");
    out.push(hide.range(a, bar >= 0 ? a + 2 + bar + 1 : a + 2));
    out.push(hide.range(b - 2, b));
  }
}

/** Ctrl+B / Ctrl+I: wrap the selection in `marker`, or unwrap it if already wrapped. */
export function toggleWrap(marker: string) {
  return (view: EditorView): boolean => {
    const { state } = view;
    const n = marker.length;
    // Count the run of marker characters on each side: `***x***` is bold
    // (2) plus italic (1), so italic is present when the run is odd.
    const ch = marker[0];
    const run = (pos: number, dir: -1 | 1) => {
      let k = 0;
      while (state.sliceDoc(dir < 0 ? pos - k - 1 : pos + k, dir < 0 ? pos - k : pos + k + 1) === ch) k++;
      return k;
    };
    const wrapped = (b: number, a: number) => (n === 1 ? b % 2 === 1 && a % 2 === 1 : b >= n && a >= n);
    view.dispatch(
      state.changeByRange((r) => {
        if (wrapped(run(r.from, -1), run(r.to, 1))) {
          return {
            changes: [
              { from: r.from - n, to: r.from },
              { from: r.to, to: r.to + n },
            ],
            range: EditorSelection.range(r.anchor - n, r.head - n),
          };
        }
        return {
          changes: [
            { from: r.from, insert: marker },
            { from: r.to, insert: marker },
          ],
          range: EditorSelection.range(r.anchor + n, r.head + n),
        };
      }),
    );
    return true;
  };
}
