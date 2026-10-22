import { describe, expect, it } from "vitest";
import { previewText } from "./live";

describe("embedded note text", () => {
  it.each(["\n", "\r\n"])("hides YAML without truncating a %j note", (newline) => {
    const text = ["---", "status: open", "tags: [review]", "---", "", "# Linked page", "", "    indented code"].join(newline);
    expect(previewText(text)).toBe("# Linked page\n\n    indented code");
  });

  it("preserves body indentation and markdown rules", () => {
    expect(previewText("---\nstatus: open\n...\n\n    code\n\n---\n")).toBe("    code\n\n---\n");
    expect(previewText("# Note\r\n\r\n---\r\n")).toBe("# Note\n\n---\n");
  });

  it("preserves an unclosed front matter fence", () => {
    expect(previewText("---\nstatus: open\nStill writing")).toBe("---\nstatus: open\nStill writing");
  });
});
