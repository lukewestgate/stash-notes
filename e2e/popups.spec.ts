import { expect, test, type Page } from "@playwright/test";

async function clickOutside(page: Page) {
  await page.mouse.click(5, 5);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api, mockEmit } = await import("/src/lib/api.ts");
    mockEmit("open-note", await api.createNote("Popup test", "personal"));
  });
  await expect(page.locator(".header .main")).toHaveText("Popup test");
});

test("switcher stays open for inside clicks and dismisses outside without opening a note", async ({ page }) => {
  await page.keyboard.press("Control+k");
  const popup = page.getByRole("dialog", { name: "Quick switcher", exact: true });
  await popup.getByRole("textbox").fill("Accidental creation");
  await popup.locator(".hints").click();
  await expect(popup).toBeVisible();
  await clickOutside(page);
  await expect(popup).toHaveCount(0);
  await expect(page.locator(".cm-content")).toBeFocused();
  await expect(page.locator(".header .main")).toHaveText("Popup test");
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog").getByRole("textbox")).toHaveValue("");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toBeFocused();
});

test("Ctrl+K toggles the switcher without repeating while held", async ({ page }) => {
  const popup = page.getByRole("dialog", { name: "Quick switcher", exact: true });
  await page.keyboard.down("Control");
  await page.keyboard.down("k");
  await expect(popup).toBeVisible();
  await page.keyboard.down("k");
  await expect(popup).toBeVisible();
  await page.keyboard.up("k");
  await page.keyboard.down("k");
  await expect(popup).toHaveCount(0);
  await page.keyboard.down("k");
  await expect(popup).toHaveCount(0);
  await page.keyboard.up("k");
  await page.keyboard.up("Control");
  await expect(page.locator(".cm-content")).toBeFocused();
});

test("the configured switcher shortcut also closes it", async ({ page }) => {
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const settings = structuredClone(await api.getSettings());
    settings.keys.quickSwitcher = "Ctrl+Shift+K";
    await api.setSettings(settings);
  });
  await expect(page.locator(".header .title")).toHaveAttribute("title", "Switch note (Ctrl+Shift+K)");
  await page.keyboard.press("Control+Shift+k");
  await expect(page.getByRole("dialog", { name: "Quick switcher", exact: true })).toBeVisible();
  await page.keyboard.press("Control+Shift+k");
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("outside dismissal returns focus to the Notes view", async ({ page }) => {
  await page.keyboard.press("Control+l");
  const search = page.getByPlaceholder(/^Search notes/);
  await expect(search).toBeFocused();
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog", { name: "Quick switcher", exact: true })).toBeVisible();
  await clickOutside(page);
  await expect(search).toBeFocused();
});

test("dismissing the link-selection switcher leaves the selected text unchanged", async ({ page }) => {
  const text = await page.evaluate(() => {
    const view = (window as any).__view();
    view.dispatch({ selection: { anchor: 2, head: 12 } });
    return view.state.doc.toString();
  });
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog", { name: "Link to note", exact: true })).toBeVisible();
  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__view().state.doc.toString())).toBe(text);
});

for (const popup of [
  { name: "rename prompt", key: "F2", role: "dialog" as const },
  { name: "status picker", key: "Control+Shift+s", role: "dialog" as const },
  { name: "tag picker", key: "Control+Shift+t", role: "dialog" as const },
  { name: "vault picker", key: "Control+Shift+o", role: "listbox" as const },
  { name: "help popup", key: "Control+/", role: "dialog" as const },
]) {
  test(`${popup.name} dismisses outside and keeps the note open`, async ({ page }) => {
    await page.keyboard.press(popup.key);
    await expect(page.getByRole(popup.role)).toBeVisible();
    await clickOutside(page);
    await expect(page.getByRole(popup.role)).toHaveCount(0);
    await expect(page.locator(".cm-content")).toBeFocused();
    await expect(page.locator(".header .main")).toHaveText("Popup test");
  });
}

test("an outside click cancels the delete confirmation", async ({ page }) => {
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  await expect(page.getByRole("dialog", { name: "Delete note", exact: true })).toBeVisible();
  await clickOutside(page);
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".header .main")).toHaveText("Popup test");
});

test("outside dismissal returns focus to a pinned note", async ({ page }) => {
  await page.goto("/?pin=work%3Anotes%2Frollout-plan.md");
  await expect(page.locator(".bar .title")).toHaveText("Rollout plan");
  await page.keyboard.press("Control+Shift+s");
  await expect(page.getByRole("dialog", { name: "Status", exact: true })).toBeVisible();
  await clickOutside(page);
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toBeFocused();
});

test("a new note has a properties row whose pickers open beside it", async ({ page }) => {
  const row = page.locator(".cm-properties");
  await expect(row).toBeVisible();
  const chip = row.getByRole("button", { name: "status", exact: true });
  const at = (await chip.boundingBox())!;
  await chip.dispatchEvent("mousedown");
  const picker = page.getByRole("dialog", { name: "Status for Popup test" });
  await expect.poll(async () => Math.round((await picker.boundingBox())!.x - at.x)).toBe(0);
  expect((await picker.boundingBox())!.y).toBeGreaterThan(at.y + at.height);
  await page.keyboard.press("2");
  await expect(row.getByRole("button", { name: "open", exact: true })).toBeVisible();

  await row.getByRole("button", { name: "Add", exact: true }).dispatchEvent("mousedown");
  await page.getByRole("menu", { name: "Add to note" }).getByRole("menuitem", { name: /^Tags…/ }).click();
  const tags = page.getByRole("dialog", { name: "Tags for Popup test" });
  await tags.getByRole("textbox").fill("demo");
  await page.keyboard.press("Enter");
  await page.keyboard.press("Escape");
  await expect(row.getByRole("button", { name: "#demo", exact: true })).toBeVisible();
});
