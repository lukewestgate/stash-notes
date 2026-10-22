import { describe, expect, it } from "vitest";
import { api } from "./api";

describe("note previews in the browser backend", () => {
  it("reads across vaults without switching the current vault", async () => {
    const path = await api.createNote("Preview test page", "personal");
    await api.switchVault("work");
    const preview = await api.notePreview("personal:Preview test page", "work:notes/source.md");
    expect(preview).toMatchObject({ path, text: "# Preview test page\n\n" });
    expect((await api.startup()).currentVault).toBe("work");
  });

  it("does not create missing notes", async () => {
    await expect(api.notePreview("Missing preview test page", "personal:notes/source.md")).rejects.toThrow("Linked note not found");
    const results = await api.search("Missing preview test page", true, null);
    expect(results.exact).toBe(false);
  });
});
