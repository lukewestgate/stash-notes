import { mount } from "svelte";
import "./app.css";
import { inTauri } from "./lib/api";

/** Which window this page is: the main window, a pinned note or the capture box. */
async function windowKind(): Promise<"main" | "pin" | "capture"> {
  if (inTauri) {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const label = getCurrentWindow().label;
    return label.startsWith("pin-") ? "pin" : label === "capture" ? "capture" : "main";
  }
  const q = new URLSearchParams(location.search);
  return q.has("pin") ? "pin" : q.has("capture") ? "capture" : "main";
}

const target = document.getElementById("app")!;
const kind = await windowKind();
document.documentElement.dataset.window = kind;
if (kind === "pin") mount((await import("./PinnedApp.svelte")).default, { target });
else if (kind === "capture") mount((await import("./CaptureApp.svelte")).default, { target });
else mount((await import("./App.svelte")).default, { target });
