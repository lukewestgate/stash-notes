// Live markdown formatting (spec section 11): markers such as `#`, `**` and
// `[[ ]]` hide when the caret is elsewhere and reappear when it moves into
// them. The document itself always stays plain markdown.

import { syntaxTree } from "@codemirror/language";
import { EditorSelection, type EditorState, type Range, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import type { MarkdownConfig } from "@lezer/markdown";
import { checkboxToggle, WIKI_RE } from "./text";
import { iconElement } from "./icons";

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

/** Does the selection touch [from, to]? */
function touches(state: EditorState, from: number, to: number): boolean {
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

class CheckboxWidget extends WidgetType {
  constructor(
    readonly checked: boolean,
    readonly markPos: number,
  ) {
    super();
  }
  eq(o: CheckboxWidget) {
    return o.checked === this.checked && o.markPos === this.markPos;
  }
  toDOM(view: EditorView) {
    const box = document.createElement("span");
    box.className = "cm-task" + (this.checked ? " cm-task-done" : "");
    box.setAttribute("role", "checkbox");
    box.setAttribute("aria-checked", String(this.checked));
    box.addEventListener("mousedown", (e) => {
      e.preventDefault();
      view.dispatch({ changes: { from: this.markPos, to: this.markPos + 1, insert: this.checked ? " " : "x" } });
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
  gerrit: number;
  aliases: string[];
}

/** The pieces of front matter the properties row shows. */
export function readFrontMatter(text: string): FrontMatterInfo {
  const status = /^status:[ \t]*([^\s#]+)/m.exec(text)?.[1] ?? null;
  const gerritBlock = /^gerrit:[ \t]*(\[.*\]|[^\n]*)\n((?:[ \t]+-[^\n]*\n?)*)/m.exec(text);
  let gerrit = 0;
  if (gerritBlock) {
    const inline = gerritBlock[1].trim();
    if (inline.startsWith("[")) gerrit = inline.slice(1, -1).split(",").filter((x) => x.trim()).length;
    else if (inline) gerrit = 1;
    gerrit += (gerritBlock[2].match(/-\s*\S/g) ?? []).length;
  }
  const aliasLine = /^aliases:[ \t]*\[(.*)\]/m.exec(text)?.[1];
  const aliases = aliasLine ? aliasLine.split(",").map((x) => x.trim().replace(/^["']|["']$/g, "")).filter(Boolean) : [];
  return { status: status?.toLowerCase() ?? null, gerrit, aliases };
}

/** What the front matter row needs from the app. */
export interface PropertiesHooks {
  statusView(status: string): { icon: string; color: string; name: string };
  onStatus(): void;
  onJira(): void;
  onGerrit(): void;
  hasJira(): boolean;
}

export const propertiesHooks: { current: PropertiesHooks | null } = { current: null };

class PropertiesWidget extends WidgetType {
  constructor(readonly info: FrontMatterInfo) {
    super();
  }
  eq(o: PropertiesWidget) {
    return JSON.stringify(o.info) === JSON.stringify(this.info);
  }
  toDOM() {
    const hooks = propertiesHooks.current;
    const row = document.createElement("div");
    row.className = "cm-properties";
    const button = (cls: string, text: string, title: string, run?: () => void, icon?: string) => {
      const b = document.createElement("button");
      b.className = cls;
      if (icon) b.append(iconElement(icon, 13));
      b.append(text);
      b.title = title;
      b.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        run?.();
      });
      row.appendChild(b);
      return b;
    };
    const st = this.info.status ? hooks?.statusView(this.info.status) : null;
    const chip = button(
      "cm-prop cm-prop-status",
      st ? st.name : "no status",
      "Set status",
      () => hooks?.onStatus(),
      st?.icon ?? "circle-dashed",
    );
    if (st) chip.style.color = `var(--c-${st.color})`;
    if (hooks?.hasJira()) button("cm-prop", "Jira", "Open in Jira", () => hooks?.onJira(), "ticket");
    if (this.info.gerrit > 0)
      button(
        "cm-prop",
        this.info.gerrit > 1 ? `Gerrit ${this.info.gerrit}` : "Gerrit",
        "Open Gerrit change",
        () => hooks?.onGerrit(),
        "git-pull-request",
      );
    if (this.info.aliases.length) {
      const a = document.createElement("span");
      a.className = "cm-prop-text";
      a.textContent = `aka ${this.info.aliases.join(", ")}`;
      row.appendChild(a);
    }
    const hint = document.createElement("span");
    hint.className = "cm-prop-hint";
    hint.textContent = "properties";
    row.appendChild(hint);
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

function buildFrontMatter(state: EditorState): DecorationSet {
  const fm = frontMatterRange(state);
  if (!fm) return Decoration.none;
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
              const done = task.insert === " ";
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
