import assert from "node:assert/strict";
import { test } from "node:test";

import { decodePng, encodePng, overlayIcon } from "../src/compose.ts";

const solid = (width: number, height: number, [r, g, b, a]: number[]) => ({
  width,
  height,
  rgba: Uint8Array.from({ length: width * height * 4 }, (_, i) => [r, g, b, a][i % 4]!),
});

test("a PNG survives encoding and decoding", () => {
  const image = solid(5, 3, [10, 20, 30, 255]);
  image.rgba[4] = 200;
  const back = decodePng(encodePng(image));
  assert.equal(back.width, 5);
  assert.equal(back.height, 3);
  assert.deepEqual([...back.rgba], [...image.rgba]);
});

test("the icon covers its corner and leaves the rest of the key", () => {
  const key = solid(20, 20, [0, 0, 0, 255]);
  const icon = solid(4, 4, [255, 0, 0, 255]);
  const out = overlayIcon(key, icon, 2, 2, 10, 2);
  const px = (x: number, y: number) => [...out.rgba.subarray((y * 20 + x) * 4, (y * 20 + x) * 4 + 4)];
  assert.deepEqual(px(7, 7), [255, 0, 0, 255]);
  assert.deepEqual(px(15, 15), [0, 0, 0, 255]);
  // The rounded corner stays the key's colour.
  assert.deepEqual(px(2, 2), [0, 0, 0, 255]);
});

test("a transparent icon shows the dark backing, not the old logo", () => {
  const key = solid(20, 20, [255, 255, 255, 255]);
  const out = overlayIcon(key, solid(4, 4, [255, 0, 0, 0]), 2, 2, 10, 2);
  assert.deepEqual([...out.rgba.subarray((7 * 20 + 7) * 4, (7 * 20 + 7) * 4 + 4)], [14, 16, 18, 255]);
});
