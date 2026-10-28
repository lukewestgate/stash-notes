// How daily notes are named on screen (Settings → Notes → Daily note date).
// Their files stay `yyyy-MM-dd.md`, so they sort by date; only what's shown changes.

import { formatDate } from "./text";

export const daily = $state({ format: "yyyy-MM-dd" });

/** A note's name to show: a daily note's date in the chosen format, else its title. */
export function noteName(n: { kind: string; title: string }): string {
  return n.kind === "daily" ? formatDate(n.title, daily.format) : n.title;
}

/** Whether the chosen format already names the weekday. */
export function showsWeekday(): boolean {
  return daily.format.includes("E");
}
