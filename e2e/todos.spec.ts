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

test("+ in the switcher adds a todo to today's note", async ({ page }) => {
  await page.keyboard.press("Control+k");
  await page.keyboard.type("+ chase Sam about the demo");
  await expect(page.locator(".row.selected .name")).toHaveText('Add "chase Sam about the demo"');
  await page.keyboard.press("Enter");
  await expect(page.locator(".toast").last()).toContainText("Added to");
  await expect.poll(() => page.evaluate(doc)).toContain("- [ ] chase Sam about the demo\n- [ ] \n");
});

test("a date query shows the day it means and offers its daily note", async ({ page }) => {
  await page.keyboard.press("Control+k");
  await page.keyboard.type("yesterday");
  await expect(page.locator(".datehint")).toBeVisible();
  await expect(page.locator(".row.create .name")).toHaveText(/^Create daily note for /);
});

test("carried todos show how long they've been open", async ({ page }) => {
  await setDocument(page, "## Todo\n\n- [ ] old thing\n- [ ] new thing\n- [x] old thing done\n");
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { decoConfig, refreshDecorations } = await import("/src/lib/editor.ts");
    decoConfig.todoAges = new Map([
      ["old thing", { since: "2026-10-22", days: 6 }],
      ["new thing", { since: "2026-10-27", days: 1 }],
    ]);
    refreshDecorations((window as any).__view());
  });
  await expect(page.locator(".cm-todo-age")).toHaveCount(1);
  await expect(page.locator(".cm-todo-age")).toHaveText("6d");
});

test("Alt+PageUp steps to the previous daily note", async ({ page }) => {
  // The demo vault has one daily note.
  await page.keyboard.press("Alt+PageUp");
  await expect(page.locator(".toast").last()).toContainText("No earlier daily note");
});
