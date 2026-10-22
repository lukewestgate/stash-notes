import { describe, expect, it } from "vitest";
import { formatKey, matches, parseKey } from "./keys";

function ev(init: Partial<KeyboardEvent>): KeyboardEvent {
  return { ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, key: "", code: "", ...init } as KeyboardEvent;
}

describe("keys", () => {
  it("parses specs", () => {
    expect(parseKey("Ctrl+Shift+G")).toEqual({ ctrl: true, alt: false, shift: true, meta: false, key: "g" });
    expect(parseKey("Alt+Left")?.key).toBe("left");
    expect(parseKey("Ctrl++")?.key).toBe("+");
    expect(parseKey("")).toBeNull();
  });
  it("matches events by key or physical code", () => {
    expect(matches(ev({ ctrlKey: true, shiftKey: true, key: "G", code: "KeyG" }), parseKey("Ctrl+Shift+G"))).toBe(true);
    expect(matches(ev({ ctrlKey: true, key: "ö", code: "Semicolon" }), parseKey("Ctrl+;"))).toBe(true);
    expect(matches(ev({ altKey: true, key: "ArrowLeft" }), parseKey("Alt+Left"))).toBe(true);
    expect(matches(ev({ key: "Escape" }), parseKey("Esc"))).toBe(true);
    expect(matches(ev({ ctrlKey: true, key: "g", code: "KeyG" }), parseKey("Ctrl+Shift+G"))).toBe(false);
  });
  it("formats for display", () => {
    expect(formatKey("ctrl+shift+b")).toBe("Ctrl+Shift+B");
    expect(formatKey("alt+left")).toBe("Alt+Left");
  });
});

describe("chords", () => {
  it("parses and matches the second key with or without Ctrl", async () => {
    const { parseChord, matchesChordTail } = await import("./keys");
    const c = parseChord("Ctrl+E P");
    expect(c.length).toBe(2);
    expect(matchesChordTail(ev({ key: "p", code: "KeyP" }), c)).toBe(true);
    expect(matchesChordTail(ev({ ctrlKey: true, key: "p", code: "KeyP" }), c)).toBe(true);
    expect(matchesChordTail(ev({ key: "x", code: "KeyX" }), c)).toBe(false);
  });
});
