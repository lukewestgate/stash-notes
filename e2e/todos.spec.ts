import { expect, test, type Page } from "@playwright/test";

async function setDocument(page: Page, text: string) {
  await page.evaluate((text) => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: { anchor: 0 } });
  }, text);
}

const doc = () => (window as any).__view().state.doc.toString();

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
});

test("a moved todo shows its own box and reopens on click", async ({ page }) => {
  await setDocument(page, "# Day\n\n## Todo\n\n- [>] moved on\n- [ ] still open\n- [x] done\n");
  const moved = page.locator(".cm-task-moved");
  await expect(moved).toHaveCount(1);
  await expect(moved).toHaveAttribute("aria-checked", "mixed");
  await expect(page.locator(".cm-moved-line")).toHaveText("moved on");
  await moved.click();
  expect(await page.evaluate(doc)).toContain("- [ ] moved on");
});

test("daily notes are named in the chosen date format", async ({ page }) => {
  const iso: string = await page.evaluate(() => (window as any).__view().state.doc.lineAt(0).text.slice(2));
  const [y, m, d] = iso.split("-");
  await page.keyboard.press("Control+,");
  await page.getByRole("button", { name: "Notes", exact: true }).click();
  await page.locator("select").filter({ hasText: "22/10/2026" }).selectOption("dd/MM/yyyy");
  await page.keyboard.press("Escape");
  await expect(page.locator(".header .main")).toHaveText(`${d}/${m}/${y}`);
});
