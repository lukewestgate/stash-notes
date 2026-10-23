// Link rules (Settings > Links): optional, global ways to recognise a
// service's links, label them, format pasted ones and offer a note
// template. Links are collected without any; matching happens in Rust
// (core/src/links.rs). This file has the presets and the bits the editor
// needs synchronously, such as which bare keys to highlight.

import type { LegacyLinks, LinkRule, NoteTemplate, Settings } from "./api";

export const DEFAULT_KEY_PATTERN = "[A-Z][A-Z0-9]+-\\d+";

const NO_TEMPLATE: NoteTemplate = { enabled: false, title: "{key} {title}", folder: "notes", filename: "", body: "", status: "", tags: [] };

function rule(r: Partial<LinkRule> & Pick<LinkRule, "name" | "icon" | "url">): LinkRule {
  return { id: "", enabled: true, pattern: "", keyPattern: "", label: "{key}", pasteFormat: false, template: { ...NO_TEMPLATE }, ...r };
}

/** Presets: a starting point, adapted to your site by an example link. */
export const RULE_PRESETS: { label: string; hint: string; rule: LinkRule }[] = [
  {
    label: "Jira",
    hint: "a Jira issue link, e.g. https://yourco.atlassian.net/browse/PROJ-123",
    rule: rule({
      name: "Jira",
      icon: "ticket",
      url: "https://yourco.atlassian.net/browse/{key}",
      keyPattern: DEFAULT_KEY_PATTERN,
      template: {
        enabled: true,
        title: "{key} {title}",
        folder: "notes",
        filename: "{key}",
        body: "# {key} {title}\n\n## Context\n\n## Notes\n\n## Next steps\n\n- [ ] \n",
        status: "open",
        tags: [],
      },
    }),
  },
  {
    label: "GitHub issues",
    hint: "an issue in your repository, e.g. https://github.com/owner/repo/issues/42",
    rule: rule({ name: "GitHub issue", icon: "circle-dot", url: "https://github.com/OWNER/REPO/issues/{n}", keyPattern: "GH-\\d+" }),
  },
  {
    label: "GitHub pull requests",
    hint: "a pull request, e.g. https://github.com/owner/repo/pull/7",
    rule: rule({ name: "Pull request", icon: "git-pull-request", url: "https://github.com/OWNER/REPO/pull/{n}", label: "PR {n}" }),
  },
  {
    label: "GitLab issues",
    hint: "an issue, e.g. https://gitlab.com/group/project/-/issues/42",
    rule: rule({ name: "GitLab issue", icon: "bug", url: "https://gitlab.com/GROUP/PROJECT/-/issues/{n}", keyPattern: "GL-\\d+" }),
  },
  {
    label: "GitLab merge requests",
    hint: "a merge request, e.g. https://gitlab.com/group/project/-/merge_requests/7",
    rule: rule({ name: "Merge request", icon: "git-merge", url: "https://gitlab.com/GROUP/PROJECT/-/merge_requests/{n}", label: "MR {n}" }),
  },
  {
    label: "Gerrit",
    hint: "a change, e.g. https://gerrit.example.com/c/project/+/12345",
    rule: rule({ name: "Gerrit", icon: "git-pull-request", url: "https://gerrit.example.com/c/{n}", label: "Gerrit {n}" }),
  },
  {
    label: "Confluence / wiki",
    hint: "a page, e.g. https://yourco.atlassian.net/wiki/spaces/ENG/pages/123/Title",
    rule: rule({ name: "Wiki", icon: "book-open", url: "https://yourco.atlassian.net/wiki/{n}", label: "Page {n}" }),
  },
  {
    label: "YouTube (optional)",
    hint: "video links already work without a rule; this one labels them",
    rule: rule({
      name: "YouTube",
      icon: "circle-play",
      url: "https://www.youtube.com/watch?v={key}",
      pattern: "^https?://(?:www\\.|m\\.)?(?:youtube\\.com/(?:watch\\?(?:[^#]*&)?v=|shorts/)|youtu\\.be/)([\\w-]{6,})",
      label: "Video {key}",
    }),
  },
  {
    label: "Custom",
    hint: "any link with an identifier in it; put {key} where it goes",
    rule: rule({ name: "Links", icon: "link", url: "https://example.com/{key}" }),
  },
];

