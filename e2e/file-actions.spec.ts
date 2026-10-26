import { expect, test, type Page } from "@playwright/test";

async function seed(page: Page) {
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const sidebar = await api.createNote("Sidebar subject", "personal");
    await api.bufferOpen(sidebar);
    await api.bufferClose(sidebar);
    await api.createNote("Collision", "work");
    const path = await api.createNote("File actions", "personal");
    const { mockEmit } = await import("/src/lib/api.ts");
    mockEmit("open-note", path);
  });
  await expect(page.locator(".header .main")).toHaveText("File actions");
}

async function noteText(page: Page, path: string) {
  return page.evaluate(async (path) => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    return (await api.bufferOpen(path)).text;
  }, path);
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
  await seed(page);
});

test("renames from the note header and retains edits", async ({ page }) => {
  await page.evaluate(() => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: view.state.doc.length, insert: "Latest unsaved text\n" } });
  });
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Rename…/ }).click();
  const input = page.getByRole("dialog", { name: "Rename note (links to it are updated)" }).getByRole("textbox");
  await input.fill("Renamed file");
  await input.press("Enter");
  await expect(page.locator(".header .main")).toHaveText("Renamed file");
  expect(await noteText(page, "personal:notes/renamed-file.md")).toContain("Latest unsaved text");
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.locator(".header .main")).toHaveText("File actions");
});

test("moves an open note between vaults, keeps images and links, and allows further editing", async ({ page }) => {
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const source = await api.importImage("personal:notes/file-actions.md", [...Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII="), (c) => c.charCodeAt(0))]);
    const view = (window as any).__view();
    view.dispatch({ changes: { from: view.state.doc.length, insert: `![shot](${source})\n[[Sidebar subject]]\n` } });
  });
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Move to/ }).click();
  await page.getByRole("menu", { name: "Move to" }).getByRole("menuitem", { name: "Work", exact: true }).click();
  await expect(page.getByRole("menu")).toHaveCount(0);
  await expect(page.locator(".sidebar .vname")).toHaveCount(0); // rail remains collapsed
  await expect(page.getByRole("img", { name: "shot", exact: true })).toBeVisible();
  await page.evaluate(() => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: view.state.doc.length, insert: "After moving\n" } });
  });
  await expect.poll(() => noteText(page, "work:notes/file-actions.md")).toContain("After moving");
  expect(await noteText(page, "work:notes/file-actions.md")).toContain("[[personal:notes/sidebar-subject]]");
  await page.getByTitle(/Expand sidebar/).click();
  await expect(page.locator(".sidebar .vname")).toHaveText("Work");
});

test("a collision reports the error and leaves both files intact", async ({ page }) => {
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const path = await api.createNote("Work collision", "work");
    await api.renameNote(path, "File actions");
  });
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Move to/ }).click();
  await page.getByRole("menuitem", { name: "Work", exact: true }).click();
  await expect(page.locator(".toast").filter({ hasText: "already exists" })).toBeVisible();
  expect(await noteText(page, "personal:notes/file-actions.md")).toContain("# File actions");
  expect(await noteText(page, "work:notes/file-actions.md")).toContain("# File actions");
  await expect(page.locator(".header .main")).toHaveText("File actions");
});

test("move only offers other vaults", async ({ page }) => {
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Move to/ }).hover();
  const vaults = page.getByRole("menu", { name: "Move to" });
  await expect(vaults.getByRole("menuitem", { name: "Personal", exact: true })).toHaveCount(0);
  await expect(vaults.getByRole("menuitem", { name: "Work", exact: true })).toHaveCount(1);
});

