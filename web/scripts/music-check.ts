// Renders every music style offline in headless Chromium and checks it:
// no NaN, no clipping, not too quiet or loud, no silent stretches, lead in a
// sane register. Writes a WAV per style so a human can actually listen.
//
//   node scripts/music-check.ts [outDir]     (default: ../../temp/music next to the repo)
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const OUT = resolve(process.argv[2] ?? join(ROOT, "..", "..", "temp", "music"));
mkdirSync(OUT, { recursive: true });

const server = spawn("bun", ["run", "serve.ts"], { cwd: ROOT, env: { ...process.env, PORT: "0" }, shell: process.platform === "win32" });
const base = await new Promise<string>((res, rej) => {
  const t = setTimeout(() => rej(new Error("server did not start")), 15_000);
  server.stdout.on("data", (c: Buffer) => { const m = /http:\/\/127\.0\.0\.1:(\d+)/.exec(c.toString()); if (m) { clearTimeout(t); res(`http://127.0.0.1:${m[1]}`); } });
});
const browser = await chromium.launch({ headless: true, args: ["--autoplay-policy=no-user-gesture-required"] });
const page = await browser.newPage();
const errors: string[] = [];
page.on("pageerror", (e) => errors.push(e.message));
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
await page.goto(`${base}/games/kintsugi/?verify=1`);
await page.waitForFunction(() => (window as any).__gamecraft?.ready === true);

const results = await page.evaluate(async () => {
  const mod = await import("/games/_shared/music.js");
  const out: any[] = [];
  for (const style of Object.keys(mod.STYLES)) {
    const d = mod.describe(style);
    const r = await mod.renderOffline(style, 30, 3);
    // mono 16-bit PCM for listening
    const L = r.buffer.getChannelData(0), R = r.buffer.getChannelData(1);
    const pcm = new Int16Array(L.length);
    for (let i = 0; i < L.length; i++) pcm[i] = Math.max(-32768, Math.min(32767, ((L[i] + R[i]) / 2) * 32767));
    let bin = ""; const bytes = new Uint8Array(pcm.buffer);
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    out.push({ style, peak: r.peak, rms: r.rms, nan: r.nan, silentSec: r.silentSec, ...d, sr: r.buffer.sampleRate, pcm: btoa(bin) });
  }
  return out;
});

function wav(pcm: Buffer, sr: number): Buffer {
  const h = Buffer.alloc(44);
  h.write("RIFF", 0); h.writeUInt32LE(36 + pcm.length, 4); h.write("WAVE", 8); h.write("fmt ", 12);
  h.writeUInt32LE(16, 16); h.writeUInt16LE(1, 20); h.writeUInt16LE(1, 22); h.writeUInt32LE(sr, 24); h.writeUInt32LE(sr * 2, 28); h.writeUInt16LE(2, 32); h.writeUInt16LE(16, 34);
  h.write("data", 36); h.writeUInt32LE(pcm.length, 40);
  return Buffer.concat([h, pcm]);
}
let fails = 0;
for (const r of results) {
  writeFileSync(join(OUT, `${r.style}.wav`), wav(Buffer.from(r.pcm, "base64"), r.sr));
  const problems = [];
  if (r.nan) problems.push("NaN samples");
  if (r.peak >= 0.99) problems.push(`clipping (peak ${r.peak})`);
  if (r.rms < 0.01) problems.push(`too quiet (rms ${r.rms})`);
  if (r.rms > 0.25) problems.push(`too loud (rms ${r.rms})`);
  if (r.silentSec > 2) problems.push(`${r.silentSec} silent seconds`);
  if (r.lo < 52 || r.hi > 93) problems.push(`lead range ${r.lo}..${r.hi} out of 52..93`);
  if (r.hi - r.lo < 7) problems.push(`lead spans only ${r.hi - r.lo} semitones`);
  if (problems.length) fails++;
  console.log(`${r.style.padEnd(11)} peak ${String(r.peak).padEnd(6)} rms ${String(r.rms).padEnd(7)} lead ${r.lo}-${r.hi}  form ${r.bars} bars ${r.seconds}s  ${problems.length ? "FAIL " + problems.join(", ") : "ok"}`);
}
if (errors.length) { console.log("page errors:", errors); fails++; }
await browser.close();
if (process.platform === "win32") spawn("taskkill", ["/pid", String(server.pid), "/T", "/F"], { stdio: "ignore" }); else server.kill();
console.log(`wav files in ${OUT}`);
process.exit(fails ? 1 : 0);
