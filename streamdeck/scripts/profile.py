#!/usr/bin/env python3
"""Builds com.niklas970.faderust.sdPlugin/Faderust.streamDeckProfile (page for a 15-key Stream Deck).

Layout (5 x 3):  Zurück | Leiser | Lauter | Auswahl stumm | Alles stumm   (Leiser/Lauter/Stumm wirken auf die Auswahl)
                 Streifen 1 … 5 (auswählen)
                 Master 1, 2 (auswählen) | Auswahl leeren
Run: python3 scripts/profile.py

The layout copies what Stream Deck 7.6 itself exports (Profiles > Export): everything lives under
Profiles/<UUID>.sdProfile/, with one folder per page below it, and the profile's default page is an
extra empty page next to the page list. Without that layout Stream Deck skips the profile on import.
"""
import json
import os
import uuid
import zipfile

ROOT = os.path.join(os.path.dirname(__file__), "..")
OUT = os.path.join(ROOT, "com.niklas970.faderust.sdPlugin", "Faderust.streamDeckProfile")
PLUGIN = {"Name": "Faderust", "UUID": "com.niklas970.faderust", "Version": "0.3.6.0"}
MODEL = "20GBL9901"  # model code of the exported profile this layout was copied from
NS = uuid.UUID("6f1d3c52-7a0e-4b57-9d2a-0f4d1e9c3a11")  # fixed, so the file is reproducible


def key(name, action, settings=None, states=1):
    return {
        "ActionID": str(uuid.uuid5(NS, f"{name}-{action}")),
        "LinkedTitle": True,
        "Name": name,
        "Plugin": PLUGIN,
        "Resources": None,
        "Settings": settings or {},
        "State": 0,
        "States": [{"FontFamily": "", "FontSize": 11, "FontStyle": "", "FontUnderline": False, "OutlineThickness": 2, "ShowTitle": True, "TitleAlignment": "bottom", "TitleColor": "#ffffff"} for _ in range(states)],
        "UUID": f"com.niklas970.faderust.{action}",
    }


actions = {"0,0": key("Zurück", "back"), "4,0": key("Alles stumm", "panic", states=2)}
for i in range(5):
    actions[f"{i},1"] = key(f"Streifen {i + 1}", "select", {"target": f"s{i}"}, 2)
for i in range(2):
    actions[f"{i},2"] = key(f"Master {i + 1}", "select", {"target": f"m{i}"}, 2)
actions["2,2"] = key("Auswahl leeren", "select", {"target": "clear"}, 2)

actions["1,0"] = key("Leiser", "group", {"selection": True, "step": -3})
actions["2,0"] = key("Lauter", "group", {"selection": True, "step": 3})
actions["3,0"] = key("Auswahl stumm", "selmute", states=2)

page = str(uuid.uuid5(NS, "page"))
default_page = str(uuid.uuid5(NS, "default-page"))
folder = f"Profiles/{str(uuid.uuid5(NS, 'profile')).upper()}.sdProfile"
root = {
    "Device": {"Model": MODEL, "UUID": ""},
    "Name": "Faderust",
    "Pages": {"Current": "00000000-0000-0000-0000-000000000000", "Default": default_page, "Pages": [page]},
    "Version": "3.0",
}
package = {
    "AppVersion": "7.6.1.23033",
    "DeviceModel": MODEL,
    "DeviceSettings": None,
    "FormatVersion": 1,
    "OSType": "Windows",
    "OSVersion": "10.0.26200",
    "RequiredPlugins": ["com.niklas970.faderust"],
}
pagefile = {"Controllers": [{"Type": "Keypad", "Actions": actions}], "Icon": "", "Name": ""}
emptypage = {"Controllers": [{"Type": "Keypad", "Actions": None}], "Icon": "", "Name": ""}

with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
    for name, data in (
        ("package.json", package),
        (f"{folder}/manifest.json", root),
        (f"{folder}/Profiles/{page.upper()}/manifest.json", pagefile),
        (f"{folder}/Profiles/{default_page.upper()}/manifest.json", emptypage),
    ):
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        z.writestr(info, json.dumps(data, indent=2, ensure_ascii=False))
print("wrote", os.path.relpath(OUT))
