import { type Action, action, type KeyDownEvent } from "@elgato/streamdeck";

import { faderust } from "../connection";
import { FaderustAction } from "./base";

/** One key that mutes every output, and lights up while all of them are muted. */
@action({ UUID: "com.niklas970.faderust.panic" })
export class Panic extends FaderustAction<Record<string, never>> {
  #muted(): boolean | undefined {
    const masters = faderust.state?.masters;
    if (!masters?.length) return undefined;
    return masters.every((m) => m.muted);
  }

  protected override async draw(action: Action<Record<string, never>>): Promise<void> {
    if (!action.isKey()) return;
    const muted = this.#muted();
    await this.show(action, muted ?? null, () => action.setState(muted ? 1 : 0));
  }

  override async onKeyDown(ev: KeyDownEvent<Record<string, never>>): Promise<void> {
    const muted = this.#muted();
    const count = faderust.state?.masters.length ?? 0;
    if (muted === undefined || !faderust.connected) return this.offline(ev.action);
    for (let output = 0; output < count; output++) {
      faderust.send({ type: "setMasterMuted", output, muted: !muted });
    }
  }
}
