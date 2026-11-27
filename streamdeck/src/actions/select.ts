import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { keyWithIcon } from "../keyimg";
import { selection } from "../selection";
import { FaderustAction } from "./base";

/** 1x1 black PNG: Stream Deck stretches it over the key. */
const BLANK = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

type Settings = {
  /** "s<input>", "m<output>" or "clear". */
  target?: string;
};

/** Picks a strip or master for the "Lauter / Leiser" keys; lit while picked. "clear" empties the selection. */
@action({ UUID: "com.niklas970.faderust.select" })
export class Select extends FaderustAction<Settings> {
  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    if (!action.isKey()) return;
    const state = faderust.state;
    const t = s.target ?? "";
    let name = "wählen";
    if (t === "clear") name = "Auswahl\nleeren";
    else if (t[0] === "s") name = state?.inputs[Number(t.slice(1))] ?? "?";
    else if (t[0] === "m") name = state?.outputs[Number(t.slice(1))] ?? "?";
    // A key whose strip or master doesn't exist (Faderust shows fewer) stays dark and does nothing.
    const missing = !!state && ((t[0] === "s" && Number(t.slice(1)) >= state.inputs.length) || (t[0] === "m" && Number(t.slice(1)) >= state.outputs.length));
    if (missing) {
      await this.show(action, { missing: true }, async () => {
        await action.setImage(BLANK, { state: 0 });
        await action.setImage(BLANK, { state: 1 });
        await action.setTitle("");
        await action.setState(0);
      });
      return;
    }
    const on = t !== "clear" && selection.has(t);
    const icon = t[0] === "s" ? faderust.iconOf(Number(t.slice(1))) : undefined;
    const view = { name, on, icon: icon?.length ?? 0 };
    await this.show(action, view, async () => {
      // The manifest image is the strip's; masters and "clear" get their own.
      if (t[0] === "s") {
        // A program's own icon replaces the Faderust logo in the corner.
        await action.setImage(icon ? keyWithIcon("actions/select/plain", icon) : undefined, { state: 0 });
        await action.setImage(icon ? keyWithIcon("actions/select/on-plain", icon) : undefined, { state: 1 });
      } else if (t === "clear") await action.setImage("imgs/actions/select/clear");
      else if (t[0] === "m") {
        await action.setImage("imgs/actions/select/master", { state: 0 });
        await action.setImage("imgs/actions/select/master-on", { state: 1 });
      }
      await action.setTitle(name);
      await action.setState(on ? 1 : 0);
    });
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const t = this.settingsOf(ev.action).target ?? "";
    const n = faderust.state;
    if (n && ((t[0] === "s" && Number(t.slice(1)) >= n.inputs.length) || (t[0] === "m" && Number(t.slice(1)) >= n.outputs.length))) return;
    if (t === "clear") selection.clear();
    else if (t[0] === "s" || t[0] === "m") selection.toggle(t);
    else return ev.action.showAlert();
    for (const a of this.actions) await this.draw(a, this.settingsOf(a));
  }
}
