// Pure helpers for note text: references under the cursor, checkboxes,
// timestamps, cursor placement and link URLs.

export type Ref =
  | { kind: "wiki"; target: string; from: number; to: number }
  | { kind: "ticket"; target: string; from: number; to: number }
  | { kind: "url"; target: string; from: number; to: number };

export const WIKI_RE = /\[\[([^\[\]\n]+?)\]\]/g;
export const URL_RE = /https?:\/\/[^\s<>()\[\]"'`]+/g;
/** `[text](https://...)` */
export const MDLINK_RE = /\[([^\]\n]+)\]\((https?:\/\/[^)\s]+)[^)\n]*\)/g;

/** Build the in-text ticket regex from the settings pattern. */
export function ticketRegex(pattern: string): RegExp {
  try {
    return new RegExp(`\\b(?:${pattern})\\b`, "g");
  } catch {
    return /\b(?:[A-Z][A-Z0-9]+-\d+)\b/g;
  }
}

/** The target of a `[[target|label]]` / `[[target#section]]` link body. */
export function wikiTarget(inner: string): string {
  return inner.split(/[|#]/)[0].trim();
}

function trimUrl(url: string): string {
  return url.replace(/[.,;:!?)]+$/, "");
}

/**
 * The reference at `pos` in `line` (a cursor on or directly next to it counts).
 * Wiki links win over ticket keys and URLs inside them.
 */
export function refAt(line: string, pos: number, tickets: RegExp): Ref | null {
  const hit = (from: number, to: number) => pos >= from && pos <= to;
  for (const m of line.matchAll(WIKI_RE)) {
    const from = m.index!;
    const to = from + m[0].length;
    if (hit(from, to)) {
      const target = wikiTarget(m[1]);
      if (target) return { kind: "wiki", target, from, to };
    }
  }
  for (const m of line.matchAll(MDLINK_RE)) {
    const from = m.index!;
    if (hit(from, from + m[0].length)) return { kind: "url", target: m[2], from, to: from + m[0].length };
  }
  for (const m of line.matchAll(URL_RE)) {
    const url = trimUrl(m[0]);
    const from = m.index!;
    if (hit(from, from + url.length)) return { kind: "url", target: url, from, to: from + url.length };
  }
  const re = new RegExp(tickets.source, "g");
  for (const m of line.matchAll(re)) {
    const from = m.index!;
    if (hit(from, from + m[0].length)) return { kind: "ticket", target: m[0], from, to: from + m[0].length };
  }
  return null;
}

/** First ticket key anywhere in `text`. */
export function firstTicket(text: string, tickets: RegExp): string | null {
  const re = new RegExp(tickets.source, "g");
  return re.exec(text)?.[0] ?? null;
}

const CHECKBOX_RE = /^(\s*(?:[-*+]|\d+[.)])\s+\[)([ xX])(\])/;

/** If `line` is a checkbox item, the offset of its mark and the replacement character. */
export function checkboxToggle(line: string): { at: number; insert: string } | null {
  const m = CHECKBOX_RE.exec(line);
  if (!m) return null;
  return { at: m[1].length, insert: m[2] === " " ? "x" : " " };
}

const pad = (n: number) => String(n).padStart(2, "0");

export function isoDate(d = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** `2026-10-22 14:05 ` */
export function timestamp(d = new Date()): string {
  return `${isoDate(d)} ${pad(d.getHours())}:${pad(d.getMinutes())} `;
}

/** Offset at the end of the `## Todo` section's last non-blank line (or of the heading). */
export function todoCursor(doc: string): number | null {
  const lines = doc.split("\n");
  const offsets: number[] = [];
  let acc = 0;
  for (const l of lines) {
    offsets.push(acc);
    acc += l.length + 1;
  }
  const start = lines.findIndex((l) => /^#{1,6}\s+todo\s*$/i.test(l.trimEnd()));
  if (start < 0) return null;
  let last = start;
  for (let i = start + 1; i < lines.length; i++) {
    if (/^#{1,2}\s/.test(lines[i])) break;
    if (lines[i].trim() !== "") last = i;
  }
  return offsets[last] + lines[last].replace(/\r$/, "").length;
}

export function jiraUrl(template: string, key: string): string {
  return template.replaceAll("{key}", key);
}

/** A front matter `gerrit` entry as a URL: bare change numbers go through the template. */
export function gerritUrl(template: string, entry: string): string {
  const e = entry.trim();
  return /^\d+$/.test(e) ? template.replaceAll("{n}", e) : e;
}

/** Does clipboard text look like something to add as a Gerrit link? */
export function looksLikeGerrit(text: string, template: string): boolean {
  const t = text.trim();
  if (!t || t.includes("\n")) return false;
  if (/^\d+$/.test(t)) return true;
  if (!/^https?:\/\//i.test(t)) return false;
  try {
    const host = new URL(template.replace("{n}", "1")).host;
    if (new URL(t).host === host) return true;
  } catch {
    // fall through
  }
  return /gerrit/i.test(t) || /\/c\/.+\/\+\/\d+/.test(t);
}

/** Text to put inside `[[...]]` for a note: key, date or title. */
export function linkText(hit: { kind: string; title: string }): string {
  return hit.title;
}

/** "Thursday" for a daily note's ISO date. */
export function weekday(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString(undefined, { weekday: "long" });
}
