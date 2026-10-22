import { syntaxTree } from "@codemirror/language";
import { type EditorState, type Extension, type Range, StateEffect, StateField } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView, WidgetType } from "@codemirror/view";
import { api } from "./api";
import { liveConfig } from "./live";
import "./images.css";

export const MAX_IMAGE_BYTES = 20 * 1024 * 1024;
export const IMAGE_ACCEPT = ".png,.jpg,.jpeg,.gif,.webp,.bmp,image/png,image/jpeg,image/gif,image/webp,image/bmp";

export interface ImageRef { from: number; to: number; source: string; alt: string }

/** Markdown's parser keeps code examples and front matter literal. */
export function imageRefs(state: EditorState): ImageRef[] {
  const refs: ImageRef[] = [];
  syntaxTree(state).iterate({ enter(node) {
    if (node.name !== "Image") return;
    const url = node.node.getChild("URL");
    if (!url) return false;
    const marks = node.node.getChildren("LinkMark");
    const alt = marks.length >= 2 ? state.sliceDoc(marks[0].to, marks[1].from) : "";
    refs.push({
      from: node.from, to: node.to,
      source: state.sliceDoc(url.from, url.to).replace(/^<|>$/g, "").replace(/\\([!"#$%&'()*+,\-./:;<=>?@[\]^_`{|}~\\])/g, "$1"),
      alt: alt.replace(/\\([\[\]\\])/g, "$1"),
    });
    return false;
  } });
  return refs;
}

/** Only web URLs and raster data URLs bypass vault-confined reads. */
export function directImageSource(source: string): string | null {
  if (/^https?:\/\//i.test(source)) return source;
  if (/^data:image\/(?:png|jpeg|gif|webp|bmp);base64,[a-z0-9+/=\s]+$/i.test(source)) return source;
  return null;
}

// Equivalent widgets can reuse DOM, so resource ownership follows the DOM.
const imageResources = new WeakMap<HTMLElement, { cancelled: boolean; url?: string }>();

class ImageWidget extends WidgetType {
  constructor(readonly ref: ImageRef, readonly path: string | null) { super(); }
  eq(other: ImageWidget) {
    return this.path === other.path && this.ref.source === other.ref.source && this.ref.alt === other.ref.alt;
  }
  toDOM(view: EditorView) {
    const dom = document.createElement("span");
    dom.className = "cm-image";
    dom.contentEditable = "false";
    const status = document.createElement("span");
    status.className = "cm-image-status";
    status.textContent = "Loading image…";
    status.setAttribute("role", "status");
    const img = document.createElement("img");
    img.alt = this.ref.alt;
    img.referrerPolicy = "no-referrer";
    img.draggable = false;
    img.hidden = true;
    const resource: { cancelled: boolean; url?: string } = { cancelled: false };
    imageResources.set(dom, resource);
    const fail = (error: unknown) => {
      if (resource.cancelled) return;
      img.hidden = true;
      status.hidden = false;
      status.textContent = `Image unavailable: ${this.ref.alt || this.ref.source}`;
      status.title = String(error);
      view.requestMeasure();
    };
    img.onload = () => {
      if (resource.cancelled) return;
      img.hidden = false;
      status.hidden = true;
      view.requestMeasure();
    };
    img.onerror = () => fail("The image could not be loaded.");
    const load = async () => {
      const direct = directImageSource(this.ref.source);
      if (direct) { img.src = direct; return; }
      if (!this.path) throw new Error("Open a note to load its images.");
      const bytes = await api.readImage(this.path, decodeURIComponent(this.ref.source));
      if (resource.cancelled) return;
      resource.url = URL.createObjectURL(new Blob([bytes]));
      img.src = resource.url;
    };
    void load().catch(fail);
    dom.append(img, status);
    if (!view.state.readOnly) {
      dom.title = "Click to edit image Markdown";
      dom.addEventListener("mousedown", (event) => {
        event.preventDefault();
        event.stopPropagation();
        const from = view.posAtDOM(dom);
        view.dispatch({ selection: { anchor: from + 2 }, scrollIntoView: true });
        view.focus();
      });
    }
    return dom;
  }
  destroy(dom: HTMLElement) {
    const resource = imageResources.get(dom);
    if (resource) {
      resource.cancelled = true;
      if (resource.url) URL.revokeObjectURL(resource.url);
      imageResources.delete(dom);
    }
  }
  ignoreEvent() { return true; }
}

export function imageEmbeds(path: () => string | null): Extension {
  const build = (state: EditorState) => {
    if (!liveConfig.enabled) return Decoration.none;
    const out: Range<Decoration>[] = [];
    for (const ref of imageRefs(state)) {
      if (!state.readOnly && state.selection.ranges.some((r) => r.from <= ref.to && r.to >= ref.from)) continue;
      out.push(Decoration.replace({ widget: new ImageWidget(ref, path()) }).range(ref.from, ref.to));
    }
    return Decoration.set(out, true);
  };
  return StateField.define<DecorationSet>({
    create: build,
    update: (value, tr) => tr.docChanged || tr.selection || tr.effects.length ? build(tr.state) : value,
    provide: (field) => EditorView.decorations.from(field),
  });
}

interface Upload { id: symbol; from: number; to: number }
const uploadEffect = StateEffect.define<{ id: symbol; range?: { from: number; to: number } }>();
const uploads = StateField.define<Upload[]>({
  create: () => [],
  update(value, tr) {
    let next = value.map((r) => ({ ...r, from: tr.changes.mapPos(r.from, 1), to: tr.changes.mapPos(r.to, 1) }));
    for (const effect of tr.effects) if (effect.is(uploadEffect)) {
      next = next.filter((r) => r.id !== effect.value.id);
      if (effect.value.range) next.push({ id: effect.value.id, ...effect.value.range });
    }
    return next;
  },
});

interface ImageHooks { path(): string | null; imageError(message: string): void }

function imageMarkdown(name: string, source: string) {
  const alt = name.replace(/\.[^.]+$/, "").replace(/[\r\n]/g, " ").replace(/[\\[\]]/g, "\\$&") || "image";
  return `![${alt}](${source})`;
}

/** Map the insertion point through edits; discard it when the note is closed. */
export async function insertImages(view: EditorView, files: File[], hooks: ImageHooks, at?: number) {
  const path = hooks.path();
  if (!path || view.state.readOnly || !files.length) return;
  const id = Symbol();
  const range = at === undefined ? view.state.selection.main : { from: at, to: at };
  view.dispatch({ effects: uploadEffect.of({ id, range }) });
  const links: string[] = [];
  try {
    for (const file of files) {
      if (!view.state.field(uploads, false)?.some((r) => r.id === id)) return;
      try {
        if (!file.size || file.size > MAX_IMAGE_BYTES) throw new Error("Images must be between 1 byte and 20 MB.");
        const bytes = new Uint8Array(await file.arrayBuffer());
        links.push(imageMarkdown(file.name, await api.importImage(path, Array.from(bytes))));
      } catch (error) { hooks.imageError(`${file.name}: ${String(error)}`); }
    }
    const anchor = view.state.field(uploads, false)?.find((r) => r.id === id);
    if (!anchor || hooks.path() !== path || !links.length) return;
    const before = view.state.sliceDoc(0, anchor.from);
    const prefix = before && !before.endsWith("\n") ? "\n" : "";
    const insert = prefix + links.join("\n\n") + "\n\n";
    view.dispatch({ changes: { from: anchor.from, to: Math.max(anchor.from, anchor.to), insert }, selection: { anchor: anchor.from + insert.length }, userEvent: "input.paste" });
    view.focus();
  } finally {
    if (view.state.field(uploads, false)?.some((r) => r.id === id)) view.dispatch({ effects: uploadEffect.of({ id }) });
  }
}

export function pickImages(view: EditorView, hooks: ImageHooks) {
  if (!hooks.path()) return;
  const path = hooks.path();
  const input = document.createElement("input");
  input.type = "file";
  input.accept = IMAGE_ACCEPT;
  input.multiple = true;
  input.onchange = () => {
    if (hooks.path() === path) void insertImages(view, Array.from(input.files ?? []), hooks);
  };
  input.click();
}

export function imageInput(hooks: ImageHooks): Extension {
  return [uploads, EditorView.domEventHandlers({
    paste(event, view) {
      const files = Array.from(event.clipboardData?.files ?? []);
      if (!files.length) return false;
      event.preventDefault();
      void insertImages(view, files, hooks);
      return true;
    },
    dragover(event) {
      if (!event.dataTransfer?.types.includes("Files")) return false;
      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
      return true;
    },
    drop(event, view) {
      const files = Array.from(event.dataTransfer?.files ?? []);
      if (!files.length) return false;
      event.preventDefault();
      const at = view.posAtCoords({ x: event.clientX, y: event.clientY }) ?? view.state.selection.main.head;
      void insertImages(view, files, hooks, at);
      return true;
    },
  })];
}
