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

const CHECKBOX_RE = /^(\s*(?:[-*+]|\d+[.)])\s+\[)([ xX>])(\])/;

/**
 * If `line` is a checkbox item, the offset of its mark, the replacement
 * character and whether it's done. `[>]` marks a todo moved to a later
 * daily note; toggling it reopens it.
 */
export function checkboxToggle(line: string): { at: number; insert: string; done: boolean; moved: boolean } | null {
  const m = CHECKBOX_RE.exec(line);
  if (!m) return null;
  const open = m[2] === " ";
  return { at: m[1].length, insert: open ? "x" : " ", done: !open && m[2] !== ">", moved: m[2] === ">" };
}

const pad = (n: number) => String(n).padStart(2, "0");

export function isoDate(d = new Date()): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/**
 * A timestamp in `format` (default `yyyy-MM-dd HH:mm`), plus a trailing
 * space. Tokens: yyyy yy MM dd HH hh mm ss a; other text is kept.
 */
export function timestamp(d = new Date(), format = "yyyy-MM-dd HH:mm"): string {
  const h12 = d.getHours() % 12 || 12;
  const tokens: Record<string, string> = {
    yyyy: String(d.getFullYear()),
    yy: String(d.getFullYear()).slice(2),
    MM: pad(d.getMonth() + 1),
    dd: pad(d.getDate()),
    HH: pad(d.getHours()),
    hh: pad(h12),
    mm: pad(d.getMinutes()),
    ss: pad(d.getSeconds()),
    a: d.getHours() < 12 ? "am" : "pm",
  };
  return `${format.replace(/yyyy|yy|MM|dd|HH|hh|mm|ss|a/g, (t) => tokens[t])} `;
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/**
 * An ISO date (`2026-10-28`) in a format like `dd/MM/yyyy` or `EEE d MMM
 * yyyy` (same tokens as Rust's `dates::format`): `yyyy` `yy`, `MMMM` `MMM`
 * `MM` `M`, `EEEE` `EEE`, `dd` `d`. Anything that isn't an ISO date is
 * returned as it is.
 */
export function formatDate(iso: string, format: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso);
  if (!m) return iso;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  const day = DAYS[new Date(y, mo - 1, d).getDay()];
  const tokens: Record<string, string> = {
    yyyy: m[1],
    yy: m[1].slice(2),
    MMMM: MONTHS[mo - 1],
    MMM: MONTHS[mo - 1].slice(0, 3),
    MM: m[2],
    M: String(mo),
    EEEE: day,
    EEE: day.slice(0, 3),
    dd: m[3],
    d: String(d),
  };
  return format.replace(/yyyy|yy|MMMM|MMM|MM|M|EEEE|EEE|dd|d/g, (t) => tokens[t]);
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
  const start = lines.findIndex((l) => /^#{1,6}\s+to[- ]?dos?\s*$/i.test(l.trimEnd()));
  if (start < 0) return null;
  let last = start;
  for (let i = start + 1; i < lines.length; i++) {
    if (/^#{1,2}\s/.test(lines[i])) break;
    if (lines[i].trim() !== "") last = i;
  }
  return offsets[last] + lines[last].replace(/\r$/, "").length;
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

/** Compact "last edited" time: `5m`, `2h`, `Mon`, `Oct 3`, `Oct 3, 2025`. */
export function relTime(ms: number, now = Date.now()): string {
  if (!ms) return "";
  const diff = Math.max(0, now - ms);
  if (diff < 60_000) return "now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  const d = new Date(ms);
  if (diff < 6 * 86_400_000) return d.toLocaleDateString("en-US", { weekday: "short" });
  const sameYear = d.getFullYear() === new Date(now).getFullYear();
  return d.toLocaleDateString("en-US", sameYear ? { month: "short", day: "numeric" } : { month: "short", day: "numeric", year: "numeric" });
}

/** A tag in canonical form (same rules as Rust's `normalize_tag`): lower case,
 * no leading `#`, letters, digits, `-`, `_` and `/`; not digits alone. */
export function normalizeTag(raw: string): string | null {
  const t = raw.trim().replace(/^#+/, "").replace(/^\/+|\/+$/g, "").toLowerCase();
  if (!t || !/^[\p{L}\p{N}_/-]+$/u.test(t) || /^\d+$/.test(t) || t.includes("//")) return null;
  return t;
}

/** The partial tag being typed at the end of a search (`rollout #bu` -> `bu`), if any. */
export function typingTag(query: string): string | null {
  const m = /(?:^|\s)#([\p{L}\p{N}_/-]*)$/u.exec(query);
  return m ? m[1].toLowerCase() : null;
}

/** Tags to suggest for the tag being typed, most used first. */
export function tagSuggestions(query: string, all: { name: string }[], limit = 6): string[] {
  const part = typingTag(query);
  if (part === null) return [];
  return all
    .map((t) => t.name)
    .filter((t) => t.startsWith(part) && t !== part)
    .slice(0, limit);
}

/** Replace the tag being typed with `tag`. */
export function completeTag(query: string, tag: string): string {
  return query.replace(/#[\p{L}\p{N}_/-]*$/u, `#${tag} `);
}
