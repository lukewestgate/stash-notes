import { markdown } from "@codemirror/lang-markdown";
import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { frontMatterSyntax } from "./live";
import { expandedPages, noteLinks, pageLinkSyntax, togglePage } from "./page-links";

const state = (doc: string) => EditorState.create({
  doc,
  extensions: [markdown({ extensions: [frontMatterSyntax, pageLinkSyntax] }), expandedPages],
});

describe("embedded page links", () => {
  it("recognizes paths, aliases, vaults and heading links", () => {
    expect(noteLinks(state("- [[work:notes/plan.md#Todo|The plan]] [[Other]]"))).toMatchObject([
      { target: "work:notes/plan.md", label: "The plan" }, { target: "Other", label: "Other" },
    ]);
  });

  it("leaves code, front matter, markdown URLs and empty targets alone", () => {
    const doc = '---\naliases: ["[[yaml]]"]\n---\n\n`[[inline]]`\n\n```md\n[[fenced]]\n```\n\n    [[indented]]\n\n[label](https://example.org/[[url]]) [[ |label]] [[Real]]';
    expect(noteLinks(state(doc)).map((link) => link.target)).toEqual(["Real"]);
  });

  it("expands duplicate occurrences independently and leaves the text intact", () => {
    let s = state("[[Plan]] and [[Plan]]");
    const original = s.doc.toString();
    s = s.update({ effects: togglePage.of(0) }).state;
    expect(s.field(expandedPages).map((link) => link.from)).toEqual([0]);
    s = s.update({ effects: togglePage.of(13) }).state;
    expect(s.field(expandedPages)).toHaveLength(2);
    s = s.update({ effects: togglePage.of(0) }).state;
    expect(s.field(expandedPages).map((link) => link.from)).toEqual([13]);
    expect(s.doc.toString()).toBe(original);
  });

  it("keeps the preview attached when text is inserted before or after it", () => {
    let s = state("[[Plan]]");
    s = s.update({ effects: togglePage.of(0) }).state;
    s = s.update({ changes: { from: 0, insert: "intro\n" } }).state;
    expect(s.field(expandedPages)[0].from).toBe(6);
    s = s.update({ changes: { from: s.doc.length, insert: " after" } }).state;
    expect(s.field(expandedPages)).toHaveLength(1);
  });

  it("closes previews when their target changes, is deleted, or becomes code", () => {
    for (const change of [
      { from: 2, to: 6, insert: "Other" },
      { from: 0, to: 8, insert: "" },
      { from: 0, to: 8, insert: "`[[Plan]]`" },
    ]) {
      let s = state("[[Plan]]");
      s = s.update({ effects: togglePage.of(0) }).state;
      expect(s.update({ changes: change }).state.field(expandedPages)).toEqual([]);
    }
  });
});
