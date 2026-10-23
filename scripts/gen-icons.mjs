// 生成打包 / 托盘所需图标：32x32.png、128x128.png、128x128@2x.png、icon.ico。
//
// 素材：`src/assets/bee-dot.png`（已抠白底的小蜜蜂）。
// 不依赖任何图形库：手写 PNG 解码/编码与 ICO 容器，保证 `pnpm icons` 零额外依赖。
//
// 用法：node scripts/gen-icons.mjs

import { deflateSync, inflateSync } from "node:zlib";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT_DIR = join(ROOT, "src-tauri", "icons");
const BEE_SRC = join(ROOT, "src", "assets", "bee-dot.png");

// ---- PNG 编码 ----

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let crc = -1;
  for (const byte of buffer) {
    crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const typeAndData = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typeAndData), 0);
  return Buffer.concat([length, typeAndData, crc]);
}

/** 把 RGBA 像素编码成 PNG。 */
function encodePng(width, height, rgba) {
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y += 1) {
    raw[y * (stride + 1)] = 0; // filter: none
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type: RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/**
 * 解码 8-bit RGBA PNG（本仓库 bee-dot 就是这种）。
 * 只支持无交错、无调色板的常见导出格式。
 */
function decodePng(buffer) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (!buffer.subarray(0, 8).equals(sig)) {
    throw new Error("bee-dot.png 不是合法 PNG");
  }

  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const idatParts = [];

  while (offset + 8 <= buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.subarray(offset + 4, offset + 8).toString("ascii");
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    offset += 12 + length;

    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === "IDAT") {
      idatParts.push(data);
    } else if (type === "IEND") {
      break;
    }
  }

  if (bitDepth !== 8 || colorType !== 6) {
    throw new Error(`仅支持 8-bit RGBA PNG，实际 bitDepth=${bitDepth} colorType=${colorType}`);
  }

  const inflated = inflateSync(Buffer.concat(idatParts));
  const stride = width * 4;
  const rgba = Buffer.alloc(stride * height);
  let src = 0;

  for (let y = 0; y < height; y += 1) {
    const filter = inflated[src];
    src += 1;
    const row = inflated.subarray(src, src + stride);
    src += stride;
    const dest = y * stride;
    const prev = y > 0 ? rgba.subarray((y - 1) * stride, y * stride) : null;

    for (let i = 0; i < stride; i += 1) {
      const x = row[i];
      const a = i >= 4 ? rgba[dest + i - 4] : 0;
      const b = prev ? prev[i] : 0;
      const c = prev && i >= 4 ? prev[i - 4] : 0;
      let value = x;
      if (filter === 1) value = (x + a) & 0xff;
      else if (filter === 2) value = (x + b) & 0xff;
      else if (filter === 3) value = (x + Math.floor((a + b) / 2)) & 0xff;
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        const pr = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
        value = (x + pr) & 0xff;
      } else if (filter !== 0) {
        throw new Error(`不支持的 PNG filter: ${filter}`);
      }
      rgba[dest + i] = value;
    }
  }

  return { width, height, rgba };
}

/** 双线性缩放到正方形画布；蜜蜂居中铺满约 92%，四周留透明边方便托盘辨认。 */
function renderBeeIcon(src, size) {
  const rgba = Buffer.alloc(size * size * 4);
  const inset = Math.max(1, Math.round(size * 0.04));
  const target = size - inset * 2;
  const scale = Math.min(target / src.width, target / src.height);
  const drawW = src.width * scale;
  const drawH = src.height * scale;
  const ox = (size - drawW) / 2;
  const oy = (size - drawH) / 2;

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const offset = (y * size + x) * 4;
      const u = (x + 0.5 - ox) / scale - 0.5;
      const v = (y + 0.5 - oy) / scale - 0.5;
      if (u < -0.5 || v < -0.5 || u >= src.width - 0.5 || v >= src.height - 0.5) {
        continue;
      }

      const x0 = Math.floor(u);
      const y0 = Math.floor(v);
      const x1 = Math.min(x0 + 1, src.width - 1);
      const y1 = Math.min(y0 + 1, src.height - 1);
      const fx = u - x0;
      const fy = v - y0;

      const sample = (sx, sy) => {
        const i = (sy * src.width + sx) * 4;
        return [
          src.rgba[i],
          src.rgba[i + 1],
          src.rgba[i + 2],
          src.rgba[i + 3],
        ];
      };

      const p00 = sample(Math.max(0, x0), Math.max(0, y0));
      const p10 = sample(x1, Math.max(0, y0));
      const p01 = sample(Math.max(0, x0), y1);
      const p11 = sample(x1, y1);

      for (let c = 0; c < 4; c += 1) {
        const top = p00[c] * (1 - fx) + p10[c] * fx;
        const bot = p01[c] * (1 - fx) + p11[c] * fx;
        rgba[offset + c] = Math.round(top * (1 - fy) + bot * fy);
      }
    }
  }

  return encodePng(size, size, rgba);
}

// ---- ICO 封装（Vista+ 支持内嵌 PNG）----

function buildIco(images) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(images.length, 4);

  const entries = Buffer.alloc(16 * images.length);
  let offset = 6 + 16 * images.length;

  images.forEach((image, index) => {
    const base = index * 16;
    entries[base] = image.size >= 256 ? 0 : image.size;
    entries[base + 1] = image.size >= 256 ? 0 : image.size;
    entries[base + 2] = 0; // palette
    entries[base + 3] = 0; // reserved
    entries.writeUInt16LE(1, base + 4); // color planes
    entries.writeUInt16LE(32, base + 6); // bits per pixel
    entries.writeUInt32LE(image.data.length, base + 8);
    entries.writeUInt32LE(offset, base + 12);
    offset += image.data.length;
  });

  return Buffer.concat([header, entries, ...images.map((image) => image.data)]);
}

// ---- 输出 ----

mkdirSync(OUT_DIR, { recursive: true });

const bee = decodePng(readFileSync(BEE_SRC));
const png32 = renderBeeIcon(bee, 32);
const png128 = renderBeeIcon(bee, 128);
const png256 = renderBeeIcon(bee, 256);

writeFileSync(join(OUT_DIR, "32x32.png"), png32);
writeFileSync(join(OUT_DIR, "128x128.png"), png128);
writeFileSync(join(OUT_DIR, "128x128@2x.png"), png256);
writeFileSync(
  join(OUT_DIR, "icon.ico"),
  buildIco([
    { size: 32, data: png32 },
    { size: 256, data: png256 },
  ]),
);

console.log("图标已生成（小蜜蜂）：");
for (const name of ["32x32.png", "128x128.png", "128x128@2x.png", "icon.ico"]) {
  console.log(` - src-tauri/icons/${name}`);
}
