import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { selection } from "../selection";
import { FaderustAction } from "./base";

type Settings = Record<string, never>;

/** Mutes or unmutes the strips and masters the "Auswählen" keys picked; lit while all of them are muted. */
@action({ UUID: "com.niklas970.faderust.selmute" })
export class SelectionMute extends FaderustAction<Settings> {
  constructor() {
    super();
    selection.onChange(() => {
      for (const a of this.actions) void this.draw(a);
    });
  }

  /** The picked faders that exist; undefined when nothing is picked. */
  #picked(): { strips: number[]; masters: number[] } | undefined {
    const state = faderust.state;
    if (!state) return undefined;
    const { strips, masters } = selection.split();
    const picked = {
      strips: strips.filter((i) => i < state.inputs.length),
      masters: masters.filter((o) => o < state.outputs.length),
    };
    return picked.strips.length + picked.masters.length > 0 ? picked : undefined;
  }

  #muted(): boolean | undefined {
    const picked = this.#picked();
    if (!picked) return undefined;
    const state = faderust.state;
    return (
      picked.strips.every((i) => state?.inputMuted?.[i]) && picked.masters.every((o) => state?.masters[o]?.muted)
    );
  }

  protected override async draw(action: Action<Settings>): Promise<void> {
    if (!action.isKey()) return;
    const muted = this.#muted();
    const count = selection.size;
    const view = { muted: muted ?? null, count };
    await this.show(action, view, async () => {
      await action.setTitle(`Auswahl\n${count ? `${muted ? "stumm" : "an"} (${count})` : "stumm"}`);
      await action.setState(muted ? 1 : 0);
    });
  }

  override async onKeyDown(ev: KeyDownEvent<Settings>): Promise<void> {
    const picked = this.#picked();
    const muted = this.#muted();
    if (!picked || muted === undefined || !faderust.connected) return this.offline(ev.action);
    for (const input of picked.strips) faderust.send({ type: "setInputMuted", input, muted: !muted });
    for (const output of picked.masters) faderust.send({ type: "setMasterMuted", output, muted: !muted });
  }
}
