// Animation preference (Settings > Appearance > Animations): `system`
// follows the OS "reduce motion" setting. CSS reads `--motion` /
// `--motion-fast` (0s when off); Svelte transitions call `motion(ms)`.

import { cubicOut } from "svelte/easing";
import { fade, scale } from "svelte/transition";
import type { Settings } from "./api";

const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)");
let mode: Settings["animations"] = "system";

function enabled() {
  return mode === "on" || (mode === "system" && !reduce?.matches);
}

/** A transition duration, or 0 when animations are off. */
export function motion(ms: number): number {
  return enabled() ? ms : 0;
}

function apply() {
  const root = document.documentElement;
  root.dataset.motion = enabled() ? "on" : "off";
}

export function applyMotion(m: Settings["animations"] | undefined) {
  mode = m ?? "system";
  apply();
}

reduce?.addEventListener?.("change", apply);

/** Intro for a dialog's backdrop. */
export function dim(node: Element) {
  return fade(node, { duration: motion(120) });
}

/** Intro for a dialog or pop-up panel: a quick fade and grow. */
export function pop(node: Element) {
  return scale(node, { start: 0.97, opacity: 0, duration: motion(140), easing: cubicOut });
}
