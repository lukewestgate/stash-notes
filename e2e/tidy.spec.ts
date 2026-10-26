import { expect, test, type Page } from "@playwright/test";

async function noteText(page: Page, path: string) {
  return page.evaluate(async (path) => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    return (await api.bufferOpen(path)).text;
  }, path);
}

async function exists(page: Page, path: string) {
  return noteText(page, path).then(() => true, () => false);
}

test.describe("a vault from before 0.8", () => {
  test.beforeEach(async ({ page }) => {
    // The work vault starts laid out the pre-0.8 way (daily/, tickets/, archive/).
    await page.addInitScript(() => sessionStorage.setItem("mock-legacy", "1"));
    await page.goto("/");
    await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
  });

  test("offers to tidy after the upgrade, moves every note into notes/, and Undo puts it back", async ({ page }) => {
    const offer = page.locator(".toast").filter({ hasText: "Tidy Work's folders?" });
    await offer.getByRole("button", { name: "Review…" }).click();
    const dialog = page.getByRole("dialog", { name: "Tidy Work's folders" });
    // OPS-4123-2.md (renamed, below) is no longer named by its key, so it counts as a regular note.
    await expect(dialog).toContainText("1 daily note, 1 ticket note, 2 other notes move into notes");
    await expect(dialog).toContainText("2 archived notes stay archived");
    // The demo already has notes/OPS-4123.md, so the archived one gets a number.
    await expect(dialog).toContainText("archive/tickets/OPS-4123.md → notes/OPS-4123-2.md");
    await expect(dialog.getByRole("button", { name: "Tidy folders" })).toBeFocused();
    await dialog.getByRole("button", { name: "Tidy folders" }).click();
    await expect(dialog).toHaveCount(0);
    const done = page.locator(".toast").filter({ hasText: "Tidied Work: 4 notes moved into notes, links updated in 1 note" });
    await expect(done).toBeVisible();

    expect(await noteText(page, "work:notes/2026-10-01.md")).toContain("[[work:notes/OPS-1]]");
    expect(await noteText(page, "work:notes/older-idea.md")).toContain("archived: true");
    expect(await noteText(page, "work:notes/OPS-4123-2.md")).toContain("archived: true");
    expect(await noteText(page, "personal:notes/links-to-work.md")).toContain("[[work:notes/OPS-1|cert ticket]]");
    expect(await exists(page, "work:tickets/OPS-1.md")).toBe(false);

    await done.getByRole("button", { name: "Undo" }).click();
    await expect(page.locator(".toast").filter({ hasText: "Put Work's folders back" })).toBeVisible();
    expect(await noteText(page, "work:tickets/OPS-1.md")).toContain("# OPS-1 Rotate certificates");
    expect(await noteText(page, "work:archive/notes/older-idea.md")).not.toContain("archived:");
    expect(await noteText(page, "personal:notes/links-to-work.md")).toContain("[[work:tickets/OPS-1|cert ticket]]");
    expect(await exists(page, "work:notes/older-idea.md")).toBe(false);
  });

  test("Not now puts the offer off, and Settings → Vaults still offers the tidy", async ({ page }) => {
    await page.locator(".toast").filter({ hasText: "Tidy Work's folders?" }).getByRole("button", { name: "Review…" }).click();
    await page.getByRole("dialog", { name: "Tidy Work's folders" }).getByRole("button", { name: "Not now" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.locator(".toast").filter({ hasText: "Settings → Vaults" })).toBeVisible();
    await page.keyboard.press("Control+,");
    await page.getByRole("button", { name: "Vaults", exact: true }).click();
    await page.getByRole("button", { name: "Tidy folders…" }).click();
    await expect(page.getByRole("dialog", { name: "Tidy Work's folders" })).toBeVisible();
  });
});

test.describe("a vault in the new layout", () => {
  test("isn't offered a tidy", async ({ page }) => {
    await page.goto("/");
    await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
    await expect.poll(() => page.evaluate(async () => {
      // @ts-expect-error Vite development module.
      const { api } = await import("/src/lib/api.ts");
      return api.tidyNeeded();
    })).toEqual([]);
    await expect(page.locator(".toast").filter({ hasText: "Tidy" })).toHaveCount(0);
  });
});
