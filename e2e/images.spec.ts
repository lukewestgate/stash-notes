import { expect, test, type Page } from "@playwright/test";

const png = Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=", "base64");

async function setDocument(page: Page, text: string) {
  await page.evaluate((text) => {
    const view = (window as any).__view();
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: { anchor: text.length } });
  }, text);
}

async function insertFile(page: Page, name = "Screenshot.png", buffer = png) {
  const chooser = page.waitForEvent("filechooser");
  if (page.url().includes("pin=")) await page.getByRole("button", { name: "Insert image", exact: true }).click();
  else {
    // The main window has it in the note menu.
    await page.getByRole("button", { name: "Note menu", exact: true }).click();
    await page.getByRole("menuitem", { name: "Insert image…", exact: true }).click();
  }
  await (await chooser).setFiles({ name, mimeType: "image/png", buffer });
}

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await page.waitForFunction(() => (window as any).__view?.().state.doc.length > 0);
});

async function delayImports(page: Page) {
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const original = api.importImage;
    api.importImage = async (path: string, bytes: number[]) => {
      const source = await original(path, bytes);
      await new Promise<void>((resolve) => { (window as any).__finishImage = resolve; });
      return source;
    };
  });
}

test("inserts, previews, edits and undoes an image without storing binary in Markdown", async ({ page }) => {
  await setDocument(page, "# Images\n\n");
  await insertFile(page);
  const image = page.getByRole("img", { name: "Screenshot", exact: true });
  await expect(image).toBeVisible();
  expect(await image.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1);
  const text = await page.evaluate(() => (window as any).__view().state.doc.toString());
  expect(text).toMatch(/!\[Screenshot\]\(\/assets\/image-.*\.png\)/);
  expect(text).not.toContain("base64");
  await image.click();
  await expect(image).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("![Screenshot]");
  await page.keyboard.press("Control+End");
  await expect(image).toBeVisible();
  await page.keyboard.press("Control+z");
  await expect(image).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => (window as any).__view().state.doc.toString())).toBe("# Images\n\n");
  await page.keyboard.press("Control+y");
  await expect(image).toBeVisible();
});

test("pastes clipboard images before smart-pasting accompanying text", async ({ page }) => {
  await setDocument(page, "# Paste\n\n");
  await page.locator(".cm-content").evaluate((element, bytes) => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array(bytes)], "Clipboard.png", { type: "image/png" }));
    data.setData("text/plain", "clipboard text should not replace the image");
    element.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  }, [...png]);
  await expect(page.getByRole("img", { name: "Clipboard" })).toBeVisible();
  expect(await page.evaluate(() => (window as any).__view().state.doc.toString())).not.toContain("clipboard text");
});

test("imports dropped image files at the drop position", async ({ page }) => {
  await setDocument(page, "# Drop\n\n");
  await page.locator(".cm-content").evaluate((element, bytes) => {
    const data = new DataTransfer();
    data.items.add(new File([new Uint8Array(bytes)], "Dropped.png", { type: "image/png" }));
    element.dispatchEvent(new DragEvent("drop", { dataTransfer: data, bubbles: true, cancelable: true }));
  }, [...png]);
  await expect(page.getByRole("img", { name: "Dropped" })).toBeVisible();
});

test("shows missing image errors and leaves code examples literal", async ({ page }) => {
  await setDocument(page, "`![literal](/assets/a.png)`\n\n```\n![code](/assets/b.png)\n```\n\n![Missing](/assets/missing.png)\n\n");
  await expect(page.locator(".cm-image")).toHaveCount(1);
  await expect(page.getByRole("status").filter({ hasText: "Image unavailable: Missing" })).toBeVisible();
  await page.locator(".cm-image").click();
  await expect(page.locator(".cm-content")).toContainText("![Missing](/assets/missing.png)");
});

