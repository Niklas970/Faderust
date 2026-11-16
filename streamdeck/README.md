# Faderust für Stream Deck

Steuert Faderust vom Elgato Stream Deck. Braucht Stream Deck 7.1 oder neuer (Windows) und ein laufendes Faderust ab 0.5.0 (Streifen, Master und MUTE des Streifens brauchen die Schnittstelle ab Version 0.5.1).

## Aktionen

| Aktion | Stream Deck+ (Drehregler) | Taste |
|---|---|---|
| **Streifen** | Wie ein Streifen im Pult: Drehen bewegt alle eingeschalteten Ausgänge des Eingangs gemeinsam (1 dB pro Raste), Drücken oder Tippen schaltet den Eingang stumm (MUTE, A1–A3 bleiben). | Ein Schritt pro Druck (3 dB, negativ für leiser) |
| **Master** | Drehen ändert den Master-Fader eines Ausgangs, Drücken schaltet ihn stumm. | Ein Schritt pro Druck, Titel zeigt den Pegel |
| **Fader** (ein Eingang auf einem Ausgang, Matrix) | Drehen ändert den Pegel (1 dB pro Raste), Drücken oder Tippen schaltet stumm. Der Streifen zeigt Name, Pegel und Faderstellung. | Ein Schritt pro Druck (3 dB, negativ für leiser), Titel zeigt den Pegel |
| **Stumm** | – | Eingang auf einem Ausgang, auf allen Ausgängen einzeln oder als „MUTE des Streifens“ stumm; Taste zeigt den Zustand |
| **Preset laden** | – | Lädt ein Preset aus Faderust; leuchtet, solange die Fader dazu passen |
| **Standardgerät** | – | Macht ein Gerät zum Windows-Standard (oder nur für Anrufe); leuchtet, solange es das ist |
| **Alles stumm** | – | Schaltet alle Ausgänge stumm oder wieder ein; leuchtet, solange alle stumm sind |
| **Pegel setzen** | – | Setzt einen Streifen, einen Fader oder einen Master auf einen festen Pegel |
| **Lauter / Leiser** | – | Eine Taste bewegt mehrere angehakte Streifen und Master gemeinsam; positiver Schritt lauter, negativer leiser (Standard 3 dB) |
| **Auswählen** | – | Wählt einen Streifen oder Master aus (leuchtet, erneut drücken hebt auf); „Auswahl leeren“ hebt alle auf. „Lauter / Leiser“ mit „Auswahl verwenden“ bewegt genau diese Fader |
| **Faderust-Seite** / **Zurück** | – | Öffnet eine Seite (Profil „Faderust“) für das Stream Deck mit 15 Tasten: Zurück oben links, drei Presets, Alles stumm, fünf Streifen und zwei Master zum Auswählen, Auswahl leeren sowie Leiser und Lauter für die Auswahl. Die Presets wählt man in der Seite aus. Beim Installieren fragt die Stream-Deck-Software, ob das Profil hinzugefügt werden soll |

Eingänge, Ausgänge, Presets und Geräte wählt man im Einstellungsfenster der Aktion aus Listen, die Faderust liefert. Läuft Faderust nicht, zeigt die Taste ein Warnzeichen.

## Installieren

`com.niklas970.faderust.streamDeckPlugin` aus den [Releases](https://github.com/Niklas970/faderust/releases) doppelklicken.

## Entwickeln

```sh
cd streamdeck
npm ci
npm run build    # nach com.niklas970.faderust.sdPlugin/bin
npm test         # Plugin zwischen simuliertem Stream Deck und Faderust
npm run pack     # dist/com.niklas970.faderust.streamDeckPlugin
```

Zum Ausprobieren ohne Paket: `npx streamdeck link com.niklas970.faderust.sdPlugin`, dann `npm run watch` und nach Änderungen `npx streamdeck restart com.niklas970.faderust`.

Die Icons entstehen aus `icons/*.svg` mit `node scripts/icons.cjs` (braucht Playwright).

## Schnittstelle

Faderust lauscht auf `ws://127.0.0.1:47811` (nur lokal) und spricht JSON. Verbindungen mit `Origin`-Header (also aus dem Browser) lehnt es ab.

**Anmeldung (ab Protokoll 2):** Die erste Nachricht muss `{"type":"hello","plugin":"<Version>","token":"<Token>"}` sein. Das Token steht in `%APPDATA%\Faderust\remote.token` (legt Faderust beim ersten Start an, 64 Hex-Zeichen). Ohne gültiges `hello` innerhalb von 5 Sekunden trennt Faderust die Verbindung und schickt nichts. So können nur Programme desselben Windows-Benutzers steuern, nicht die anderer Benutzer auf demselben PC. Das Plugin liest die Datei bei jedem Verbindungsversuch.

Faderust schickt bei jeder Änderung:

- `{"type":"state","protocol":3,"inputs":[…],"outputs":[…],"gains":[[{"db":0,"muted":false}]],"inputMuted":[false],"masters":[{"db":0,"muted":false}],"presets":[…],"activePreset":null}`
- `{"type":"devices","supported":true,"playback":[…],"recording":[…]}` (Windows-Geräte mit `id`, `name`, `isDefault`, `isDefaultCommunications`)
- `{"type":"error","message":"…"}` bei einer ungültigen Anfrage

Anfragen:

| `type` | Felder |
|---|---|
| `setGain` | `input`, `output`, `db` |
| `addGain` | `input`, `output`, `delta` (dB) |
| `setMuted` | `input`, `output`, `muted` |
| `toggleMuted` | `input`, `output` |
| `addStripGain` | `input`, `delta` (dB), bewegt alle eingeschalteten Ausgänge wie der Pult-Streifen |
| `setInputMuted` | `input`, `muted` |
| `setMasterGain` | `output`, `db` |
| `addMasterGain` | `output`, `delta` (dB) |
| `setMasterMuted` | `output`, `muted` |
| `loadPreset` | `name` |
| `setDefaultDevice` | `id`, `communications` |

`input` und `output` zählen ab 0 in der Reihenfolge der Faderust-Oberfläche.
