#!/usr/bin/env python3
"""Builds com.niklas970.faderust.sdPlugin/Faderust.streamDeckProfile (page for a 15-key Stream Deck).

Layout (5 x 3):  Alles stumm | Preset | Preset | Preset | Zurück
                 Streifen 1 … 5
                 Master 1 … 4 | Standardgerät
Run: python3 scripts/profile.py
"""
import json
import os
import uuid
import zipfile

ROOT = os.path.join(os.path.dirname(__file__), "..")
OUT = os.path.join(ROOT, "com.niklas970.faderust.sdPlugin", "Faderust.streamDeckProfile")
PLUGIN = {"Name": "Faderust", "UUID": "com.niklas970.faderust", "Version": "0.3.5.0"}
NS = uuid.UUID("6f1d3c52-7a0e-4b57-9d2a-0f4d1e9c3a11")  # fixed, so the file is reproducible


def key(name, action, settings=None, states=1):
    return {
        "ActionID": str(uuid.uuid5(NS, f"{name}-{action}")),
        "LinkedTitle": True,
        "Name": name,
        "Plugin": PLUGIN,
        "Settings": settings or {},
        "State": 0,
        "States": [{} for _ in range(states)],
        "UUID": f"com.niklas970.faderust.{action}",
    }


actions = {"0,0": key("Alles stumm", "panic", states=2), "4,0": key("Zurück", "back")}
for i in range(3):
    actions[f"{i + 1},0"] = key(f"Preset {i + 1}", "preset", {}, 2)
for i in range(5):
    actions[f"{i},1"] = key(f"Streifen {i + 1}", "strip", {"input": i})
for i in range(4):
    actions[f"{i},2"] = key(f"Master {i + 1}", "master", {"output": i})

page = str(uuid.uuid5(NS, "page")).upper()
folder = f"{str(uuid.uuid5(NS, 'profile')).upper()}.sdProfile"
root = {
    "Device": {"Model": "20GAA9902", "UUID": ""},
    "Name": "Faderust",
    "Pages": {"Current": page, "Default": page, "Pages": [page]},
    "Version": "2.0",
}
pagefile = {"Controllers": [{"Type": "Keypad", "Actions": actions}], "Icon": "", "Name": ""}

with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
    for name, data in ((f"{folder}/manifest.json", root), (f"{folder}/Profiles/{page}/manifest.json", pagefile)):
        info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
        z.writestr(info, json.dumps(data, indent=2, ensure_ascii=False))
print("wrote", os.path.relpath(OUT))
