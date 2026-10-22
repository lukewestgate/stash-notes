import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { frontMatterSyntax } from "./live";
import { directImageSource, imageRefs } from "./images";

function refs(doc: string) {
  return imageRefs(EditorState.create({ doc, extensions: [markdown({ base: markdownLanguage, extensions: [frontMatterSyntax] })] }));
}

describe("image embeds", () => {
  it("reads image URLs, titles and escaped paths without treating ordinary links as images", () => {
    const images = refs('![Screenshot](/assets/screen.png "A title")\n\n![Other](<../images/a b.png>)\n\n![Third](../a\\(b\\).png)\n\n[link](https://example.com)');
    expect(images.map(({ source, alt }) => ({ source, alt }))).toEqual([
      { source: "/assets/screen.png", alt: "Screenshot" },
      { source: "../images/a b.png", alt: "Other" },
      { source: "../a(b).png", alt: "Third" },
    ]);
  });

  it("keeps inline, fenced, indented code and front matter literal", () => {
    expect(refs('---\nimage: ![YAML](/assets/a.png)\n---\n\n`![inline](/a.png)`\n\n```md\n![fenced](/b.png)\n```\n\n    ![indented](/c.png)\n\n![Real](/assets/real.png)').map((r) => r.alt)).toEqual(["Real"]);
  });

  it("preserves escaped brackets in image descriptions", () => {
    expect(refs('![Screen \\[1\\]](/assets/a.png)')[0].alt).toBe("Screen [1]");
  });

  it("only permits explicit web and raster data URLs as direct image sources", () => {
    expect(directImageSource("https://example.com/a.png")).toBe("https://example.com/a.png");
    expect(directImageSource("data:image/png;base64,YQ==")).toBe("data:image/png;base64,YQ==");
    for (const url of ["javascript:alert(1)", "file:///C:/secret.png", "data:text/html;base64,YQ==", "data:image/svg+xml;base64,YQ==", "//example.com/a.png", "/assets/a.png"]) {
      expect(directImageSource(url)).toBeNull();
    }
  });
});
