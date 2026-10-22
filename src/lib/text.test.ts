import { describe, expect, it } from "vitest";
import {
  checkboxToggle,
  firstTicket,
  gerritUrl,
  isGerritUrl,
  normalizeTag,
  typingTag,
  tagSuggestions,
  completeTag,
  jiraKeyFromUrl,
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
  expect(timestamp(new Date(2026, 9, 2, 14, 5), "dd/MM/yy hh:mm a")).toBe("02/10/26 02:05 pm ");
});

it("finds the first ticket", () => {
  expect(firstTicket("x OPS-9 then PROJ-1", T)).toBe("OPS-9");
  expect(firstTicket("none here", T)).toBeNull();
});

describe("smart paste", () => {
  const jira = "https://yourco.atlassian.net/browse/{key}";
  it("turns Jira URLs into keys", () => {
    expect(jiraKeyFromUrl("https://yourco.atlassian.net/browse/PROJ-123?focusedId=1#x", T, jira)).toBe("PROJ-123");
    expect(jiraKeyFromUrl("https://other.net/jira/browse/OPS-9", T, jira)).toBe("OPS-9");
    expect(jiraKeyFromUrl("https://yourco.atlassian.net/jira/software/c/projects/P/boards/1?selectedIssue=PR-77", T, jira)).toBe("PR-77");
    expect(jiraKeyFromUrl("https://yourco.atlassian.net/issues/PROJ-5/details", T, jira)).toBe("PROJ-5");
    expect(jiraKeyFromUrl("https://example.com/PROJ-5", T, jira)).toBeNull();
    expect(jiraKeyFromUrl("see https://yourco.atlassian.net/browse/PROJ-1", T, jira)).toBeNull();
  });
  it("recognizes Gerrit URLs by host", () => {
    const g = "https://gerrit.example.com/c/{n}";
    expect(isGerritUrl("https://gerrit.example.com/c/proj/+/12399", g)).toBe(true);
    expect(isGerritUrl("https://example.com/c/proj/+/12399", g)).toBe(false);
  });
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