/** A rule id from a name, unique among `taken` (same rule as Rust's `rule_id`). */
export function ruleId(name: string, taken: string[]): string {
  const base = name.toLowerCase().normalize("NFKD").replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "link";
  let id = base;
  for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
  return id;
}

/** Placeholder words in presets (`OWNER`, `REPO` …) that an example link fills in. */
const SLOT = /^(OWNER|REPO|GROUP|PROJECT|SPACE)$/;

/**
 * A preset's link format for your site, from an example link: its scheme and
 * host, and the example's path segments where the preset has placeholders.
 * `https://github.com/OWNER/REPO/issues/{n}` + `…/acme/app/issues/42` ->
 * `https://github.com/acme/app/issues/{n}`. Null if the example isn't a link.
 */
export function adaptFormat(format: string, example: string): string | null {
  let ex: URL;
  let fmt: URL;
  try {
    ex = new URL(example.trim());
    fmt = new URL(format.replaceAll("{key}", "__KEY__").replaceAll("{n}", "__N__"));
  } catch {
    return null;
  }
  if (!/^https?:$/.test(ex.protocol)) return null;
  const segs = fmt.pathname.split("/");
  const exSegs = ex.pathname.split("/");
  const path = segs.map((s, i) => (SLOT.test(s) && exSegs[i] ? exSegs[i] : s)).join("/");
  const out = `${ex.protocol}//${ex.host}${path}${fmt.search}`;
  return decodeURI(out).replaceAll("__KEY__", "{key}").replaceAll("__N__", "{n}");
}

function compiles(pattern: string): boolean {
  try {
    new RegExp(pattern);
    return true;
  } catch {
    return false;
  }
}

/**
 * The bare keys notes may contain (to highlight and follow): the enabled
 * rules' key patterns, and the 0.6 ticket systems of `vault` (any vault when
 * null) so old ticket notes keep their keys. Null when there are none.
 */
export function keyRegex(settings: Pick<Settings, "linkRules" | "legacyLinks"> | null, vault: string | null): RegExp | null {
  if (!settings) return null;
  const legacy: LegacyLinks["vaults"] = settings.legacyLinks?.vaults ?? {};
  const old = vault === null ? Object.values(legacy) : legacy[vault] ? [legacy[vault]] : [];
  const pats = [
    ...settings.linkRules.filter((r) => r.enabled).map((r) => r.keyPattern.trim()),
    ...old.flatMap((v) => v.tickets.map((t) => t.pattern.trim())),
  ].filter((p, i, all) => p && all.indexOf(p) === i && compiles(p));
  return pats.length ? new RegExp(pats.map((p) => `(?:${p})`).join("|")) : null;
}

/** Is `text` (as a whole, any case) a key some rule reads? */
export function isKey(settings: Pick<Settings, "linkRules" | "legacyLinks"> | null, text: string): boolean {
  const re = keyRegex(settings, null);
  return !!re && new RegExp(`^(?:${re.source})$`).test(text.trim().toUpperCase());
}

/** Is `text` (as a whole) an http(s) link? */
export function isUrl(text: string): boolean {
  const t = text.trim();
  if (!/^https?:\/\/\S+$/i.test(t)) return false;
  try {
    return !!new URL(t).host;
  } catch {
    return false;
  }
}

/** Link input for creating a note: a link, or a known key, then maybe a title. */
export function isLinkInput(settings: Pick<Settings, "linkRules" | "legacyLinks"> | null, text: string): boolean {
  const first = text.trim().split(/\s+/)[0] ?? "";
  return isUrl(first) || isKey(settings, first.replace(/[:,]$/, ""));
}
