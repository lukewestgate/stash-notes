import { describe, expect, it } from "vitest";
import type { LinkRule } from "./api";
import { RULE_PRESETS, adaptFormat, isKey, isLinkInput, isUrl, keyRegex, ruleId } from "./links";

const jira: LinkRule = { ...RULE_PRESETS[0].rule, id: "jira", url: "https://acme.atlassian.net/browse/{key}" };
const gh: LinkRule = { ...RULE_PRESETS[1].rule, id: "gh" };

describe("links", () => {
  it("adapts presets to an example link", () => {
    expect(adaptFormat("https://yourco.atlassian.net/browse/{key}", "https://acme.atlassian.net/browse/PROJ-1")).toBe(
      "https://acme.atlassian.net/browse/{key}",
    );
    expect(adaptFormat("https://github.com/OWNER/REPO/issues/{n}", "https://github.com/acme/app/issues/42")).toBe(
      "https://github.com/acme/app/issues/{n}",
    );
    expect(adaptFormat("https://gitlab.com/GROUP/PROJECT/-/merge_requests/{n}", "http://git.acme.dev:8080/web/site/-/merge_requests/3")).toBe(
      "http://git.acme.dev:8080/web/site/-/merge_requests/{n}",
    );
    expect(adaptFormat("https://www.youtube.com/watch?v={key}", "https://youtu.be/abc")).toBe("https://youtu.be/watch?v={key}");
    expect(adaptFormat("https://x.com/{key}", "not a link")).toBeNull();
  });

  it("knows the keys rules read, in every vault, and old ticket keys per vault", () => {
    const settings = {
      linkRules: [jira, { ...gh, enabled: false }],
      legacyLinks: { vaults: { work: { tickets: [{ name: "Ops", icon: "ticket", pattern: "OPS-\\d+", url: "https://o/{key}" }], links: [] } } },
    };
    expect(keyRegex(settings, "home")!.test("PROJ-1")).toBe(true);
    expect(keyRegex({ linkRules: [], legacyLinks: undefined }, "home")).toBeNull();
    expect(keyRegex({ linkRules: [], legacyLinks: settings.legacyLinks }, "work")!.test("OPS-4")).toBe(true);
    expect(keyRegex({ linkRules: [], legacyLinks: settings.legacyLinks }, "home")).toBeNull();
    expect(isKey(settings, "proj-12")).toBe(true);
    expect(isKey(settings, "Rollout plan")).toBe(false);
    expect(isLinkInput(settings, "PROJ-12: Fix the login")).toBe(true);
    expect(isLinkInput(settings, "https://www.youtube.com/watch?v=x A talk")).toBe(true);
    expect(isLinkInput(settings, "Rollout plan")).toBe(false);
    expect(isUrl("https://x.com/a b")).toBe(false);
  });

  it("makes unique ids", () => {
    expect(ruleId("Jira (Work)", [])).toBe("jira-work");
    expect(ruleId("Jira", ["jira"])).toBe("jira-2");
  });
});
