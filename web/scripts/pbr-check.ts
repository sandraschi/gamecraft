// Headless check of `pbr()` from games/_shared/gc.js: the material gets the right maps,
// colour spaces and wrapping, and the maps change what is rendered (a metal ORM pixel loses
// its diffuse colour, an open ORM keeps it). Usage: node scripts/pbr-check.ts
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { chromium } from "playwright";
import { deflateSync } from "node:zlib";

const root = resolve(import.meta.dirname, "..");
const threeRoot = join(root, "node_modules", "three");

/** A solid-colour 4x4 PNG, built by hand so the check needs no image library. */
function png(r: number, g: number, b: number): Buffer {
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc = (buf: Buffer) => {
    let c = 0xffffffff;
    for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer) => {
    const head = Buffer.alloc(8);
    head.writeUInt32BE(data.length, 0);
    head.write(type, 4, "ascii");
    const checksum = Buffer.alloc(4);
    checksum.writeUInt32BE(crc(Buffer.concat([head.subarray(4), data])), 0);
    return Buffer.concat([head, data, checksum]);
  };
  const size = 4;
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array(size).fill([r, g, b]).flat())]);
  const raw = Buffer.concat(Array(size).fill(row));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const images: Record<string, Buffer> = {
  "/albedo.png": png(220, 30, 30),
  "/flat-normal.png": png(128, 128, 255),
  "/open.png": png(255, 26, 0), // occlusion 1, roughness 0.1, metallic 0
  "/metal.png": png(255, 26, 255), // same, metallic 1 (smooth: a rough metal is not dark)
};

const page_html = `<!doctype html><html><body style="margin:0;background:#000">
<script type="importmap">{"imports":{"three":"/three/build/three.module.js"}}</script>
<script type="module">
import * as THREE from "three";
import { pbr } from "/gc.js";
const renderer = new THREE.WebGLRenderer({ preserveDrawingBuffer: true });
renderer.setSize(64, 64);
document.body.appendChild(renderer.domElement);
const camera = new THREE.PerspectiveCamera(40, 1, 0.1, 50);
camera.position.set(0, 0, 4);
const result = {};
const shade = async (orm) => {
  const mat = pbr({ albedo: "/albedo.png", orm, normal: "/flat-normal.png" }, { repeat: 2 });
  const maps = [mat.map, mat.aoMap, mat.roughnessMap, mat.metalnessMap, mat.normalMap].filter(Boolean);
  for (let i = 0; i < 400 && !maps.every((t) => t.image); i++) await new Promise((r) => setTimeout(r, 25));
  const scene = new THREE.Scene();
  scene.add(new THREE.Mesh(new THREE.PlaneGeometry(3, 3), mat));
  const sun = new THREE.DirectionalLight(0xffffff, 3);
  sun.position.set(1, 1, 3);
  scene.add(sun);
  renderer.render(scene, camera);
  const gl = renderer.getContext();
  const px = new Uint8Array(4);
  gl.readPixels(32, 32, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
  return { mat, red: px[0], loaded: maps.every((t) => t.image) };
};
const open = await shade("/open.png");
const metal = await shade("/metal.png");
result.red = { open: open.red, metal: metal.red };
result.loaded = open.loaded && metal.loaded;
const m = open.mat;
result.setup = {
  sharedOrm: m.aoMap === m.roughnessMap && m.roughnessMap === m.metalnessMap,
  albedoSrgb: m.map.colorSpace === THREE.SRGBColorSpace,
  dataLinear: [m.aoMap, m.normalMap].every((t) => t.colorSpace === THREE.NoColorSpace),
  repeats: m.map.wrapS === THREE.RepeatWrapping && m.map.repeat.x === 2,
  factors: m.roughness === 1 && m.metalness === 1,
};
window.__pbr = result;
</script></body></html>`;

const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage();
const errors: string[] = [];
page.on("pageerror", (e) => errors.push(String(e)));
await page.route("http://gc.local/**", async (route) => {
  const path = new URL(route.request().url()).pathname;
  if (path === "/") return route.fulfill({ contentType: "text/html", body: page_html });
  if (path === "/gc.js") return route.fulfill({ contentType: "text/javascript", body: readFileSync(join(root, "games", "_shared", "gc.js")) });
  if (images[path]) return route.fulfill({ contentType: "image/png", body: images[path] });
  if (path.startsWith("/three/")) return route.fulfill({ contentType: "text/javascript", body: readFileSync(join(threeRoot, path.slice(7))) });
  return route.fulfill({ status: 404, body: "" });
});
await page.goto("http://gc.local/");
await page.waitForFunction(() => (window as any).__pbr, null, { timeout: 30000 });
const r = await page.evaluate(() => (window as any).__pbr);
await browser.close();

const problems: string[] = [...errors];
if (!r.loaded) problems.push("textures did not load");
for (const [key, value] of Object.entries(r.setup)) if (!value) problems.push(`material setup: ${key}`);
if (!(r.red.metal + 20 < r.red.open)) problems.push(`metal ORM must be darker than open ORM: ${JSON.stringify(r.red)}`);
console.log(`${problems.length ? "FAIL" : "ok  "} pbr(): ${JSON.stringify(r)}${problems.length ? " problems: " + problems.join("; ") : ""}`);
process.exit(problems.length ? 1 : 0);