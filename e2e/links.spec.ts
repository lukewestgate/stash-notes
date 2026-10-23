import { expect, test, type Page } from "@playwright/test";

async function setDocument(page: Page, text: string) {
  await page.evaluate((text) => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: { anchor: text.length } });
  }, text);
}

function doc(page: Page) {
  return page.evaluate(() => (window as any).__view().state.doc.toString() as string);
}

async function paste(page: Page, text: string) {
  await page.locator(".cm-content").evaluate((el, text) => {
    const data = new DataTransfer();
    data.setData("text/plain", text);
    el.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  }, text);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api, mockEmit } = await import("/src/lib/api.ts");
    mockEmit("open-note", await api.createNote("Links test", "personal"));
  });
  await expect(page.locator(".header .main")).toHaveText("Links test");
});

test("collects pasted and typed links without setup, leaving code out", async ({ page }) => {
  const header = page.getByRole("button", { name: /^Links \(\d+\)$/ });
  await expect(header).toHaveText("Links (0)");
  await header.click();
  const panel = page.getByRole("complementary", { name: "Links" });
  await expect(panel.getByText("Links you paste into this note appear here.")).toBeVisible();
  await setDocument(page, "# Links test\n\nWatch https://www.youtube.com/watch?v=abc123 and [the plan](https://docs.example.com/plan).\n\n`https://code.example.com/x`\n\nAgain: https://www.youtube.com/watch?v=abc123\n");
  await expect(header).toHaveText("Links (2)");
  const rows = panel.getByRole("listitem");
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(0)).toContainText("https://www.youtube.com/watch?v=abc123");
  await expect(rows.nth(0)).toContainText("×2");
  await expect(rows.nth(1)).toContainText("the plan");
  await expect(rows.nth(1)).toContainText("docs.example.com");
  // Go to source puts the caret on the link in the text.
  await rows.nth(1).getByRole("button", { name: "Go to source of the plan" }).click();
  await expect(page.locator(".cm-content")).toBeFocused();
  const selected = await page.evaluate(() => {
    const v = (window as any).__view();
    return v.state.sliceDoc(v.state.selection.main.from, v.state.selection.main.to);
  });
  expect(selected).toBe("[the plan](https://docs.example.com/plan)");
});

test("adds a link from the panel, keyboard only, and Esc returns to the editor", async ({ page }) => {
  await page.keyboard.press("Control+Shift+L");
  const panel = page.getByRole("complementary", { name: "Links" });
  await expect(panel).toBeVisible();
  await panel.getByRole("button", { name: "Add link" }).first().click();
  await page.getByRole("dialog", { name: "Add a link to this note" }).getByRole("textbox").fill("https://example.org/spec");
  await page.keyboard.press("Enter");
  await expect(panel.getByRole("listitem")).toHaveCount(1);
  expect(await doc(page)).toContain("links:\n  - https://example.org/spec");
  await expect(panel.getByRole("button", { name: /Edit attached link/ })).toBeVisible();
  await panel.getByRole("button", { name: /^Open .* in browser$/ }).focus();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect(page.locator(".cm-content")).toBeFocused();
});

test("All notes lists links from every vault with their notes", async ({ page }) => {
  await page.getByRole("button", { name: /^Links \(all notes\)$/ }).click();
  const panel = page.getByRole("complementary", { name: "Links" });
  await expect(panel.getByRole("tab", { name: "All notes" })).toHaveAttribute("aria-selected", "true");
  const rollout = panel.getByRole("listitem").filter({ hasText: "https://example.com/rollout" });
  await expect(rollout).toHaveCount(1);
  await expect(rollout.getByRole("button", { name: "Open note Rollout plan in Work" })).toBeVisible();
  await panel.getByRole("textbox", { name: "Filter links" }).fill("gerrit");
  await expect(panel.getByRole("listitem").first()).toContainText("gerrit.example.com");
  await expect(panel.getByRole("listitem").filter({ hasText: "example.com/rollout" })).toHaveCount(0);
  await panel.getByRole("listitem").first().getByRole("button", { name: /^Open note PROJ-123/ }).click();
  await expect(page.locator(".header .main")).toHaveText("PROJ-123");
});

test("a rule's paste format keeps the destination, and undo restores the text", async ({ page }) => {
  await setDocument(page, "# Links test\n\n");
  await paste(page, "https://yourco.atlassian.net/browse/PROJ-9");
  await expect.poll(() => doc(page)).toContain("[PROJ-9](https://yourco.atlassian.net/browse/PROJ-9)");
  await page.keyboard.press("Control+z");
  await expect.poll(() => doc(page)).toBe("# Links test\n\n");
  // Several links with prose stay as pasted.
  await paste(page, "see https://yourco.atlassian.net/browse/PROJ-9 and https://example.com/a");
  await expect.poll(() => doc(page)).toContain("see https://yourco.atlassian.net/browse/PROJ-9 and https://example.com/a");
  await expect(page.getByRole("button", { name: /^Links \(\d+\)$/ })).toHaveText("Links (2)");
});

test("a primary link opens with Ctrl+J, and changing it keeps the old one attached", async ({ page }) => {
  await setDocument(page, "---\nprimary_link: https://example.com/one\n---\n# Links test\n");
  await expect(page.getByRole("button", { name: "Open example.com" })).toBeVisible();
  await page.getByRole("button", { name: /^Links \(\d+\)$/ }).click();
  const panel = page.getByRole("complementary", { name: "Links" });
  await panel.getByRole("button", { name: "Add link" }).first().click();
  await page.getByRole("dialog").getByRole("textbox").fill("https://example.com/two");
  await page.keyboard.press("Enter");
  await panel.getByRole("button", { name: "More actions for https://example.com/two" }).click();
  await page.getByRole("option", { name: /Make it the primary link/ }).click();
  await expect.poll(() => doc(page)).toContain("primary_link: https://example.com/two");
  expect(await doc(page)).toContain("  - https://example.com/one");
});
