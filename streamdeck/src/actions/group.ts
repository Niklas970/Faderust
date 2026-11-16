import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { selection } from "../selection";
import { FaderustAction } from "./base";

type Settings = {
  /** Inputs whose strip moves (all outputs the input plays on). */
  strips?: number[];
  /** Outputs whose master fader moves. */
  masters?: number[];
  /** Move the faders the "Auswählen" keys picked instead of the lists above. */
  selection?: boolean;
  /** dB per press; negative is quieter. */
  step?: number;
};

const STEP = 3;

/** One key that makes several strips and masters louder or quieter together. */
@action({ UUID: "com.niklas970.faderust.group" })
export class Group extends FaderustAction<Settings> {
  constructor() {
    super();
    selection.onChange(() => {
      for (const a of this.actions) void this.draw(a, this.settingsOf(a));
    });
  }

  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    if (!action.isKey()) return;
    const step = s.step ?? STEP;
    const count = s.selection ? selection.size : (s.strips?.length ?? 0) + (s.masters?.length ?? 0);
    const title = `${step < 0 ? "Leiser" : "Lauter"}\n${s.selection ? `Auswahl: ${count}` : count ? `${count} Fader` : "wählen"}`;
    await this.show(action, title, () => action.setTitle(title));
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const delta = s.step ?? STEP;
    const state = faderust.state;
    // Skip faders that no longer exist, e.g. a profile made for more outputs.
    const chosen = s.selection ? selection.split() : { strips: s.strips ?? [], masters: s.masters ?? [] };
    const strips = chosen.strips.filter((i) => i < (state?.inputs.length ?? 0));
    const masters = chosen.masters.filter((o) => o < (state?.outputs.length ?? 0));
    if (!faderust.connected || strips.length + masters.length === 0) return this.offline(ev.action);
    for (const input of strips) faderust.send({ type: "addStripGain", input, delta });
    for (const output of masters) faderust.send({ type: "addMasterGain", output, delta });
  }
}
