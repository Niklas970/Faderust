import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { decodePng, encodePng, overlayIcon, type Pixels } from "./compose";

/** The images folder: next to `bin/` when running, in the plugin folder when run from `src/`. */
function imgs(): string {
  for (const rel of ["../imgs/", "../com.niklas970.faderust.sdPlugin/imgs/"]) {
    const dir = fileURLToPath(new URL(rel, import.meta.url));
    if (existsSync(dir)) return dir;
  }
  return "";
}

const bases = new Map<string, Pixels>();
const made = new Map<string, string>();

/**
 * A key image (`base`, a path below imgs/ without `.png`) with the program
 * icon (`iconPng`, base64) in its top-left corner, as a data URL.
 */
export function keyWithIcon(base: string, iconPng: string): string | undefined {
  const key = `${base}|${iconPng}`;
  const known = made.get(key);
  if (known) return known;
  try {
    let pixels = bases.get(base);
    if (!pixels) {
      pixels = decodePng(readFileSync(`${imgs()}${base}@2x.png`));
      bases.set(base, pixels);
    }
    const icon = decodePng(Buffer.from(iconPng, "base64"));
    // Top-left of the 72 px key, in the 144 px image.
    const png = encodePng(overlayIcon(pixels, icon, 8, 8, 36, 8));
    const url = `data:image/png;base64,${Buffer.from(png).toString("base64")}`;
    if (made.size > 200) made.clear();
    made.set(key, url);
    return url;
  } catch {
    return undefined;
  }
}