test("rejects unsupported files without changing the note", async ({ page }) => {
  await setDocument(page, "# No image\n\n");
  await insertFile(page, "fake.png", Buffer.from("This is not an image"));
  await expect(page.getByText(/Unsupported image/)).toBeVisible();
  expect(await page.evaluate(() => (window as any).__view().state.doc.toString())).toBe("# No image\n\n");
});

test("renders images inside expanded note previews", async ({ page }) => {
  await page.evaluate(async (bytes) => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    const path = await api.createNote("Image child", "personal");
    const source = await api.importImage(path, bytes);
    const opened = await api.bufferOpen(path);
    // @ts-expect-error Vite's CodeMirror module.
    const { ChangeSet } = await import("/node_modules/.vite/deps/@codemirror_state.js");
    const changes = ChangeSet.of({ from: 0, to: opened.text.length, insert: `# Image child\n\n![Nested](${source})` }, opened.text.length);
    await api.bufferPush(path, opened.version, [{ changes: changes.toJSON(), clientID: opened.clientID }]);
  }, [...png]);
  await setDocument(page, "[[personal:Image child]]\n\n");
  const toggle = page.getByRole("button", { name: "Expand personal:Image child" });
  await toggle.focus();
  await toggle.press("Enter");
  const preview = page.getByRole("region", { name: "Preview of personal:Image child" });
  const image = preview.getByRole("img", { name: "Nested" });
  await expect(image).toBeVisible();
  await image.click();
  await expect(image).toBeVisible();
});

test("keeps an import at its insertion point while the user continues typing", async ({ page }) => {
  await setDocument(page, "# Async\n\n");
  await delayImports(page);
  await insertFile(page);
  await page.waitForFunction(() => (window as any).__finishImage);
  await page.evaluate(() => (window as any).__view().focus());
  await page.keyboard.insertText("Typed while loading\n");
  await page.evaluate(() => (window as any).__finishImage());
  await expect(page.getByRole("img", { name: "Screenshot" })).toBeVisible();
  const text = await page.evaluate(() => (window as any).__view().state.doc.toString());
  expect(text).toMatch(/Typed while loading\n!\[Screenshot\]/);
});

test("does not insert an outstanding import into a different note", async ({ page }) => {
  await setDocument(page, "# First\n\n");
  await delayImports(page);
  await insertFile(page);
  await page.waitForFunction(() => (window as any).__finishImage);
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api, mockEmit } = await import("/src/lib/api.ts");
    const path = await api.createNote("Other image note", "personal");
    mockEmit("open-note", path);
  });
  await expect.poll(() => page.evaluate(() => (window as any).__view().state.doc.toString())).toContain("# Other image note");
  await page.evaluate(() => (window as any).__finishImage());
  await expect(page.locator(".cm-image")).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__view().state.doc.toString())).not.toContain("![Screenshot]");
});

test("inserts and previews an image in a pinned note", async ({ page }) => {
  await page.goto("/?pin=work:notes/PROJ-123.md");
  await expect(page.getByRole("button", { name: "Insert image" })).toBeEnabled();
  await insertFile(page);
  await expect(page.getByRole("img", { name: "Screenshot" })).toBeVisible();
});

test("renders web and data images and follows the live formatting setting", async ({ page }) => {
  await page.route("https://images.example.test/screen.png", (route) => route.fulfill({ contentType: "image/png", body: png }));
  await setDocument(page, `![Web](https://images.example.test/screen.png)\n\n![Data](data:image/png;base64,${png.toString("base64")})\n\n`);
  await expect(page.getByRole("img", { name: "Web", exact: true })).toBeVisible();
  await expect(page.getByRole("img", { name: "Data", exact: true })).toBeVisible();
  await page.evaluate(async () => {
    // @ts-expect-error Vite development module.
    const { api } = await import("/src/lib/api.ts");
    await api.setSettings({ ...await api.getSettings(), liveFormatting: false });
  });
  await expect(page.locator(".cm-image")).toHaveCount(0);
  await expect(page.locator(".cm-content")).toContainText("![Web](https://images.example.test/screen.png)");
});
