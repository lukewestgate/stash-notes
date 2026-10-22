// A vault's links to other systems (Settings > Vaults > Links): ticket
// systems, whose items get a note each (Jira, GitHub issues), and link types
// a note collects in front matter (Gerrit changes, pull requests, pages).
// A vault with neither is a plain notes vault.

import type { LinkType, NoteMeta, TicketSystem, VaultDef } from "./api";

export const DEFAULT_TICKET_PATTERN = "[A-Z][A-Z0-9]+-\\d+";

export const TICKET_PRESETS: { label: string; system: TicketSystem }[] = [
  {
    label: "Jira",
    system: { name: "Jira", icon: "ticket", pattern: DEFAULT_TICKET_PATTERN, url: "https://yourco.atlassian.net/browse/{key}" },
  },
  {
    label: "GitHub issues",
    system: { name: "GitHub issue", icon: "circle-dot", pattern: "GH-\\d+", url: "https://github.com/OWNER/REPO/issues/{n}" },
  },
  {
    label: "GitLab issues",
    system: { name: "GitLab issue", icon: "bug", pattern: "GL-\\d+", url: "https://gitlab.com/GROUP/PROJECT/-/issues/{n}" },
  },
  { label: "Custom", system: { name: "Tickets", icon: "ticket", pattern: "[A-Z]+-\\d+", url: "https://example.com/{key}" } },
];

export const LINK_PRESETS: { label: string; type: LinkType }[] = [
  { label: "Gerrit", type: { name: "Gerrit", icon: "git-pull-request", field: "gerrit", url: "https://gerrit.example.com/c/{n}" } },
  {
    label: "GitHub pull requests",
    type: { name: "PR", icon: "git-pull-request", field: "prs", url: "https://github.com/OWNER/REPO/pull/{n}" },
  },
  {
    label: "GitLab merge requests",
    type: { name: "MR", icon: "git-merge", field: "mrs", url: "https://gitlab.com/GROUP/PROJECT/-/merge_requests/{n}" },
  },
  {
    label: "Confluence / wiki",
    type: { name: "Wiki", icon: "book-open", field: "docs", url: "https://yourco.atlassian.net/wiki/pages/viewpage.action?pageId={n}" },
  },
  { label: "Custom", type: { name: "Links", icon: "link", field: "links", url: "https://example.com/{n}" } },
];

