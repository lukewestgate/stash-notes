import { describe, expect, it } from "vitest";
import { highlight } from "./highlight";

const marked = (text: string, q: string) =>
  highlight(text, q)
    .map((p) => (p.hit ? `[${p.text}]` : p.text))
    .join("");

describe("highlight", () => {
  it("marks substrings, case-insensitively", () => {
    expect(marked("Rollout plan", "PLAN")).toBe("Rollout [plan]");
    expect(marked("Rollout plan", "roll plan")).toBe("[Roll]out [plan]");
  });
  it("falls back to in-order letters", () => {
    expect(marked("Rollout date question", "rlt")).toBe("[R]o[l]lou[t] date question");
    expect(marked("PROJ-123", "proj123")).toBe("[PROJ]-[123]");
  });
  it("ignores status filters and words that don't match", () => {
    expect(marked("Disk alerts", "@open disk zzz")).toBe("[Disk] alerts");
    expect(marked("abc", "")).toBe("abc");
  });
});
