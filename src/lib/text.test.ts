import { describe, expect, it } from "vitest";
import {
  checkboxToggle,
  formatDate,
  firstTicket,
  normalizeTag,
  typingTag,
  tagSuggestions,
  completeTag,
  refAt,
  ticketRegex,
  timestamp,
  todoCursor,
} from "./text";

const T = ticketRegex("[A-Z][A-Z0-9]+-\\d+");

describe("refAt", () => {
  const line = "see [[Rollout plan|plan]] and PROJ-123 at https://x.com/a.";
  it("finds wiki links on or next to the cursor", () => {
    expect(refAt(line, 4, T)).toMatchObject({ kind: "wiki", target: "Rollout plan" });
    expect(refAt(line, 25, T)).toMatchObject({ kind: "wiki", target: "Rollout plan" });
  });
  it("finds ticket keys and URLs", () => {
    expect(refAt(line, 32, T)).toMatchObject({ kind: "ticket", target: "PROJ-123" });
    expect(refAt(line, 50, T)).toMatchObject({ kind: "url", target: "https://x.com/a" });
  });
  it("prefers the link over a key inside it", () => {
    expect(refAt("[[PROJ-1]]", 4, T)).toMatchObject({ kind: "wiki", target: "PROJ-1" });
  });
  it("follows markdown links by their URL", () => {
    expect(refAt("see [the docs](https://d.io/x) ok", 7, T)).toMatchObject({ kind: "url", target: "https://d.io/x" });
  });
  it("returns null elsewhere", () => {
    expect(refAt(line, 1, T)).toBeNull();
  });
});

describe("checkboxes", () => {
  it("toggles both ways", () => {
    expect(checkboxToggle("- [ ] a")).toEqual({ at: 3, insert: "x", done: false, moved: false });
    expect(checkboxToggle("  * [x] a")).toEqual({ at: 5, insert: " ", done: true, moved: false });
    expect(checkboxToggle("1. [X] a")).toEqual({ at: 4, insert: " ", done: true, moved: false });
    expect(checkboxToggle("- a")).toBeNull();
  });
  it("reopens a moved todo", () => {
    expect(checkboxToggle("- [>] a")).toEqual({ at: 3, insert: " ", done: false, moved: true });
  });
});

describe("todoCursor", () => {
  it("lands at the end of the Todo section", () => {
    const doc = "# d\n\n## Todo\n\n- [ ] a\n- [ ] \n\n## Notes\n\nx\n";
    expect(doc.slice(0, todoCursor(doc)!)).toBe("# d\n\n## Todo\n\n- [ ] a\n- [ ] ");
  });
  it("handles a missing section", () => {
    expect(todoCursor("# d\n")).toBeNull();
    expect(todoCursor("## Todo")).toBe(7);
    expect(todoCursor("## To do\n- [ ] a")).toBe(16);
  });
});

it("formats daily dates", () => {
  expect(formatDate("2026-10-02", "yyyy-MM-dd")).toBe("2026-10-02");
  expect(formatDate("2026-10-02", "dd/MM/yyyy")).toBe("02/10/2026");
  expect(formatDate("2026-10-02", "EEE d MMM yyyy")).toBe("Fri 2 Oct 2026");
  expect(formatDate("2026-10-02", "EEEE d MMMM yy")).toBe("Friday 2 October 26");
  expect(formatDate("rollout", "dd/MM/yyyy")).toBe("rollout");
});

it("formats timestamps", () => {
  expect(timestamp(new Date(2026, 9, 2, 9, 5))).toBe("2026-10-02 09:05 ");
  expect(timestamp(new Date(2026, 9, 2, 14, 5), "dd/MM/yy hh:mm a")).toBe("02/10/26 02:05 pm ");
});

it("finds the first ticket", () => {
  expect(firstTicket("x OPS-9 then PROJ-1", T)).toBe("OPS-9");
  expect(firstTicket("none here", T)).toBeNull();
});

describe("normalizeTag", () => {
  it("canonicalizes and rejects", () => {
    expect(normalizeTag("#Bug")).toBe("bug");
    expect(normalizeTag("bug/UI/")).toBe("bug/ui");
    expect(normalizeTag("123")).toBeNull();
    expect(normalizeTag("a b")).toBeNull();
    expect(normalizeTag("")).toBeNull();
  });
});

describe("tag completion", () => {
  const all = [{ name: "bug" }, { name: "bug/ui" }, { name: "research" }];
  it("suggests and completes the tag being typed", () => {
    expect(typingTag("rollout #bu")).toBe("bu");
    expect(typingTag("rollout #bug ")).toBeNull();
    expect(typingTag("#")).toBe("");
    expect(tagSuggestions("x #b", all)).toEqual(["bug", "bug/ui"]);
    expect(tagSuggestions("x #bug", all)).toEqual(["bug/ui"]);
    expect(tagSuggestions("x", all)).toEqual([]);
    expect(completeTag("x #bu", "bug")).toBe("x #bug ");
  });
});
