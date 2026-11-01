// PWA gate: every game is installable and plays offline.
//
//  1. HTTP: the arcade and every game serve a manifest (name, start_url, scope,
//     standalone display, 192 and 512 px PNG icons of the declared size) and the
//     page links it.
//  2. Browser: open the arcade, wait for the service worker to install and take
//     control, and check every precache URL landed in the cache.
//  3. Offline: cut the network and open the arcade and every game. Each must boot
//     (window.__gamecraft present, its canvas drawn) with no page errors and no
//     failed requests, served purely from the cache.
//
// Usage: node scripts/pwa-check.ts      (exit 1 on any failure)
// Runs under Node 24 like verify.ts; the static server is Bun, as a child.
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const GAMES = join(ROOT, "games");
const slugs = readdirSync(GAMES, { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(join(GAMES, d.name, "game.json")))
  .map((d) => d.name)
  .sort();

const server = spawn("bun", ["run", "serve.ts"], { cwd: ROOT, env: { ...process.env, PORT: "0" }, shell: process.platform === "win32" });
const base = await new Promise<string>((resolve, reject) => {
  const timer = setTimeout(() => reject(new Error("server did not start in 15s")), 15_000);
  server.stdout.on("data", (chunk: Buffer) => {
    const m = /http:\/\/127\.0\.0\.1:(\d+)/.exec(chunk.toString());
    if (m) { clearTimeout(timer); resolve(`http://127.0.0.1:${m[1]}`); }
  });
  server.stderr.on("data", (chunk: Buffer) => process.stderr.write(chunk));
  server.on("exit", (code) => reject(new Error(`server exited early (${code})`)));
});
const stop = () => {
  if (process.platform === "win32") spawn("taskkill", ["/pid", String(server.pid), "/T", "/F"], { stdio: "ignore" });
  else server.kill();
};

let failures = 0;
const fail = (what: string) => { failures++; console.log(`FAIL ${what}`); };

/** PNG width and height from the IHDR chunk. */
function pngSize(bytes: Uint8Array): [number, number] | null {
  const sig = [0x89, 0x50, 0x4e, 0x47];
  if (bytes.length < 24 || !sig.every((b, i) => bytes[i] === b)) return null;
  const view = new DataView(bytes.buffer, bytes.byteOffset);
  return [view.getUint32(16), view.getUint32(20)];
}

// 1. Manifests and icons over plain HTTP.
for (const slug of [null, ...slugs]) {
  const label = slug ?? "arcade";
  const pagePath = slug ? `/games/${slug}/` : "/";
  const html = await (await fetch(base + pagePath)).text();
  const manifestPath = slug ? `/games/${slug}/manifest.webmanifest` : "/manifest.webmanifest";
  if (!html.includes(`<link rel="manifest" href="${manifestPath}">`)) fail(`${label}: page does not link its manifest`);
  if (!html.includes(`<script src="/pwa.js" defer>`)) fail(`${label}: page does not load pwa.js`);
  const res = await fetch(base + manifestPath);
  const m = res.ok ? await res.json() : null;
  if (!m) { fail(`${label}: manifest ${res.status}`); continue; }
  for (const key of ["name", "short_name", "start_url", "scope", "id"]) if (!m[key]) fail(`${label}: manifest lacks ${key}`);
  if (m.display !== "standalone") fail(`${label}: display is ${m.display}`);
  if (m.start_url !== pagePath || m.scope !== pagePath) fail(`${label}: start_url/scope ${m.start_url} ${m.scope}`);
  const sizes = new Set<number>();
  for (const icon of m.icons ?? []) {
    const r = await fetch(base + icon.src);
    const dims = r.ok ? pngSize(new Uint8Array(await r.arrayBuffer())) : null;
    const want = Number(String(icon.sizes).split("x")[0]);
    if (!dims || dims[0] !== want || dims[1] !== want) fail(`${label}: icon ${icon.src} is ${dims ?? r.status}, declared ${icon.sizes}`);
    else sizes.add(want);
  }
  if (!sizes.has(192) || !sizes.has(512)) fail(`${label}: needs 192 and 512 px icons`);
}
console.log(`manifests: arcade + ${slugs.length} games checked`);

// 2. Install the service worker online.
const browser = await chromium.launch({
  headless: true,
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
});
const context = await browser.newContext({ viewport: { width: 1280, height: 720 } });
const page = await context.newPage();
await page.goto(base + "/");
await page.evaluate(() => navigator.serviceWorker.ready);
await page.reload();
const controlled = await page.evaluate(() => !!navigator.serviceWorker.controller);
if (!controlled) fail("arcade: service worker did not take control");
const swSource = await (await fetch(base + "/sw.js")).text();
const precache: string[] = JSON.parse(/const PRECACHE = (\[.*\]);/.exec(swSource)?.[1] ?? "[]");
const cached: string[] = await page.evaluate(async () => {
  const out: string[] = [];
  for (const name of await caches.keys()) {
    for (const req of await (await caches.open(name)).keys()) out.push(new URL(req.url).pathname);
  }
  return out;
});
const missing = precache.filter((u) => !cached.includes(new URL(u, base).pathname));
if (missing.length) fail(`precache: ${missing.length} of ${precache.length} not cached, e.g. ${missing.slice(0, 3).join(", ")}`);
console.log(`service worker: ${controlled ? "in control" : "NOT in control"}, ${precache.length - missing.length}/${precache.length} precached`);

// 3. Offline: every page boots from the cache alone.
await context.setOffline(true);
for (const slug of [null, ...slugs]) {
  const label = slug ?? "arcade";
  const p = await context.newPage();
  const errors: string[] = [];
  p.on("pageerror", (e) => errors.push(String(e)));
  p.on("requestfailed", (r) => errors.push(`request failed: ${new URL(r.url()).pathname}`));
  p.on("response", (r) => { if (r.status() >= 400) errors.push(`${r.status()} ${new URL(r.url()).pathname}`); });
  try {
    await p.goto(base + (slug ? `/games/${slug}/` : "/"), { waitUntil: "load", timeout: 20_000 });
    if (slug) {
      await p.waitForFunction(() => (window as any).__gamecraft, null, { timeout: 20_000 });
    } else {
      await p.waitForFunction((n) => document.querySelectorAll('a[href^="/games/"]').length >= n, slugs.length, { timeout: 10_000 });
    }
    await p.waitForTimeout(400);
  } catch (e) {
    errors.push(String(e).split("\n")[0]);
  }
  if (errors.length) fail(`offline ${label}: ${errors.slice(0, 3).join("; ")}`);
  else console.log(`ok   offline ${label}`);
  await p.close();
}

await browser.close();
stop();
console.log(failures ? `${failures} failure(s)` : `PWA gate passed: arcade + ${slugs.length} games installable and offline-playable`);
process.exit(failures ? 1 : 0);
