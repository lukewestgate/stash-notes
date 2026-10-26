import { expect, test, type Page } from "@playwright/test";

async function setDocument(page: Page, text: string) {
  await page.evaluate((text) => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: { anchor: 0 } });
  }, text);
}

async function expandLink(page: Page, label: string) {
  const button = page.getByRole("button", { name: `Expand ${label}`, exact: true });
  await page.locator(".cm-page-reference").filter({ has: button }).hover();
  await button.click();
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
  await page.evaluate(async () => {
    // @ts-expect-error Vite serves this module directly in development.
    const { api } = await import("/src/lib/api.ts");
    for (const [title, text] of [
      ["Embedded child", "# Embedded child\n\n**Visible body**\n\n- [ ] Read only\n\n[[Embedded grandchild]]"],
      ["Embedded grandchild", "# Embedded grandchild\n\nNested body\n\n[[Embedded child]]"],
    ]) {
      const path = await api.createNote(title, "personal");
      const opened = await api.bufferOpen(path);
      // @ts-expect-error Vite serves the same CodeMirror module as the app.
      const { ChangeSet } = await import("/node_modules/.vite/deps/@codemirror_state.js");
      const changes = ChangeSet.of({ from: 0, to: opened.text.length, insert: text }, opened.text.length);
      await api.bufferPush(path, opened.version, [{ changes: changes.toJSON(), clientID: opened.clientID }]);
    }
  });
});

test("expands and collapses independent links without editing the parent", async ({ page }) => {
  const text = "# Parent\n\n- [[personal:Embedded child|Child]]\n- [[personal:Embedded child|Again]]";
  await setDocument(page, text);
  await page.mouse.move(0, 0);
  const toggle = page.getByRole("button", { name: "Expand Child", exact: true });
  const reference = page.locator(".cm-page-reference").filter({ has: toggle });
  await expect(toggle).toHaveCSS("width", "0px");
  const restingWidth = (await reference.boundingBox())!.width;
  await reference.hover();
  expect((await reference.boundingBox())!.width).toBeGreaterThan(restingWidth + 10);
  const alignment = await reference.evaluate((element) => {
    const chip = element.getBoundingClientRect();
    const icon = element.querySelector(".cm-page-toggle svg")!.getBoundingClientRect();
    return Math.abs((chip.top + chip.bottom) / 2 - (icon.top + icon.bottom) / 2);
  });
  expect(alignment).toBeLessThanOrEqual(1);
  await expandLink(page, "Child");
  const preview = page.getByRole("region", { name: "Preview of Child", exact: true });
  await expect(preview.getByText("Visible body")).toBeVisible();
  await expect(preview).toHaveCSS("border-radius", "12px");
  await expect(preview).toHaveCSS("overflow", "hidden");
  await expect(preview.locator(".cm-editor")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  const borders = await preview.evaluate((element) => {
    const style = getComputedStyle(element);
    return [style.borderTopWidth, style.borderRightWidth, style.borderBottomWidth, style.borderLeftWidth];
  });
  expect(borders.every((border) => parseFloat(border) > 0)).toBe(true);
  await expect(preview.locator(".cm-content")).toHaveAttribute("contenteditable", "false");
  await expect(preview.getByRole("button", { name: "Expand Embedded grandchild" })).toHaveCount(1);
  await expandLink(page, "Again");
  await expect(page.locator(".cm-page-preview")).toHaveCount(2);
  await page.screenshot({ path: test.info().outputPath("expanded-pages.png"), fullPage: true });
  await page.getByRole("button", { name: "Collapse Child", exact: true }).click();
  await expect(preview).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Preview of Again" })).toBeVisible();
  expect(await page.evaluate(() => (window as any).__view().state.doc.toString())).toBe(text);
});

test("keeps checkboxes read-only, refreshes changed content and opens the full note", async ({ page }) => {
  await setDocument(page, "[[personal:Embedded child|Child]]");
  const toggle = page.getByRole("button", { name: "Expand Child", exact: true });
  await toggle.focus();
  await toggle.press("Enter");
  const preview = page.getByRole("region", { name: "Preview of Child" });
  const checkbox = preview.getByRole("checkbox");
  await expect(checkbox).toHaveAttribute("aria-checked", "false");
  await checkbox.click();
  await expect(checkbox).toHaveAttribute("aria-checked", "false");
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const path = await api.resolveLink("personal:Embedded child", null);
    const opened = await api.bufferOpen(path);
    // @ts-expect-error Vite's CodeMirror module.
    const { ChangeSet } = await import("/node_modules/.vite/deps/@codemirror_state.js");
    const changes = ChangeSet.of({ from: opened.text.length, insert: "\n\nUpdated preview body" }, opened.text.length);
    await api.bufferPush(path, opened.version, [{ changes: changes.toJSON(), clientID: opened.clientID }]);
  });
  await expect(preview.getByText("Updated preview body")).toBeVisible();
  await preview.hover();
  await preview.getByRole("button", { name: "Open note", exact: true }).click();
  await expect(preview).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).__view().state.doc.toString())).toContain("Updated preview body");
});

