import { describe, expect, it } from "vitest";
import { placement } from "./popup";

const vw = 1000;
const vh = 800;

describe("placement", () => {
  it("opens below a button, aligned to its left edge", () => {
    expect(placement({ x: 100, y: 50, w: 30, h: 20 }, 200, 150, vw, vh)).toEqual({ left: 100, top: 74 });
  });

  it("opens at the pointer for a right-click", () => {
    expect(placement({ x: 300, y: 200, w: 0, h: 0 }, 200, 150, vw, vh)).toEqual({ left: 300, top: 204 });
  });

  it("flips above when there is no room below", () => {
    expect(placement({ x: 100, y: 700, w: 30, h: 20 }, 200, 150, vw, vh)).toEqual({ left: 100, top: 546 });
  });

  it("stays inside the window on the right", () => {
    expect(placement({ x: 950, y: 50, w: 0, h: 0 }, 200, 150, vw, vh).left).toBe(792);
  });

  it("puts a submenu beside its item, or on the left when the right is full", () => {
    expect(placement({ x: 100, y: 100, w: 240, h: 30, side: "right" }, 180, 150, vw, vh)).toEqual({ left: 342, top: 95 });
    expect(placement({ x: 700, y: 100, w: 240, h: 30, side: "right" }, 180, 150, vw, vh)).toEqual({ left: 518, top: 95 });
  });

  it("keeps a tall popup on screen", () => {
    expect(placement({ x: 100, y: 400, w: 0, h: 0 }, 200, 790, vw, vh).top).toBe(8);
  });
});
