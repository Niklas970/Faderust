import streamDeck, { action, type KeyDownEvent, SingletonAction } from "@elgato/streamdeck";

/** Name of the profile in the manifest; the Stream Deck app only switches to profiles by this name. */
export const PROFILE = "Faderust Seite";

/** Opens the Faderust page: a profile shipped with the plugin that holds the important keys. */
@action({ UUID: "com.niklas970.faderust.page" })
export class Page extends SingletonAction {
  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    try {
      await streamDeck.profiles.switchToProfile(ev.action.device.id, PROFILE, 0);
    } catch (err) {
      streamDeck.logger.warn(`Faderust page: ${err}`);
      await ev.action.showAlert();
    }
  }
}

/** Goes back to the profile that was active before the Faderust page. */
@action({ UUID: "com.niklas970.faderust.back" })
export class Back extends SingletonAction {
  override async onKeyDown(ev: KeyDownEvent): Promise<void> {
    try {
      await streamDeck.profiles.switchToProfile(ev.action.device.id);
    } catch (err) {
      streamDeck.logger.warn(`Faderust back: ${err}`);
      await ev.action.showAlert();
    }
  }
}
