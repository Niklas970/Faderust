import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { formatDb, stripGain } from "../faderust";
import { FaderustAction } from "./base";

type Settings = {
  /** -1: the master of `output`. */
  input?: number;
  /** -1: the whole strip of `input`. */
  output?: number;
  /** The level the key sets, in dB. */
  db?: number;
};

const NONE = -1;

/** A key that sets a fader to a fixed level (strip, one output or a master). */
@action({ UUID: "com.niklas970.faderust.level" })
export class Level extends FaderustAction<Settings> {
  protected override async draw(action: Action<Settings>, s: Settings): Promise<void> {
    if (!action.isKey()) return;
    const state = faderust.state;
    const input = s.input ?? 0;
    const output = s.output ?? 0;
    let name = "Faderust";
    let now = "–";
    if (state) {
      if (input === NONE && output >= 0) {
        name = state.outputs[output] ?? "?";
        now = formatDb(state.masters[output]);
      } else if (input >= 0 && output === NONE) {
        name = state.inputs[input] ?? "?";
        now = formatDb(stripGain(state.gains[input]));
      } else if (input >= 0 && output >= 0) {
        name = `${state.inputs[input] ?? "?"} → ${state.outputs[output] ?? "?"}`;
        now = formatDb(state.gains[input]?.[output]);
      }
    }
    const title = `${name}\n${now}`;
    await this.show(action, title, () => action.setTitle(title));
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const s = this.settingsOf(ev.action);
    const state = faderust.state;
    const input = s.input ?? 0;
    const output = s.output ?? 0;
    const db = s.db ?? 0;
    if (!state || !faderust.connected) return this.offline(ev.action);
    if (input === NONE && output >= 0) {
      faderust.send({ type: "setMasterGain", output, db });
    } else if (input >= 0 && output === NONE) {
      // The outputs the strip plays on; all of them while none is.
      const row = state.gains[input] ?? [];
      const on = row.map((g, o) => (g.muted ? -1 : o)).filter((o) => o >= 0);
      const outputs = on.length ? on : row.map((_, o) => o);
      for (const o of outputs) faderust.send({ type: "setGain", input, output: o, db });
    } else if (input >= 0 && output >= 0) {
      faderust.send({ type: "setGain", input, output, db });
    } else {
      await this.offline(ev.action);
    }
  }
}
