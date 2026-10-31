// Web family gate: every games/<slug>/ must
//   1. boot in headless Chromium (WebGL via SwiftShader) with zero console errors
//      and zero uncaught page errors, and expose window.__gamecraft.ready;
//   2. win its own scripted self-play tour inside the step budget, printing the
//      win marker from game.json - checkpoints along the way assert real state
//      (which layer is drawn, wall opacity, floor height), not just position;
//   3. produce a thumbnail and one screenshot per checkpoint.
//
// Results go to games/<slug>/verify.json. Exit code 1 if any game fails.
//
//   node scripts/verify.ts            all games
//   node scripts/verify.ts mossbrook  one game
//
// Runs under Node 24 (native type stripping), not Bun: Playwright's browser
// pipe hangs under the Bun runtime. The static server is still Bun, as a child.

import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

type GameMeta = { slug: string; title: string; win_marker: string; thumb_checkpoint?: string; step_budget?: number };

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const GAMES = join(ROOT, "games");
const only = process.argv[2];
const slugs = readdirSync(GAMES, { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(join(GAMES, d.name, "game.json")))
  .map((d) => d.name)
  .filter((s) => !only || s === only);
if (slugs.length === 0) {
  console.error(only ? `no game named ${only}` : "no games found");
  process.exit(1);
}

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
const browser = await chromium.launch({
  headless: true,
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"],
});

let failures = 0;
for (const slug of slugs) {
  const meta = JSON.parse(readFileSync(join(GAMES, slug, "game.json"), "utf8")) as GameMeta;
  const shotsDir = join(GAMES, slug, "shots");
  mkdirSync(shotsDir, { recursive: true });
  // stale shots from an older tour would sit next to the new ones and mislead
  for (const f of readdirSync(shotsDir)) if (f.endsWith(".png")) unlinkSync(join(shotsDir, f));
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors: string[] = [];
  const warnings: string[] = [];
  const markers: string[] = [];
  page.on("console", (m) => {
    const text = m.text();
    if (m.type() === "error") errors.push(text);
    else if (m.type() === "warning") warnings.push(text);
    if (text.startsWith("GameCraft:")) markers.push(text);
  });
  page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
  const t0 = performance.now();
  const result: Record<string, unknown> = { slug, title: meta.title, date: new Date().toISOString() };
  try {
    await page.goto(`${base}/games/${slug}/?verify=1`, { waitUntil: "load" });
    await page.waitForFunction(() => (window as any).__gamecraft?.ready === true, null, { timeout: 60_000 });
    result.boot_ms = Math.round(performance.now() - t0);
    // Optional design check: games with a pure sim prove that their decisions matter.
    const self = await page.evaluate(() => (window as any).__gamecraft.selfTest?.() ?? null);
    if (self) result.self_test = self;
    await page.evaluate(() => (window as any).__gamecraft.autoplay());
    const budget = meta.step_budget ?? 60_000;
    let st: any = null;
    let seen = 0;
    const shots: string[] = [];
    while (true) {
      st = await page.evaluate(() => (window as any).__gamecraft.step(300));
      while (seen < st.checkpoints.length) {
        const cp = st.checkpoints[seen++].name as string;
        await page.evaluate(() => (window as any).__gamecraft.render());
        const file = join(shotsDir, `${String(seen).padStart(2, "0")}-${cp}.png`);
        await page.screenshot({ path: file });
        shots.push(`shots/${String(seen).padStart(2, "0")}-${cp}.png`);
        if (cp === meta.thumb_checkpoint) await page.screenshot({ path: join(GAMES, slug, "thumb.png") });
      }
      if (st.won || st.failed || !st.autoplay || st.steps >= budget) break;
    }
    result.steps = st.steps;
    result.sim_seconds = +(st.steps / 60).toFixed(1);
    result.checkpoints = st.checkpoints;
    result.shots = shots;
    result.won = st.won && markers.includes(meta.win_marker);
    result.failed = st.failed ?? (st.won ? null : st.steps >= budget ? `step budget ${budget} exhausted` : "autoplay stopped without a win");
  } catch (e) {
    result.won = false;
    result.failed = `harness: ${(e as Error).message}`;
  }
  result.wall_ms = Math.round(performance.now() - t0);
  result.console_errors = errors;
  result.console_warnings = warnings.slice(0, 20);
  result.markers = markers;
  const selfOk = !result.self_test || (result.self_test as { pass: boolean }).pass;
  result.pass = result.won === true && errors.length === 0 && selfOk;
  if (!selfOk) result.failed = `${result.failed ?? ""} self_test failed`.trim();
  writeFileSync(join(GAMES, slug, "verify.json"), JSON.stringify(result, null, 2) + "\n");
  console.log(`${slug}: ${result.pass ? "PASS" : "FAIL"}  steps=${result.steps ?? "-"}  errors=${errors.length}  ${result.failed ?? ""}`);
  if (!result.pass) failures++;
  await page.close();
}

await browser.close();
if (process.platform === "win32") spawn("taskkill", ["/pid", String(server.pid), "/T", "/F"], { stdio: "ignore" });
else server.kill();
if (failures > 0) {
  console.log(`${failures} web game(s) unproven`);
  process.exit(1);
}
console.log("all web games win");
