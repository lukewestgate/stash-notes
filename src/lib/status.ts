// Looking up how a status is shown.

import type { StatusDef } from "./api";

export interface StatusInfo {
  name: string;
  icon: string;
  color: string;
  active: boolean;
  known: boolean;
}

export const CLOSED = "closed";

/** Display info for a status; unknown values show as a grey question mark. */
export function statusInfo(name: string, defs: StatusDef[]): StatusInfo {
  const d = defs.find((x) => x.name === name);
  return d ? { ...d, known: true } : { name, icon: "circle-question-mark", color: "gray", active: false, known: false };
}

export const COLORS = ["blue", "green", "red", "orange", "purple", "teal", "pink", "gray"] as const;