test("the note menu opens at the pointer without dimming and works from the keyboard", async ({ page }) => {
  await page.getByTitle(/Expand sidebar/).click();
  const row = page.locator(".sidebar button.note").filter({ hasText: "Sidebar subject" });
  const box = (await row.boundingBox())!;
  await row.click({ button: "right", position: { x: 40, y: 10 } });
  const menu = page.getByRole("menu", { name: "Note menu for Sidebar subject" });
  const at = (await menu.boundingBox())!;
  expect(Math.abs(at.x - (box.x + 40))).toBeLessThan(10);
  expect(Math.abs(at.y - (box.y + 14))).toBeLessThan(10);
  await expect(page.locator(".popup-backdrop.anchored")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await expect(menu.getByRole("menuitem", { name: /^Open/ })).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");
  await expect(menu.getByRole("menuitem", { name: /^Status/ })).toBeFocused();
  await page.keyboard.press("ArrowRight");
  const statuses = page.getByRole("menu", { name: "Status" });
  await expect(statuses).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(statuses).toHaveCount(0);
  await expect(menu).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("3");
  await expect(menu).toHaveCount(0);
  await expect.poll(() => noteText(page, "personal:notes/sidebar-subject.md")).toContain("status: blocked");
  await expect(page.locator(".header .main")).toHaveText("File actions");
});

test("copies a link to the note", async ({ page }) => {
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Copy link/ }).click();
  await expect(page.locator(".toast").filter({ hasText: "Link copied" })).toBeVisible();
  expect(await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    return api.readClipboard();
  })).toBe("[[File actions]]");
});

test("sidebar actions target another note without changing the open note", async ({ page }) => {
  await page.getByTitle(/Expand sidebar/).click();
  const actions = page.getByRole("button", { name: "Actions for Sidebar subject", exact: true });
  await actions.click();
  await page.getByRole("menuitem", { name: /^Rename…/ }).click();
  const input = page.getByRole("dialog").getByRole("textbox");
  await input.fill("Sidebar renamed");
  await input.press("Enter");
  await expect(page.locator(".header .main")).toHaveText("File actions");
  await page.getByRole("button", { name: "Actions for Sidebar renamed", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Move to/ }).click();
  await page.getByRole("menuitem", { name: "Work", exact: true }).click();
  await expect(page.getByRole("menu")).toHaveCount(0);
  await expect(page.locator(".header .main")).toHaveText("File actions");
  await expect(page.locator(".sidebar .vname")).toHaveText("Personal");
  expect(await noteText(page, "work:notes/sidebar-renamed.md")).toContain("# Sidebar renamed");
});

test("deletes the open note only after confirmation and restores it with Undo", async ({ page }) => {
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  let dialog = page.getByRole("dialog", { name: "Delete note", exact: true });
  await expect(dialog.getByRole("button", { name: "Archive note Recommended", exact: true })).toBeFocused();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.locator(".header .main")).toHaveText("File actions");
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  dialog = page.getByRole("dialog", { name: "Delete note", exact: true });
  await expect(dialog.getByRole("textbox")).toHaveCount(0);
  await expect(dialog.getByRole("button", { name: "Delete note", exact: true })).toBeEnabled();
  await dialog.getByRole("button", { name: "Delete note", exact: true }).click();
  await expect(page.locator(".pane")).toHaveClass(/hidden/);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.locator(".header .main")).toHaveText("File actions");
});

test("sidebar right-click can delete a different note without closing the editor", async ({ page }) => {
  await page.getByTitle(/Expand sidebar/).click();
  await page.locator(".sidebar button.note").filter({ hasText: "Sidebar subject" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Delete note", exact: true }).click();
  await expect(page.getByRole("button", { name: "Actions for Sidebar subject", exact: true })).toHaveCount(0);
  await expect(page.locator(".header .main")).toHaveText("File actions");
});

test("recommends archiving, preserves edits and offers Undo", async ({ page }) => {
  await page.evaluate(() => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: view.state.doc.length, insert: "Keep this work\n" } });
  });
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  const dialog = page.getByRole("dialog", { name: "Delete note", exact: true });
  const archive = dialog.getByRole("button", { name: "Archive note Recommended", exact: true });
  await expect(archive).toBeFocused();
  await expect(dialog).toContainText("Archiving is recommended");
  await archive.press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".header .tag")).toHaveText("archived");
  // Archived where it is, marked in its front matter.
  expect(await noteText(page, "personal:notes/file-actions.md")).toContain("archived: true");
  expect(await noteText(page, "personal:notes/file-actions.md")).toContain("Keep this work");
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.locator(".header .tag")).toHaveCount(0);
  expect(await noteText(page, "personal:notes/file-actions.md")).toContain("Keep this work");
  expect(await noteText(page, "personal:notes/file-actions.md")).not.toContain("archived:");
});

