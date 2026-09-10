// Genere l'icone source de Mecatol, sans dependance externe.
//
// Le motif est celui du diagramme de zones de tolerance : une ligne zero, une
// zone d'alesage au-dessus a gauche, une zone d'arbre en dessous a droite.
// L'icone dit donc ce que fait le logiciel.
//
//   node scripts/make-icon.mjs
//   npx tauri icon src-tauri/icon-source.png

import { deflateSync, crc32 as nodeCrc32 } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

const SIZE = 1024;
const OUT = "src-tauri/icon-source.png";

// Palette alignee sur celle du diagramme.
const BACKGROUND = [15, 23, 42, 255]; //  ardoise profonde
const ZERO_LINE = [226, 232, 240, 255]; //  ligne zero
const HOLE = [59, 130, 246, 255]; //  alesage
const SHAFT = [234, 88, 12, 255]; //  arbre

const CORNER_RADIUS = SIZE * 0.22;

/** Vrai si le pixel tombe dans le carre aux angles arrondis. */
function insideRoundedSquare(x, y) {
  const r = CORNER_RADIUS;
  const cx = Math.min(Math.max(x, r), SIZE - r);
  const cy = Math.min(Math.max(y, r), SIZE - r);
  return (x - cx) ** 2 + (y - cy) ** 2 <= r * r;
}

function inRect(x, y, left, top, width, height) {
  return x >= left && x < left + width && y >= top && y < top + height;
}

// Geometrie du motif, en fractions de l'icone.
const ZERO_Y = SIZE * 0.5;
const LINE_HALF = SIZE * 0.012;
const HOLE_RECT = [SIZE * 0.2, SIZE * 0.26, SIZE * 0.3, SIZE * 0.24];
const SHAFT_RECT = [SIZE * 0.5, SIZE * 0.56, SIZE * 0.3, SIZE * 0.24];

function pixelAt(x, y) {
  if (!insideRoundedSquare(x, y)) return [0, 0, 0, 0];
  if (inRect(x, y, ...HOLE_RECT)) return HOLE;
  if (inRect(x, y, ...SHAFT_RECT)) return SHAFT;
  if (Math.abs(y - ZERO_Y) < LINE_HALF && x > SIZE * 0.12 && x < SIZE * 0.88) {
    return ZERO_LINE;
  }
  return BACKGROUND;
}

/** Assemble les lignes de balayage, chacune prefixee par son octet de filtre. */
function rasterise() {
  const stride = SIZE * 4;
  const raw = Buffer.alloc((stride + 1) * SIZE);
  for (let y = 0; y < SIZE; y += 1) {
    const rowStart = y * (stride + 1);
    raw[rowStart] = 0; // filtre « None »
    for (let x = 0; x < SIZE; x += 1) {
      const [r, g, b, a] = pixelAt(x + 0.5, y + 0.5);
      const at = rowStart + 1 + x * 4;
      raw[at] = r;
      raw[at + 1] = g;
      raw[at + 2] = b;
      raw[at + 3] = a;
    }
  }
  return raw;
}

const crc32 =
  typeof nodeCrc32 === "function"
    ? (buffer) => nodeCrc32(buffer) >>> 0
    : (() => {
        const table = Array.from({ length: 256 }, (_, n) => {
          let c = n;
          for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
          return c >>> 0;
        });
        return (buffer) => {
          let c = 0xffffffff;
          for (const byte of buffer) c = table[(c ^ byte) & 0xff] ^ (c >>> 8);
          return (c ^ 0xffffffff) >>> 0;
        };
      })();

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

function encodePng() {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(SIZE, 0);
  ihdr.writeUInt32BE(SIZE, 4);
  ihdr[8] = 8; //  8 bits par canal
  ihdr[9] = 6; //  RVB + alpha
  ihdr[10] = 0; //  compression deflate
  ihdr[11] = 0; //  filtrage adaptatif
  ihdr[12] = 0; //  pas d'entrelacement

  return Buffer.concat([
    signature,
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(rasterise(), { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

mkdirSync(dirname(OUT), { recursive: true });
const png = encodePng();
writeFileSync(OUT, png);
console.log(`${OUT} — ${SIZE}×${SIZE}, ${(png.length / 1024).toFixed(1)} Kio`);
