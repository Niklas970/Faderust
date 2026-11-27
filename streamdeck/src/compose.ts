/**
 * Puts a program's icon onto a key image. The Stream Deck software takes
 * PNG, so both images are decoded, mixed and encoded again here.
 */
import { deflateSync, inflateSync } from "node:zlib";

export type Pixels = { width: number; height: number; rgba: Uint8Array };

const SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of bytes) c = CRC_TABLE[(c ^ b) & 0xff]! ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/** Reads 8-bit RGB or RGBA PNGs without interlacing, which is what Faderust and the icons use. */
export function decodePng(file: Uint8Array): Pixels {
  const view = new DataView(file.buffer, file.byteOffset, file.byteLength);
  if (!SIGNATURE.every((b, i) => file[i] === b)) throw new Error("not a PNG");
  let width = 0;
  let height = 0;
  let channels = 0;
  const data: Uint8Array[] = [];
  for (let at = 8; at + 8 <= file.length; ) {
    const length = view.getUint32(at);
    const kind = String.fromCharCode(...file.subarray(at + 4, at + 8));
    const body = file.subarray(at + 8, at + 8 + length);
    if (kind === "IHDR") {
      width = view.getUint32(at + 8);
      height = view.getUint32(at + 12);
      const [depth, color, , , interlace] = body.subarray(8, 13);
      channels = color === 6 ? 4 : color === 2 ? 3 : 0;
      if (depth !== 8 || !channels || interlace !== 0) throw new Error("unsupported PNG");
    } else if (kind === "IDAT") {
      data.push(body);
    } else if (kind === "IEND") {
      break;
    }
    at += 12 + length;
  }
  const raw = inflateSync(Buffer.concat(data));
  const stride = width * channels;
  const out = new Uint8Array(width * height * 4);
  const prev = new Uint8Array(stride);
  const row = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)]!;
    const line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let i = 0; i < stride; i++) {
      const left = i >= channels ? row[i - channels]! : 0;
      const up = prev[i]!;
      const upLeft = i >= channels ? prev[i - channels]! : 0;
      let add = 0;
      if (filter === 1) add = left;
      else if (filter === 2) add = up;
      else if (filter === 3) add = (left + up) >> 1;
      else if (filter === 4) {
        const p = left + up - upLeft;
        const [pa, pb, pc] = [Math.abs(p - left), Math.abs(p - up), Math.abs(p - upLeft)];
        add = pa <= pb && pa <= pc ? left : pb <= pc ? up : upLeft;
      } else if (filter !== 0) throw new Error("bad PNG filter");
      row[i] = (line[i]! + add) & 0xff;
    }
    for (let x = 0; x < width; x++) {
      for (let c = 0; c < 3; c++) out[(y * width + x) * 4 + c] = row[x * channels + c]!;
      out[(y * width + x) * 4 + 3] = channels === 4 ? row[x * 4 + 3]! : 255;
    }
    prev.set(row);
  }
  return { width, height, rgba: out };
}

export function encodePng({ width, height, rgba }: Pixels): Uint8Array {
  const raw = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) {
    raw.set(rgba.subarray(y * width * 4, (y + 1) * width * 4), y * (width * 4 + 1) + 1);
  }
  const chunk = (kind: string, body: Uint8Array): Buffer => {
    const out = Buffer.alloc(12 + body.length);
    out.writeUInt32BE(body.length, 0);
    out.write(kind, 4, "ascii");
    out.set(body, 8);
    out.writeUInt32BE(crc32(out.subarray(4, 8 + body.length)), 8 + body.length);
    return out;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([
    Buffer.from(SIGNATURE),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", new Uint8Array()),
  ]);
}

/**
 * Draws `icon` scaled into the rounded square at (`x`, `y`) of `base`,
 * on a dark backing so transparent icons stay readable.
 */
export function overlayIcon(base: Pixels, icon: Pixels, x: number, y: number, size: number, radius: number): Pixels {
  const rgba = Uint8Array.from(base.rgba);
  const backing = [14, 16, 18];
  for (let dy = 0; dy < size; dy++) {
    for (let dx = 0; dx < size; dx++) {
      const px = x + dx;
      const py = y + dy;
      if (px < 0 || py < 0 || px >= base.width || py >= base.height) continue;
      // Rounded corners: distance to the nearest corner centre.
      const cx = dx < radius ? radius : dx >= size - radius ? size - radius - 1 : dx;
      const cy = dy < radius ? radius : dy >= size - radius ? size - radius - 1 : dy;
      if (Math.hypot(dx - cx, dy - cy) > radius) continue;
      // Bilinear sample of the icon.
      const sx = Math.min(icon.width - 1, Math.max(0, ((dx + 0.5) * icon.width) / size - 0.5));
      const sy = Math.min(icon.height - 1, Math.max(0, ((dy + 0.5) * icon.height) / size - 0.5));
      const [x0, y0] = [Math.floor(sx), Math.floor(sy)];
      const [x1, y1] = [Math.min(x0 + 1, icon.width - 1), Math.min(y0 + 1, icon.height - 1)];
      const [fx, fy] = [sx - x0, sy - y0];
      const at = (ix: number, iy: number, c: number): number => icon.rgba[(iy * icon.width + ix) * 4 + c]!;
      const mix = (c: number): number =>
        (at(x0, y0, c) * (1 - fx) + at(x1, y0, c) * fx) * (1 - fy) + (at(x0, y1, c) * (1 - fx) + at(x1, y1, c) * fx) * fy;
      const alpha = mix(3) / 255;
      const o = (py * base.width + px) * 4;
      for (let c = 0; c < 3; c++) rgba[o + c] = Math.round(mix(c) * alpha + backing[c]! * (1 - alpha));
      rgba[o + 3] = 255;
    }
  }
  return { width: base.width, height: base.height, rgba };
}
