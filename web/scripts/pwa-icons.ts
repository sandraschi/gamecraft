// Generate install icons from each game's thumb.png: a centre-cropped square at
// 192 and 512 px (games/<slug>/icon-*.png), plus the arcade icon, a 2x2 mosaic
// of four games (icons/icon-*.png). Re-run after a thumbnail changes.
// Usage: node scripts/pwa-icons.ts
import { chromium } from "playwright";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const GAMES = join(ROOT, "games");
const slugs = readdirSync(GAMES, { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(join(GAMES, d.name, "game.json")) && existsSync(join(GAMES, d.name, "thumb.png")))
  .map((d) => d.name)
  .sort();
const dataUrl = (slug: string) => "data:image/png;base64," + readFileSync(join(GAMES, slug, "thumb.png")).toString("base64");

const browser = await chromium.launch();
const page = await browser.newPage();
await page.setContent("<canvas></canvas>");

/** Draw images into an n x n canvas, one per tile of a k x k grid, each centre-cropped. */
async function render(sources: string[], size: number, grid: number): Promise<Buffer> {
  const url = await page.evaluate(
    async ({ sources, size, grid }) => {
      const canvas = document.querySelector("canvas")!;
      canvas.width = canvas.height = size;
      const ctx = canvas.getContext("2d")!;
      ctx.fillStyle = "#101418";
      ctx.fillRect(0, 0, size, size);
      ctx.imageSmoothingQuality = "high";
      const gap = grid > 1 ? Math.round(size * 0.02) : 0;
      const tile = (size - gap * (grid - 1)) / grid;
      for (let i = 0; i < sources.length; i++) {
        const img = new Image();
        img.src = sources[i];
        await img.decode();
        const side = Math.min(img.width, img.height);
        const sx = (img.width - side) / 2;
        const sy = (img.height - side) / 2;
        const x = (i % grid) * (tile + gap);
        const y = Math.floor(i / grid) * (tile + gap);
        ctx.drawImage(img, sx, sy, side, side, x, y, tile, tile);
      }
      return canvas.toDataURL("image/png");
    },
    { sources, size, grid },
  );
  return Buffer.from(url.split(",")[1], "base64");
}

for (const slug of slugs) {
  for (const size of [192, 512]) {
    writeFileSync(join(GAMES, slug, `icon-${size}.png`), await render([dataUrl(slug)], size, 1));
  }
  console.log(`icons: ${slug}`);
}
// Arcade: four games that read well small, falling back to the first four.
const preferred = ["where-is-benny", "hoshi", "kintsugi", "lantern-river"].filter((s) => slugs.includes(s));
const mosaic = (preferred.length === 4 ? preferred : slugs.slice(0, 4)).map(dataUrl);
mkdirSync(join(ROOT, "icons"), { recursive: true });
for (const size of [192, 512]) writeFileSync(join(ROOT, "icons", `icon-${size}.png`), await render(mosaic, size, 2));
console.log(`icons: arcade (${mosaic.length} tiles)`);
await browser.close();