test("allows nested expansion and stops circular references", async ({ page }) => {
  await setDocument(page, "[[personal:Embedded child|Child]]");
  await expandLink(page, "Child");
  await expandLink(page, "Embedded grandchild");
  await expect(page.getByText("Nested body")).toBeVisible();
  const outer = page.getByRole("region", { name: "Preview of Child", exact: true });
  const inner = page.getByRole("region", { name: "Preview of Embedded grandchild", exact: true });
  const outerColor = await outer.evaluate((element) => getComputedStyle(element).backgroundColor);
  const innerColor = await inner.evaluate((element) => getComputedStyle(element).backgroundColor);
  expect(outerColor).not.toBe(innerColor);
  await expect(inner).toHaveCSS("border-radius", "12px");
  await expandLink(page, "Embedded child");
  await expect(page.getByText("This note is already shown above.")).toBeVisible();
});

test("supports multiple links on one line and keyboard collapse", async ({ page }) => {
  await setDocument(page, "[[personal:Embedded child|First]] and [[personal:Embedded grandchild|Second]]");
  await expandLink(page, "First");
  await expandLink(page, "Second");
  await expect(page.getByRole("region")).toHaveCount(2);
  await expect(page.getByText("Visible body")).toBeVisible();
  await expect(page.getByText("Nested body")).toBeVisible();
  const toggle = page.getByRole("button", { name: "Collapse First", exact: true });
  await toggle.focus();
  await toggle.press("Space");
  await expect(page.getByRole("region", { name: "Preview of First", exact: true })).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Preview of Second", exact: true })).toBeVisible();
});

test("previews links in a pinned window and opens them in the main window", async ({ page }) => {
  await page.goto("/?pin=work:notes/PROJ-123.md");
  await expandLink(page, "Rollout plan");
  const preview = page.getByRole("region", { name: "Preview of Rollout plan" });
  await expect(preview.getByText("Phase one: internal users.", { exact: false })).toBeVisible();
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { on } = await import("/src/lib/api.ts");
    await on("open-note", (path: string) => { (window as any).__opened = path; });
  });
  await preview.hover();
  await preview.getByRole("button", { name: "Open note", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__opened)).toBe("work:notes/rollout-plan.md");
  await expect(preview).toBeVisible();
});

test("shows a missing note error and retries after it is created", async ({ page }) => {
  await setDocument(page, "[[personal:Uncreated child]]");
  await expandLink(page, "personal:Uncreated child");
  await expect(page.getByRole("status").filter({ hasText: "Linked note not found" })).toBeVisible();
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    await api.createNote("Uncreated child", "personal");
  });
  await page.getByRole("button", { name: "Retry" }).click();
  await expect(page.getByRole("button", { name: "Open note", exact: true })).toBeEnabled();
  await expect(page.getByText("Could not preview this note:")).toHaveCount(0);
});

test("ignores code examples and keeps expansion attached through parent edits", async ({ page }) => {
  await setDocument(page, "`[[literal]]`\n\n```\n[[literal too]]\n```\n\n[[personal:Embedded child|Child]]");
  await expect(page.locator(".cm-page-toggle")).toHaveCount(1);
  await expandLink(page, "Child");
  await expect(page.getByText("Visible body")).toBeVisible();
  await page.evaluate(() => (window as any).__view().dispatch({ changes: { from: 0, insert: "Intro\n\n" } }));
  await expect(page.getByRole("button", { name: "Collapse Child", exact: true })).toBeVisible();
  await expect(page.getByText("Visible body")).toBeVisible();
  await page.getByRole("button", { name: "Collapse Child", exact: true }).click();
  await expect(page.locator(".cm-page-preview")).toHaveCount(0);
});
