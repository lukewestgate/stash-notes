// Disclosures and block previews for [[note links]]. Expansion is view state;
// it never changes the markdown or joins/closes an editable note buffer.
import { syntaxTree } from "@codemirror/language";
import { type EditorState, type Extension, type Range, StateEffect, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, ViewPlugin, WidgetType } from "@codemirror/view";
import type { MarkdownConfig } from "@lezer/markdown";
import { api, on } from "./api";
import { iconElement } from "./icons";
import { WIKI_RE, wikiTarget } from "./text";

export interface PageLink {
  from: number;
  to: number;
  target: string;
  label: string;
  source: string;
}

/** Keep wiki links separate from markdown's shortcut-link parser. */
export const pageLinkSyntax: MarkdownConfig = {
  defineNodes: ["WikiLink"],
  parseInline: [{
    name: "WikiLink",
    before: "Link",
    parse(cx, next, pos) {
      if (next !== 91 || cx.char(pos + 1) !== 91) return -1;
      const match = /^\[\[([^\[\]\n]+?)\]\]/.exec(cx.slice(pos, cx.end));
      return match ? cx.addElement(cx.elt("WikiLink", pos, pos + match[0].length)) : -1;
    },
  }],
};

/** Literal examples and links inside other markdown links aren't page links. */
export function isPageLink(state: EditorState, pos: number): boolean {
  let found = false;
  for (let node = syntaxTree(state).resolveInner(pos, 1); node; node = node.parent!) {
    if (/^(FencedCode|CodeBlock|InlineCode|FrontMatter|Link|Image)$/.test(node.name)) return false;
    if (node.name === "WikiLink") found = true;
  }
  return found;
}

export function noteLinks(state: EditorState): PageLink[] {
  const links: PageLink[] = [];
  for (const m of state.doc.toString().matchAll(WIKI_RE)) {
    const from = m.index!;
    const target = wikiTarget(m[1]);
    if (!target || !isPageLink(state, from + 2)) continue;
    const bar = m[1].indexOf("|");
    links.push({ from, to: from + m[0].length, target, label: (bar < 0 ? m[1] : m[1].slice(bar + 1)).trim(), source: m[0] });
  }
  return links;
}

export const togglePage = StateEffect.define<number>();

/** Map each occurrence independently and close previews whose link was edited. */
export const expandedPages = StateField.define<PageLink[]>({
  create: () => [],
  update(open, tr) {
    if (tr.docChanged) {
      open = open.map((link) => ({
        ...link,
        from: tr.changes.mapPos(link.from, 1),
        to: tr.changes.mapPos(link.to, -1),
      })).filter((link) => link.to > link.from && tr.state.sliceDoc(link.from, link.to) === link.source && isPageLink(tr.state, link.from + 2));
    }
    for (const effect of tr.effects) {
      if (!effect.is(togglePage)) continue;
      if (open.some((link) => link.from === effect.value)) open = open.filter((link) => link.from !== effect.value);
      else {
        const link = noteLinks(tr.state).find((link) => link.from === effect.value);
        if (link) open = [...open, link];
      }
    }
    return open;
  },
});

interface PageHooks {
  path(): string | null;
  open(path: string): void;
  render(parent: HTMLElement, text: string, path: string, ancestors: string[]): EditorView;
}

class Disclosure extends WidgetType {
  constructor(readonly link: PageLink, readonly expanded: boolean) { super(); }
  eq(other: Disclosure) {
    return this.link.from === other.link.from && this.link.source === other.link.source && this.expanded === other.expanded;
  }
  toDOM(view: EditorView) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "cm-page-toggle";
    button.contentEditable = "false";
    button.setAttribute("aria-expanded", String(this.expanded));
    button.setAttribute("aria-label", `${this.expanded ? "Collapse" : "Expand"} ${this.link.label}`);
    button.title = `${this.expanded ? "Collapse" : "Expand"} linked note`;
    button.append(iconElement(this.expanded ? "chevron-down" : "chevron-right", 13));
    button.addEventListener("mousedown", (event) => event.preventDefault());
    button.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      view.dispatch({ effects: togglePage.of(this.link.from) });
    });
    return button;
  }
  ignoreEvent() { return true; }
}

// CodeMirror can reuse a widget's DOM with an equivalent new WidgetType.
// Tie cleanup to the DOM node so those replacements keep their lifecycle.
const previewCleanups = new WeakMap<HTMLElement, () => void>();

