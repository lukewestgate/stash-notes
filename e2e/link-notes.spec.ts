import { expect, test, type Page } from "@playwright/test";

function doc(page: Page) {
  return page.evaluate(() => (window as any).__view().state.doc.toString() as string);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
});

test("a pasted link in the switcher offers a note, keeping the link, and creates nothing until asked", async ({ page }) => {
  await page.keyboard.press("Control+k");
  const switcher = page.getByRole("dialog", { name: "Quick switcher" });
  const input = switcher.getByRole("textbox").first();
  await input.fill("https://docs.example.com/startup-plan");
  await expect(input).toHaveValue("https://docs.example.com/startup-plan");
  const row = switcher.getByRole("button", { name: /Create note from https:\/\/docs\.example\.com\/startup-plan/ });
  await expect(row).toBeVisible();
  await row.click();
  const form = switcher.getByRole("group", { name: "Create note from link" });
  await expect(form).toContainText("No template — a plain note");
  await expect(form.getByLabel("Title")).toBeFocused();
  // Esc goes back to the results; the query is still there.
  await page.keyboard.press("Escape");
  await expect(form).toHaveCount(0);
  await expect(input).toHaveValue("https://docs.example.com/startup-plan");
  await row.click();
  await form.getByLabel("Title").fill("Startup plan");
  await page.keyboard.press("Enter");
  await expect(page.locator(".header .main")).toHaveText("Startup plan");
  expect(await doc(page)).toContain("primary_link: https://docs.example.com/startup-plan");
  await expect(page.getByRole("button", { name: "Open docs.example.com" })).toBeVisible();
});

test("a key uses its rule's template, and offers the existing note next time", async ({ page }) => {
  await page.keyboard.press("Control+k");
  const switcher = page.getByRole("dialog", { name: "Quick switcher" });
  await switcher.getByRole("textbox").first().fill("PROJ-777 Investigate startup delay");
  await switcher.getByRole("button", { name: /Create note from PROJ-777/ }).click();
  const form = switcher.getByRole("group", { name: "Create note from link" });
  await expect(form).toContainText("Jira template");
  await expect(form).toContainText("notes/PROJ-777.md");
  await expect(form.getByLabel("Title")).toHaveValue("Investigate startup delay");
  await form.getByRole("button", { name: /^Create note/ }).click();
  await expect(page.locator(".header .main")).toHaveText("PROJ-777");
  expect(await doc(page)).toContain("primary_link: https://yourco.atlassian.net/browse/PROJ-777");
  expect(await doc(page)).toContain("# PROJ-777 Investigate startup delay");

  await page.keyboard.press("Control+k");
  await switcher.getByRole("textbox").first().fill("https://yourco.atlassian.net/browse/PROJ-777");
  await switcher.getByRole("button", { name: /Create note from https/ }).click();
  await expect(form.getByRole("button", { name: /Open existing/ })).toBeVisible();
  await expect(form.getByRole("button", { name: /Create another note/ })).toBeVisible();
  await expect(form).toContainText("PROJ-777-2.md");
});

test("the Notes view creates from a link through the same form", async ({ page }) => {
  await page.keyboard.press("Control+l");
  await page.getByPlaceholder(/Search notes/).fill("https://www.youtube.com/watch?v=xyz789");
  await page.getByText("Create note from https://www.youtube.com/watch?v=xyz789").click();
  const form = page.getByRole("dialog", { name: "Create note from link" });
  await expect(form).toBeVisible();
  await form.getByLabel("Title").fill("A talk");
  await form.getByRole("button", { name: /^Create note/ }).click();
  await expect(page.locator(".header .main")).toHaveText("A talk");
  expect(await doc(page)).toContain("primary_link: https://www.youtube.com/watch?v=xyz789");
});

test("following a bare key opens its existing note, else offers to create it", async ({ page }) => {
  await page.evaluate(() => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "See PROJ-123 and PROJ-4242\n" }, selection: { anchor: 6 } });
  });
  await page.keyboard.press("Control+Enter");
  await expect(page.locator(".header .main")).toHaveText("PROJ-123");
  await page.keyboard.press("Alt+ArrowLeft");
  await expect(page.locator(".header .main")).not.toHaveText("PROJ-123");
  await page.evaluate(() => (window as any).__view().dispatch({ selection: { anchor: 20 } }));
  await page.keyboard.press("Control+Enter");
  const form = page.getByRole("dialog", { name: "Create note from link" });
  await expect(form).toContainText("https://yourco.atlassian.net/browse/PROJ-4242");
  await page.keyboard.press("Escape");
  await expect(form).toHaveCount(0);
});
