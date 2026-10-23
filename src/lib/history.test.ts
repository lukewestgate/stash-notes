import { expect, it } from "vitest";
import { History, MAX_HISTORY } from "./history";

it("works like browser history", () => {
  const h = new History();
  h.push("a");
  h.push("b");
  h.push("c");
  expect(h.back()?.path).toBe("b");
  expect(h.back()?.path).toBe("a");
  expect(h.back()).toBeNull();
  expect(h.forward()?.path).toBe("b");
  h.push("d"); // discards "c"
  expect(h.canForward).toBe(false);
  expect(h.entries.map((e) => e.path)).toEqual(["a", "b", "d"]);
});

it("peeks without touching the stack", () => {
  const h = new History();
  h.push("a");
  h.push("b");
  h.back();
  h.peekAt("x");
  expect(h.current?.path).toBe("x");
  expect(h.back()?.path).toBe("a");
  expect(h.forward()?.path).toBe("b");
});

it("follows renames and caps length", () => {
  const h = new History();
  for (let i = 0; i < MAX_HISTORY + 20; i++) h.push(`n${i}`);
  expect(h.entries.length).toBe(MAX_HISTORY);
  h.rename(`n${MAX_HISTORY + 19}`, "moved");
  expect(h.current?.path).toBe("moved");
});

it("drops deleted files from past, current, forward and peek entries", () => {
  const h = new History();
  h.push("deleted");
  h.push("keep");
  h.push("deleted");
  h.push("forward");
  h.back();
  h.peekAt("deleted");
  h.remove("deleted");
  expect(h.current?.path).toBe("keep");
  expect(h.entries.map((e) => e.path)).toEqual(["keep", "forward"]);
  expect(h.forward()?.path).toBe("forward");
  h.remove("keep");
  expect(h.current?.path).toBe("forward");
  h.remove("forward");
  expect(h.current).toBeNull();
  expect(h.canBack).toBe(false);
  expect(h.canForward).toBe(false);
});
