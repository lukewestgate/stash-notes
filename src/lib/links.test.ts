import { describe, expect, it } from "vitest";
import type { VaultDef } from "./api";
import { LINK_PRESETS, TICKET_PRESETS, linkTypeForUrl, linkUrl, systemForKey, ticketFromUrl, ticketUrl, vaultTickets } from "./links";

const work: VaultDef = {
  id: "work",
  name: "Work",
  path: "",
  color: "orange",
  tickets: [
    { ...TICKET_PRESETS[0].system, url: "https://acme.atlassian.net/browse/{key}" },
    { ...TICKET_PRESETS[1].system, url: "https://github.com/acme/app/issues/{n}" },
  ],
  links: [
    { ...LINK_PRESETS[0].type, url: "https://gerrit.acme.com/c/{n}" },
    { ...LINK_PRESETS[1].type, url: "https://github.com/acme/app/pull/{n}" },
  ],
};
const home: VaultDef = { id: "home", name: "Home", path: "", color: "blue", tickets: [], links: [] };

describe("links", () => {
  it("matches keys per vault", () => {
    expect(vaultTickets(home)).toBeNull();
    expect(vaultTickets(work)!.test("GH-12")).toBe(true);
    expect(systemForKey(work, "GH-12")?.name).toBe("GitHub issue");
    expect(systemForKey(work, "PROJ-1")?.name).toBe("Jira");
  });

  it("builds URLs", () => {
    expect(ticketUrl(work.tickets[0], "PROJ-1")).toBe("https://acme.atlassian.net/browse/PROJ-1");
    expect(ticketUrl(work.tickets[1], "GH-12")).toBe("https://github.com/acme/app/issues/12");
    expect(linkUrl(work.links[0], "12345")).toBe("https://gerrit.acme.com/c/12345");
    expect(linkUrl(work.links[0], "https://x/y")).toBe("https://x/y");
  });

  it("reads pasted URLs", () => {
    expect(ticketFromUrl("https://acme.atlassian.net/browse/PROJ-7", [home, work])).toEqual({ key: "PROJ-7", vault: "work" });
    expect(ticketFromUrl("https://acme.atlassian.net/jira/board?selectedIssue=OPS-2", [work])?.key).toBe("OPS-2");
    expect(ticketFromUrl("https://github.com/acme/app/issues/42", [work])?.key).toBe("GH-42");
    expect(ticketFromUrl("https://github.com/acme/app/pull/42", [work])).toBeNull();
    expect(ticketFromUrl("PROJ-7", [work])).toBeNull();
    expect(ticketFromUrl("https://acme.atlassian.net/browse/PROJ-123?focusedId=1#x", [work])?.key).toBe("PROJ-123");
    expect(ticketFromUrl("https://other.net/jira/browse/OPS-9", [work])?.key).toBe("OPS-9");
    expect(ticketFromUrl("https://acme.atlassian.net/issues/PROJ-5/details", [work])?.key).toBe("PROJ-5");
    expect(ticketFromUrl("https://example.com/PROJ-5", [work])).toBeNull();
    expect(ticketFromUrl("see https://acme.atlassian.net/browse/PROJ-1", [work])).toBeNull();
    expect(linkTypeForUrl("https://github.com/acme/app/pull/42", work)?.name).toBe("PR");
    expect(linkTypeForUrl("https://gerrit.acme.com/c/proj/+/99", work)?.name).toBe("Gerrit");
    expect(linkTypeForUrl("https://example.org/x", work)).toBeNull();
    expect(linkTypeForUrl("https://gerrit.acme.com/c/1", home)).toBeNull();
  });
});
