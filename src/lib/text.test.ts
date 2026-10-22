import { describe, expect, it } from "vitest";
import {
  checkboxToggle,
  firstTicket,
  gerritUrl,
  jiraUrl,
  looksLikeGerrit,
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
    expect(checkboxToggle("- [ ] a")).toEqual({ at: 3, insert: "x" });
    expect(checkboxToggle("  * [x] a")).toEqual({ at: 5, insert: " " });
    expect(checkboxToggle("1. [X] a")).toEqual({ at: 4, insert: " " });
    expect(checkboxToggle("- a")).toBeNull();
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
  });
});

describe("urls", () => {
  it("builds Jira and Gerrit links", () => {
    expect(jiraUrl("https://j/browse/{key}", "A-1")).toBe("https://j/browse/A-1");
    expect(gerritUrl("https://g/c/{n}", "123")).toBe("https://g/c/123");
    expect(gerritUrl("https://g/c/{n}", "https://g/c/p/+/9")).toBe("https://g/c/p/+/9");
  });
  it("recognizes Gerrit clipboard text", () => {
    const tpl = "https://gerrit.example.com/c/{n}";
    expect(looksLikeGerrit("12345", tpl)).toBe(true);
    expect(looksLikeGerrit("https://gerrit.example.com/c/proj/+/1", tpl)).toBe(true);
    expect(looksLikeGerrit("https://review.corp/c/proj/+/1", tpl)).toBe(true);
    expect(looksLikeGerrit("https://news.example.com/", tpl)).toBe(false);
    expect(looksLikeGerrit("hello", tpl)).toBe(false);
  });
});

it("formats timestamps", () => {
  expect(timestamp(new Date(2026, 9, 2, 9, 5))).toBe("2026-10-02 09:05 ");
});

it("finds the first ticket", () => {
  expect(firstTicket("x OPS-9 then PROJ-1", T)).toBe("OPS-9");
  expect(firstTicket("none here", T)).toBeNull();
});