class PagePreview extends WidgetType {
  constructor(readonly link: PageLink, readonly hooks: PageHooks, readonly ancestors: string[]) { super(); }
  eq(other: PagePreview) {
    return this.link.source === other.link.source && this.hooks === other.hooks;
  }
  toDOM(view: EditorView) {
    const root = document.createElement("section");
    root.className = "cm-page-preview";
    // Alternate surfaces at each depth so nested pages remain distinct.
    root.style.setProperty("--embed-surface", this.ancestors.length % 2 ? "var(--bg-raised)" : "var(--bg)");
    root.contentEditable = "false";
    root.setAttribute("aria-label", `Preview of ${this.link.label}`);
    const header = document.createElement("div");
    header.className = "cm-page-preview-header";
    const open = document.createElement("button");
    open.type = "button";
    open.textContent = "Open note";
    open.disabled = true;
    header.append(open);
    const body = document.createElement("div");
    body.className = "cm-page-preview-body";
    const status = document.createElement("div");
    status.className = "cm-page-preview-status";
    status.setAttribute("role", "status");
    status.textContent = "Loading note…";
    body.append(status);
    root.append(header, body);
    let child: EditorView | undefined;
    let path: string | undefined;
    let text: string | undefined;
    let disposed = false;
    let generation = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const stops: (() => void)[] = [];
    open.addEventListener("click", () => { if (path) this.hooks.open(path); });
    // Keep the outer editor's mouse handlers away from the nested editor.
    root.addEventListener("mousedown", (event) => event.stopPropagation());
    root.addEventListener("keydown", (event) => event.stopPropagation());
    const load = async () => {
      const request = ++generation;
      try {
        const note = await api.notePreview(this.link.target, this.hooks.path());
        if (disposed || request !== generation) return;
        open.disabled = false;
        if (note.path === path && note.text === text) return;
        path = note.path;
        text = note.text;
        child?.destroy();
        child = undefined;
        body.replaceChildren();
        if (this.ancestors.includes(path) || this.ancestors.length >= 5) {
          status.textContent = this.ancestors.includes(path) ? "This note is already shown above." : "Open this note to see further embedded pages.";
          body.append(status);
        } else if (!text.trim()) {
          status.textContent = "This note is empty.";
          body.append(status);
        } else {
          child = this.hooks.render(body, text, path, [...this.ancestors, path]);
        }
        view.requestMeasure();
      } catch (error) {
        if (disposed || request !== generation) return;
        child?.destroy();
        child = undefined;
        path = text = undefined;
        open.disabled = true;
        status.textContent = `Could not preview this note: ${String(error)}`;
        const retry = document.createElement("button");
        retry.type = "button";
        retry.textContent = "Retry";
        retry.addEventListener("click", load);
        body.replaceChildren(status, retry);
        view.requestMeasure();
      }
    };
    const refresh = () => {
      clearTimeout(timer);
      timer = setTimeout(load, 100);
    };
    // File changes are broadcast to all windows, including previews which
    // deliberately don't register as editable buffer clients.
    for (const event of ["vault-changed", "index-ready"]) {
      on(event, refresh).then((stop) => { if (disposed) stop(); else stops.push(stop); });
    }
    previewCleanups.set(root, () => {
      disposed = true;
      clearTimeout(timer);
      stops.forEach((stop) => stop());
      child?.destroy();
    });
    load();
    return root;
  }
  destroy(dom: HTMLElement) {
    previewCleanups.get(dom)?.();
    previewCleanups.delete(dom);
  }
  ignoreEvent() { return true; }
}

/** Block decorations must come from a state field, so they affect layout. */
export function pageEmbeds(hooks: PageHooks, ancestors?: string[]): Extension[] {
  const build = (state: EditorState): DecorationSet => {
    const out: Range<Decoration>[] = [];
    const open = state.field(expandedPages);
    for (const link of noteLinks(state)) {
      const expanded = open.some((item) => item.from === link.from);
      out.push(Decoration.mark({ class: "cm-page-reference", inclusive: true }).range(link.from, link.to));
      out.push(Decoration.widget({ widget: new Disclosure(link, expanded), side: -1 }).range(link.to));
      if (expanded) {
        out.push(Decoration.widget({
          widget: new PagePreview(link, hooks, ancestors ?? (hooks.path() ? [hooks.path()!] : [])),
          block: true,
          side: 1,
        }).range(state.doc.lineAt(link.to).to));
      }
    }
    return Decoration.set(out, true);
  };
  const field = StateField.define<DecorationSet>({
    create: build,
    update: (_old, tr) => build(tr.state),
    provide: (field) => EditorView.decorations.from(field),
  });
  return [expandedPages, field,
    // A nested preview's height can change after an async load or expansion.
    ViewPlugin.define((view) => {
      const observer = new ResizeObserver(() => view.requestMeasure());
      observer.observe(view.contentDOM);
      return { destroy: () => observer.disconnect() };
    }),
  ];
}
