import { mount } from "svelte";
import "./app.css";
import { inTauri } from "./lib/api";

/** Which window this page is: the main window, a pinned note or the Alt+J pop-up. */
async function windowKind(): Promise<"main" | "pin" | "quick"> {
  if (inTauri) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const label = getCurrentWindow().label;
    return label.startsWith("pin-") ? "pin" : label === "quick" ? "quick" : "main";
  }
  const q = new URLSearchParams(location.search);
  return q.has("pin") ? "pin" : q.has("quick") ? "quick" : "main";
}

const target = document.getElementById("app")!;
const kind = await windowKind();
document.documentElement.dataset.window = kind;
if (kind === "pin") mount((await import("./PinnedApp.svelte")).default, { target });
else if (kind === "quick") mount((await import("./QuickApp.svelte")).default, { target });
else mount((await import("./App.svelte")).default, { target });
