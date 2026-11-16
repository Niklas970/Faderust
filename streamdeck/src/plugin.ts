import streamDeck from "@elgato/streamdeck";

import { Device } from "./actions/device";
import { Group } from "./actions/group";
import { Level } from "./actions/level";
import { Master } from "./actions/master";
import { Mute } from "./actions/mute";
import { Back, Page } from "./actions/page";
import { Panic } from "./actions/panic";
import { Preset } from "./actions/preset";
import { Select } from "./actions/select";
import { Strip } from "./actions/strip";
import { Volume } from "./actions/volume";
import { faderust } from "./connection";

faderust.on("error", (message) => streamDeck.logger.warn(`Faderust: ${message}`));

streamDeck.actions.registerAction(new Strip());
streamDeck.actions.registerAction(new Master());
streamDeck.actions.registerAction(new Volume());
streamDeck.actions.registerAction(new Mute());
streamDeck.actions.registerAction(new Preset());
streamDeck.actions.registerAction(new Device());
streamDeck.actions.registerAction(new Panic());
streamDeck.actions.registerAction(new Level());
streamDeck.actions.registerAction(new Group());
streamDeck.actions.registerAction(new Select());
streamDeck.actions.registerAction(new Page());
streamDeck.actions.registerAction(new Back());

faderust.version = streamDeck.info.plugin.version;
faderust.connect();
streamDeck.connect();