function compiles(pattern: string): boolean {
  try {
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

/** Every ticket key of the vault (all its systems), or null for a plain vault. */
export function vaultTickets(v: VaultDef | undefined): RegExp | null {
  const pats = (v?.tickets ?? []).map((t) => t.pattern).filter(compiles);
  return pats.length ? new RegExp(pats.map((p) => `(?:${p})`).join("|")) : null;
}

/** Ticket keys of any vault (for the pop-up, which belongs to none). */
export function anyTickets(vaults: VaultDef[]): RegExp | null {
  const pats = vaults.flatMap((v) => v.tickets.map((t) => t.pattern)).filter(compiles);
  return pats.length ? new RegExp(pats.map((p) => `(?:${p})`).join("|")) : null;
}

function full(pattern: string): RegExp | null {
  return compiles(pattern) ? new RegExp(`^(?:${pattern})$`) : null;
}

/** The ticket system `key` belongs to in this vault. */
export function systemForKey(v: VaultDef | undefined, key: string): TicketSystem | null {
  // A fixed prefix (`GH-\d+`) is more specific than a general pattern (Jira's).
  const ordered = [...(v?.tickets ?? [])].sort((a, b) => Number(!keyPrefix(a.pattern)) - Number(!keyPrefix(b.pattern)));
  return ordered.find((t) => full(t.pattern)?.test(key)) ?? null;
}

/** `PROJ-123` -> its trailing number, `123`. */
function keyNumber(key: string): string {
  return /(\d+)$/.exec(key)?.[1] ?? key;
}

export function ticketUrl(system: TicketSystem, key: string): string {
  return system.url.replaceAll("{key}", key).replaceAll("{n}", keyNumber(key));
}

/** A front matter entry as a URL: URLs as they are, anything else through the template. */
export function linkUrl(type: LinkType, item: string): string {
  const e = item.trim();
  return /^https?:\/\//i.test(e) ? e : type.url.replaceAll("{n}", e).replaceAll("{key}", e);
}

/** `GH-\d+` -> `GH`: the fixed part of a key, to turn `{n}` back into a key. */
function keyPrefix(pattern: string): string | null {
  return /^([A-Za-z][A-Za-z0-9_]*)-\\d\+$/.exec(pattern)?.[1] ?? null;
}

/** A URL template as a regex for URLs it produces, capturing the placeholder. */
function templateRegex(template: string): RegExp | null {
  if (!/\{(key|n)\}/.test(template)) return null;
  const esc = template.replace(/[.*+?^$()|[\]\\]/g, "\\$&");
  return new RegExp(`^${esc.replace(/\{(?:key|n)\}/, "([^/?#&]+)").replace(/\{(?:key|n)\}/g, "[^/?#&]+")}(?:[/?#].*)?$`, "i");
}

function hostOf(template: string): string | null {
  try {
    return new URL(template.replace(/\{(?:key|n)\}/g, "1")).host;
  } catch {
    return null;
  }
}

function asUrl(text: string): URL | null {
  const t = text.trim();
  if (!/^https?:\/\/\S+$/i.test(t)) return null;
  try {
    return new URL(t);
  } catch {
    return null;
  }
}

/**
 * The ticket a pasted URL points at, in any of `vaults`: a URL its template
 * makes (`.../browse/PROJ-123`, `.../issues/42` -> `GH-42`), or, on the same
 * site, a key in the URL (Jira boards: `selectedIssue=PROJ-123`).
 */
export function ticketFromUrl(text: string, vaults: VaultDef[]): { key: string; vault: string } | null {
  const url = asUrl(text);
  if (!url) return null;
  const href = text.trim();
  for (const v of vaults) {
    for (const t of v.tickets) {
      const keyRe = full(t.pattern);
      if (!keyRe) continue;
      const m = templateRegex(t.url)?.exec(href);
      if (m) {
        const c = decodeURIComponent(m[1]);
        if (t.url.includes("{key}") && keyRe.test(c)) return { key: c, vault: v.id };
        const prefix = keyPrefix(t.pattern);
        if (t.url.includes("{n}") && /^\d+$/.test(c) && prefix) return { key: `${prefix}-${c}`, vault: v.id };
      }
      // Jira-style URLs anywhere: a board's `selectedIssue=KEY`, or `/browse/KEY`.
      if (t.url.includes("{key}")) {
        const selected = url.searchParams.get("selectedIssue");
        if (selected && keyRe.test(selected)) return { key: selected, vault: v.id };
        const browse = /\/browse\/([^/?#]+)/.exec(url.pathname)?.[1];
        if (browse && keyRe.test(browse)) return { key: browse, vault: v.id };
      }
      if (url.host === hostOf(t.url)) {
        const selected = url.searchParams.get("selectedIssue");
        if (selected && keyRe.test(selected)) return { key: selected, vault: v.id };
        const found = new RegExp(`(?:${t.pattern})`).exec(decodeURIComponent(url.pathname));
        if (found && keyRe.test(found[0])) return { key: found[0], vault: v.id };
      }
    }
  }
  return null;
}

/** The link type a pasted URL belongs to: one its template makes, else one on the same site. */
export function linkTypeForUrl(text: string, v: VaultDef | undefined): LinkType | null {
  const url = asUrl(text);
  if (!url || !v) return null;
  const href = text.trim();
  return v.links.find((l) => templateRegex(l.url)?.test(href)) ?? v.links.find((l) => hostOf(l.url) === url.host) ?? null;
}

export interface LinkItem {
  type: LinkType;
  item: string;
  url: string;
}

/** The note's attached links, for the vault's link types, in their order. */
export function linkItems(meta: NoteMeta, v: VaultDef | undefined): LinkItem[] {
  return (v?.links ?? []).flatMap((type) => (meta.fields[type.field] ?? []).map((item) => ({ type, item, url: linkUrl(type, item) })));
}
