// Which characters of a search result to highlight. The ranking itself is
// done in Rust; this only has to explain a result to the eye, so it's simple:
// each query word marks a substring if it has one, otherwise the first
// characters that spell it in order.

export interface Part {
  text: string;
  hit: boolean;
}

export function highlight(text: string, query: string): Part[] {
  const marks = new Array<boolean>(text.length).fill(false);
  const lower = text.toLowerCase();
  const words = query
    .toLowerCase()
    .split(/\s+/)
    .filter((w) => w && !w.startsWith("@") && !w.startsWith("#") && !w.startsWith("in:"));
  for (const word of words) {
    const at = lower.indexOf(word);
    if (at >= 0) {
      marks.fill(true, at, at + word.length);
      continue;
    }
    // In-order letters; separators like `-` in keys don't need to be typed.
    const chars = [...word].filter((c) => /[\p{L}\p{N}]/u.test(c));
    const found: number[] = [];
    let from = 0;
    for (const c of chars) {
      const i = lower.indexOf(c, from);
      if (i < 0) break;
      found.push(i);
      from = i + 1;
    }
    if (chars.length && found.length === chars.length) for (const i of found) marks[i] = true;
  }
  const parts: Part[] = [];
  for (let i = 0; i < text.length; i++) {
    const last = parts[parts.length - 1];
    if (last && last.hit === marks[i]) last.text += text[i];
    else parts.push({ text: text[i], hit: marks[i] });
  }
  return parts;
}
