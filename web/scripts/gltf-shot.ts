// Render glTF/GLB files with three.js in headless Chromium and save a PNG each.
// Proves an export loads in a real consumer and looks right, which a validator cannot.
// The report counts meshes whose material actually decoded a map (albedo, normal and the
// three ORM channels), so a texture that fails to bind is visible instead of silently flat.
// Usage: node scripts/gltf-shot.ts <out-dir> <file.glb>...
// The camera auto-frames the scene bounds from a 3/4 view; scene lights are kept
// (scaled down, since the exporter copies Godot's unitless energy) plus a soft fill.
import { readFileSync, mkdirSync } from "node:fs";
import { basename, join, resolve, extname } from "node:path";
import { chromium } from "playwright";

const [outDir, ...inputs] = process.argv.slice(2);
if (!outDir || inputs.length === 0) {
  console.error("usage: node scripts/gltf-shot.ts <out-dir> <file.glb>...");
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });
const threeRoot = resolve(import.meta.dirname, "..", "node_modules", "three");

const page_html = `<!doctype html><html><body style="margin:0;background:#20242c">
<script type="importmap">{"imports":{"three":"/three/build/three.module.js","three/addons/":"/three/examples/jsm/"}}</script>
<script type="module">
import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
renderer.setSize(960, 540);
renderer.outputColorSpace = THREE.SRGBColorSpace;
document.body.appendChild(renderer.domElement);
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x20242c);
scene.add(new THREE.HemisphereLight(0xdde6ff, 0x30281c, 1.4));
const sun = new THREE.DirectionalLight(0xffffff, 1.6);
sun.position.set(-4, 10, 6);
scene.add(sun);
try {
  const gltf = await new GLTFLoader().loadAsync("/model.glb");
  gltf.scene.traverse((o) => { if (o.isLight) o.intensity = Math.min(o.intensity, 1.5); });
  scene.add(gltf.scene);
  const box = new THREE.Box3().setFromObject(gltf.scene);
  const size = box.getSize(new THREE.Vector3());
  const centre = box.getCenter(new THREE.Vector3());
  const radius = Math.max(size.length() / 2, 0.5);
  const camera = new THREE.PerspectiveCamera(45, 960 / 540, radius / 100, radius * 20);
  camera.position.copy(centre).add(new THREE.Vector3(0.9, 0.75, 1.1).normalize().multiplyScalar(radius * 2.3));
  camera.lookAt(centre);
  renderer.render(scene, camera);
  let meshes = 0;
  const maps = { map: 0, normalMap: 0, roughnessMap: 0, metalnessMap: 0, aoMap: 0 };
  gltf.scene.traverse((o) => {
    if (!o.isMesh) return;
    meshes++;
    for (const m of [].concat(o.material)) {
      for (const key of Object.keys(maps)) if (m[key] && m[key].image) maps[key]++;
    }
  });
  window.__shot = { ok: true, meshes, maps, size: size.toArray().map((v) => +v.toFixed(2)) };
} catch (e) {
  window.__shot = { ok: false, error: String(e) };
}
</script></body></html>`;

const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
let failed = 0;
for (const input of inputs) {
  const page = await browser.newPage({ viewport: { width: 960, height: 540 } });
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.route("http://gc.local/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === "/") return route.fulfill({ contentType: "text/html", body: page_html });
    if (path === "/model.glb") return route.fulfill({ contentType: "model/gltf-binary", body: readFileSync(input) });
    if (path.startsWith("/three/")) {
      return route.fulfill({ contentType: "text/javascript", body: readFileSync(join(threeRoot, path.slice(7))) });
    }
    return route.fulfill({ status: 404, body: "" });
  });
  await page.goto("http://gc.local/");
  await page.waitForFunction(() => (window as any).__shot, null, { timeout: 30000 });
  const shot = await page.evaluate(() => (window as any).__shot);
  const png = join(outDir, basename(input, extname(input)) + ".png");
  await page.screenshot({ path: png });
  const ok = shot.ok && shot.meshes > 0 && errors.length === 0;
  if (!ok) failed++;
  console.log(`${ok ? "ok  " : "FAIL"} ${basename(input)}: ${JSON.stringify(shot)}${errors.length ? " errors: " + errors.join("; ") : ""} -> ${png}`);
  await page.close();
}
await browser.close();
process.exit(failed ? 1 : 0);
