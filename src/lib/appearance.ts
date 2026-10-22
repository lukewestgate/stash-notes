// Theme, fonts and accent color from settings, applied to the page.

import type { Settings } from "./api";

export function applyAppearance(s: Settings) {
  const root = document.documentElement;
  if (s.theme === "light" || s.theme === "dark") root.dataset.theme = s.theme;
  else delete root.dataset.theme;
  root.style.setProperty("--font-size", `${s.fontSize || 14}px`);
  root.style.setProperty("--line-height", String(s.lineHeight || 1.6));
  if (s.fontFamily) root.style.setProperty("--editor-font", `"${s.fontFamily}", "Segoe UI", system-ui, sans-serif`);
  else root.style.removeProperty("--editor-font");
  root.style.setProperty("--accent", `var(--c-${s.accent || "blue"})`);
}
