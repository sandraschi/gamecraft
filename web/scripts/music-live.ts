// Live smoke test of the music path (the gate runs in ?verify=1, where music is off):
// open each game normally, click once (the browser's gesture rule), let it play a
// few seconds, toggle with M, and fail on any page error. Also confirms the page
// really created an AudioContext that is running.
//
//   node scripts/music-live.ts [slug]
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const GAMES = join(ROOT, "games");
const only = process.argv[2];
const slugs = readdirSync(GAMES, { withFileTypes: true }).filter((d) => d.isDirectory() && existsSync(join(GAMES, d.name, "game.json"))).map((d) => d.name).filter((s) => !only || s === only);
const server = spawn("bun", ["run", "serve.ts"], { cwd: ROOT, env: { ...process.env, PORT: "0" }, shell: process.platform === "win32" });
const base = await new Promise<string>((res, rej) => {
  const t = setTimeout(() => rej(new Error("server did not start")), 15_000);
  server.stdout.on("data", (c: Buffer) => { const m = /http:\/\/127\.0\.0\.1:(\d+)/.exec(c.toString()); if (m) { clearTimeout(t); res(`http://127.0.0.1:${m[1]}`); } });
});
const browser = await chromium.launch({ headless: true, args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--autoplay-policy=no-user-gesture-required"] });
let fails = 0;
for (const slug of slugs) {
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  await ctx.addInitScript(() => {
    // count AudioContexts and their state so the test can see music really started
    const AC = window.AudioContext;
    (window as any).__acs = [];
    (window as any).AudioContext = class extends AC { constructor(...a: any[]) { super(...a); (window as any).__acs.push(this); } };
  });
  const page = await ctx.newPage();
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  await page.goto(`${base}/games/${slug}/`);
  await page.waitForFunction(() => (window as any).__gamecraft?.ready === true, null, { timeout: 30_000 });
  await page.mouse.click(640, 600);
  await page.keyboard.press("Escape");
  await page.mouse.click(640, 650);
  await page.waitForTimeout(4000);
  const st = await page.evaluate(() => ({ n: (window as any).__acs.length, states: (window as any).__acs.map((a: AudioContext) => a.state), btn: document.getElementById("music-btn")?.className ?? null }));
  await page.keyboard.press("m"); await page.waitForTimeout(500); await page.keyboard.press("m"); await page.waitForTimeout(800);
  const ok = errors.length === 0 && st.n >= 1 && st.states.includes("running");
  if (!ok) fails++;
  console.log(`${slug.padEnd(20)} ${ok ? "ok  " : "FAIL"} contexts=${st.n} ${st.states.join(",")} button=${st.btn} ${errors.slice(0, 3).join(" | ")}`);
  await ctx.close();
}
await browser.close();
if (process.platform === "win32") spawn("taskkill", ["/pid", String(server.pid), "/T", "/F"], { stdio: "ignore" }); else server.kill();
process.exit(fails ? 1 : 0);
