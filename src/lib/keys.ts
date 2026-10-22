// Shortcut strings like "Ctrl+Shift+G" as written in settings.json.

export interface KeySpec {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
  key: string; // normalized, lower case
}

const ALIASES: Record<string, string> = {
  escape: "esc",
  return: "enter",
  arrowleft: "left",
  arrowright: "right",
  arrowup: "up",
  arrowdown: "down",
  " ": "space",
  spacebar: "space",
  del: "delete",
};

function normalizeKey(k: string): string {
  const l = k.toLowerCase();
  return ALIASES[l] ?? l;
}

export function parseKey(spec: string): KeySpec | null {
  if (!spec) return null;
  // Split on "+" but allow a literal "+" key ("Ctrl++").
  const parts = spec.split(/\+(?!$)/).map((p) => p.trim());
  const out: KeySpec = { ctrl: false, alt: false, shift: false, meta: false, key: "" };
  for (const p of parts) {
    const l = p.toLowerCase();
    if (l === "ctrl" || l === "control" || l === "cmdorctrl" || l === "commandorcontrol") out.ctrl = true;
    else if (l === "alt" || l === "option") out.alt = true;
    else if (l === "shift") out.shift = true;
    else if (l === "meta" || l === "super" || l === "win" || l === "cmd") out.meta = true;
    else out.key = normalizeKey(p);
  }
  return out.key ? out : null;
}

const CODE_KEYS: Record<string, string> = {
  Semicolon: ";",
  Slash: "/",
  Comma: ",",
  Period: ".",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Minus: "-",
  Equal: "=",
  Backquote: "`",
};

/** The key from `KeyboardEvent.code`, layout independent ("KeyG" -> "g"). */
function codeKey(code: string): string | null {
  if (code.startsWith("Key") && code.length === 4) return code.slice(3).toLowerCase();
  if (code.startsWith("Digit") && code.length === 6) return code.slice(5);
  return CODE_KEYS[code] ?? null;
}

export function matches(e: KeyboardEvent, spec: KeySpec | null): boolean {
  if (!spec) return false;
  if (e.ctrlKey !== spec.ctrl || e.altKey !== spec.alt || e.metaKey !== spec.meta) return false;
  if (e.shiftKey !== spec.shift) return false;
  const key = normalizeKey(e.key);
  return key === spec.key || codeKey(e.code) === spec.key;
}

/** Display form: "Ctrl+Shift+G". */
export function formatKey(spec: string): string {
  const k = parseKey(spec);
  if (!k) return spec;
  const name =
    k.key.length === 1 ? k.key.toUpperCase() : k.key === "esc" ? "Esc" : k.key[0].toUpperCase() + k.key.slice(1);
  return [k.ctrl && "Ctrl", k.alt && "Alt", k.shift && "Shift", k.meta && "Win", name].filter(Boolean).join("+");
}

/** A chord like "Ctrl+E P": the keys pressed one after the other. */
export function parseChord(spec: string): KeySpec[] {
  return spec
    .trim()
    .split(/\s+/)
    .map(parseKey)
    .filter((k): k is KeySpec => k !== null);
}

/** Does `e` match the chord's second key? Holding the first key's modifiers is fine too ("Ctrl+P"). */
export function matchesChordTail(e: KeyboardEvent, chord: KeySpec[]): boolean {
  const [first, second] = chord;
  if (!second) return false;
  return matches(e, second) || matches(e, { ...second, ctrl: second.ctrl || first.ctrl, alt: second.alt || first.alt });
}

/** Set while a shortcut recorder is listening, so app shortcuts don't fire. */
export const keyCapture = { active: false };

const NAMED: Record<string, string> = {
  " ": "Space",
  Escape: "Esc",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
};

/** The shortcut string for a key press ("Ctrl+Shift+G"), or null for a lone modifier. */
export function specFromEvent(e: KeyboardEvent): string | null {
  if (["Control", "Shift", "Alt", "Meta", "OS"].includes(e.key)) return null;
  let key = NAMED[e.key] ?? codeKey(e.code) ?? e.key;
  if (key.length === 1) key = key.toUpperCase();
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
  return [...mods, key].join("+");
}

/** Do two shortcut strings mean the same keys? */
export function sameKeys(a: string, b: string): boolean {
  const x = parseKey(a);
  const y = parseKey(b);
  return !!x && !!y && x.ctrl === y.ctrl && x.alt === y.alt && x.shift === y.shift && x.meta === y.meta && x.key === y.key;
}