test("closing a note archives it in place, and reopening unarchives it", async ({ page }) => {
  await page.keyboard.press("Control+Shift+s");
  await page.keyboard.press("5");
  await expect(page.locator(".header .tag")).toHaveText("archived");
  await expect.poll(() => noteText(page, "personal:notes/file-actions.md")).toContain("archived: true");
  expect(await noteText(page, "personal:notes/file-actions.md")).toContain("status: closed");
  await page.keyboard.press("Control+Shift+s");
  await page.keyboard.press("2");
  await expect(page.locator(".header .tag")).toHaveCount(0);
  expect(await noteText(page, "personal:notes/file-actions.md")).not.toContain("archived:");
  expect(await noteText(page, "personal:notes/file-actions.md")).toContain("status: open");
});

test("archives the sidebar target without switching the open note", async ({ page }) => {
  await page.getByTitle(/Expand sidebar/).click();
  await page.getByRole("button", { name: "Actions for Sidebar subject", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Archive note Recommended", exact: true }).click();
  await expect(page.locator(".header .main")).toHaveText("File actions");
  await expect(page.getByRole("button", { name: "Actions for Sidebar subject", exact: true })).toHaveCount(0);
  expect(await noteText(page, "personal:notes/sidebar-subject.md")).toContain("archived: true");
});

test("keeps an already archived note archived from the recommended action", async ({ page }) => {
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Archive/ }).click();
  await page.getByRole("button", { name: "Note menu", exact: true }).click();
  await page.getByRole("menuitem", { name: /^Delete…/ }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Keep archived Recommended", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".header .tag")).toHaveText("archived");
});

test("quickly discards an accidental switcher creation and can restore it", async ({ page }) => {
  await page.keyboard.press("Control+k");
  await page.getByRole("dialog", { name: "Quick switcher" }).getByRole("textbox").fill("Accidental note");
  await page.getByRole("button", { name: /Create "Accidental note"/ }).click();
  await expect(page.locator(".header .main")).toHaveText("Accidental note");
  const notice = page.locator(".toast").filter({ hasText: "Created Accidental note" });
  await notice.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator(".pane")).toHaveClass(/hidden/);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(page.locator(".header .main")).toHaveText("Accidental note");
});

test("offers the same quick delete after creating from the Notes view", async ({ page }) => {
  await page.keyboard.press("Control+l");
  await page.getByPlaceholder(/^Search notes/).fill("Wrong name");
  await page.getByRole("option", { name: /Create "Wrong name"/ }).click();
  await expect(page.locator(".header .main")).toHaveText("Wrong name");
  await page.locator(".toast").filter({ hasText: "Created Wrong name" }).getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.locator(".pane")).toHaveClass(/hidden/);
});

test("opening an existing note does not offer to discard it as a new creation", async ({ page }) => {
  await page.locator(".toast").filter({ hasText: "Created File actions" }).getByRole("button", { name: "Dismiss" }).click();
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    await api.createNote("File actions", "personal");
  });
  await expect(page.locator(".toast").filter({ hasText: "Created File actions" })).toHaveCount(0);
  await expect(page.locator(".header .main")).toHaveText("File actions");
});

test("the same note menu opens on Notes view rows and search results", async ({ page }) => {
  await page.keyboard.press("Control+l");
  await page.getByPlaceholder(/^Search notes/).fill("Sidebar subject");
  await page.getByRole("option", { name: /Sidebar subject/ }).click({ button: "right" });
  const menu = page.getByRole("menu", { name: "Note menu for Sidebar subject" });
  await expect(menu).toBeVisible();
  await menu.getByRole("menuitem", { name: /^Archive/ }).click();
  await expect.poll(() => noteText(page, "personal:notes/sidebar-subject.md")).toContain("archived: true");
  await page.keyboard.press("Escape");

  await page.keyboard.press("Control+k");
  const switcher = page.getByRole("dialog", { name: "Quick switcher", exact: true });
  await switcher.getByRole("textbox").fill("Collision");
  await expect(switcher.locator(".row").first()).toContainText("Collision");
  await page.keyboard.press("Shift+F10");
  await expect(switcher).toHaveCount(0);
  await expect(page.getByRole("menu", { name: "Note menu for Collision" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: /^Open/ })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator(".header .main")).toHaveText("Collision");
});
