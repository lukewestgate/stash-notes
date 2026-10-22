<script lang="ts">
  import type { Keys } from "../lib/api";
  import { formatKey } from "../lib/keys";

  interface Props {
    keys: Keys;
    hotkey: string;
    settingsPath: string;
    vaultPath: string;
  }
  let { keys, hotkey, settingsPath, vaultPath }: Props = $props();

  const rows = $derived<[string, string, string?][]>([
    [hotkey, "Show or hide Stash Notes", "global, works from any app"],
    [keys.hide, "Close overlay, else hide the window"],
    [keys.quickSwitcher, "Quick switcher: find, open or create a note", "with text selected: link it"],
    [keys.dailyNote, "Today's daily note"],
    [keys.followLink, "Follow link / ticket / URL under cursor", "on a checkbox line: toggle it"],
    [keys.back, "Back"],
    [keys.forward, "Forward"],
    [keys.openJira, "Open Jira for this ticket"],
    [keys.openGerrit, "Open Gerrit change"],
    [keys.addGerrit, "Add Gerrit link"],
    [keys.timestamp, "Insert timestamp"],
    [keys.backlinks, "Expand / collapse backlinks"],
    [keys.archive, "Archive / unarchive note"],
    [keys.setStatus, "Set status", "1–5 pick directly; closed also archives"],
    [keys.help, "This help"],
  ]);
  const extra = $derived<[string, string][]>([
    ["[[", "Link picker"],
    ["Ctrl+Click", "Follow link"],
    [keys.rename, "Rename note (updates links)"],
    [keys.openSettings, "Open settings.json"],
    [keys.quit, "Quit"],
  ]);
  const inSwitcher: [string, string][] = [
    ["↑ ↓", "Move"],
    ["Enter", "Open / create"],
    ["Alt+Enter", "Peek without adding to history"],
    ["Tab", "Include archived notes"],
  ];
</script>

<div class="backdrop" role="presentation">
  <div class="help" role="dialog" aria-label="Keyboard shortcuts">
    <h2>Keyboard shortcuts</h2>
    <div class="cols">
      <table>
        <tbody>
          {#each rows as [k, what, note]}
            <tr>
              <td class="key"><kbd>{formatKey(k)}</kbd></td>
              <td>{what}{#if note}<span class="note">&nbsp;— {note}</span>{/if}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <div>
        <h3>More</h3>
        <table>
          <tbody>
            {#each extra as [k, what]}
              <tr><td class="key"><kbd>{k === "[[" ? k : formatKey(k)}</kbd></td><td>{what}</td></tr>
            {/each}
          </tbody>
        </table>
        <h3>In the switcher</h3>
        <table>
          <tbody>
            {#each inSwitcher as [k, what]}
              <tr><td class="key"><kbd>{k}</kbd></td><td>{what}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
    <p class="paths">
      Notes: <code>{vaultPath}</code><br />
      Settings: <code>{settingsPath}</code> (shortcuts are configurable there)
    </p>
    <p class="close">Press <kbd>Esc</kbd> to close</p>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: center;
    background: var(--scrim);
    z-index: 30;
  }
  .help {
    width: min(820px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow-y: auto;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 16px 20px;
  }
  h2 {
    margin: 0 0 10px;
    font-size: 16px;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-muted);
  }
  h3 + table {
    margin-bottom: 12px;
  }
  .cols {
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: 24px;
  }
  @media (max-width: 640px) {
    .cols {
      grid-template-columns: 1fr;
    }
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  td {
    padding: 3px 0;
    vertical-align: top;
  }
  .key {
    width: 1%;
    white-space: nowrap;
    padding-right: 12px;
  }
  .note {
    color: var(--fg-muted);
  }
  kbd {
    font: inherit;
    font-size: 12px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 5px;
    background: var(--bg);
  }
  .paths {
    font-size: 12px;
    color: var(--fg-muted);
    margin: 8px 0 0;
    word-break: break-all;
  }
  code {
    color: var(--fg);
  }
  .close {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--fg-muted);
  }
</style>
