// GameCraft web arcade: static server for the three.js game family.
//
//   bun run serve.ts            -> http://127.0.0.1:11258/
//   PORT=0 (via startServer(0)) -> ephemeral port, used by scripts/verify.ts
//
// Routes
//   /                     arcade index (index.html)
//   /api/games            game.json + verify.json for every games/<slug>/
//   /vendor/three/*       node_modules/three/build/*   (import map target)
//   /vendor/three-addons/* node_modules/three/examples/jsm/*
//   /games/<slug>/*       the game files
//   /sw.js, /pwa.js       service worker (generated) and its registration (pwa.ts)
//   /manifest.webmanifest, /games/<slug>/manifest.webmanifest, /icons/*   install metadata
//
// HTML pages (arcade index, each game's index.html) get the PWA <head> tags
// injected at serve time, so no game file carries PWA boilerplate.
//
// Only the web/ tree is served, never node_modules wholesale, never ..

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { gameSlugs, injectHead, manifest, serviceWorker } from "./pwa.ts";

const ROOT = import.meta.dir;
const GAMES = join(ROOT, "games");
const THREE_BUILD = join(ROOT, "node_modules", "three", "build");
const THREE_ADDONS = join(ROOT, "node_modules", "three", "examples", "jsm");
export const DEFAULT_PORT = 11258;

function safeJoin(base: string, rel: string): string | null {
  const full = resolve(base, "." + sep + decodeURIComponent(rel));
  return full === base || full.startsWith(base + sep) ? full : null;
}

function readJson(path: string): unknown {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

export function listGames() {
  if (!existsSync(GAMES)) return [];
  return readdirSync(GAMES, { withFileTypes: true })
    .filter((d) => d.isDirectory() && existsSync(join(GAMES, d.name, "game.json")))
    .map((d) => ({
      ...(readJson(join(GAMES, d.name, "game.json")) as object),
      verify: readJson(join(GAMES, d.name, "verify.json")),
      has_thumb: existsSync(join(GAMES, d.name, "thumb.png")),
    }));
}

async function file(path: string | null): Promise<Response> {
  if (!path) return new Response("forbidden", { status: 403 });
  const f = Bun.file(path);
  if (!(await f.exists())) return new Response("not found", { status: 404 });
  return new Response(f, { headers: { "cache-control": "no-store" } });
}

const NO_STORE = { "cache-control": "no-store" };

async function page(path: string, slug: string | null): Promise<Response> {
  const f = Bun.file(path);
  if (!(await f.exists())) return new Response("not found", { status: 404 });
  return new Response(injectHead(await f.text(), slug), {
    headers: { ...NO_STORE, "content-type": "text/html; charset=utf-8" },
  });
}

const PRESETS = [
  { name: "Emberwick Village", family: "web-rpg", prompts: [
    "A three.js engine demo for an angled top-down RPG: player walks a fantasy village with one or two companions; entering a building turns the camera-side wall transparent.",
    "Add NPCs, plenty of furniture, several building types, some with an upstairs.",
    "Make the landscape not flat; give two buildings basements joined by a tunnel; add a rocky mountain with a cave holding a sleeping troll.",
    "Turn it into an actual game: four times the playable area, fun mechanics, a plot that is not too cliche with twists, a magic system, make it really good." ] },
  { name: "Space Station", family: "web-rpg", prompts: [
    "A three.js engine demo for an angled top-down RPG: player + 1-2 crew walk a ring station (bridge, sick bay, engineering, mess); entering a module turns the camera-side wall transparent. Transporter pad to a surface outpost.",
    "Add NPC crew with shifts, consoles/bunks/med-beds, module types (bridge, sick bay, engineering, hydroponics), some with an upper deck. Tribbles multiply if fed — cute, then a problem.",
    "Split levels with ramps/ladders; two service modules joined by crawlable ventilation ducts; asteroid with a cave holding a sleeping alien. Dilithium crystal search: 3 shards hidden (one in ducts, one in alien cave, one behind a locked door).",
    "Turn it into a game: 4x modules, oxygen + mag-boots + hacking + transporter charges, mutiny-over-dilithium plot with twists, tech system, really good. Win: all shards to engineering + cure the tribble infestation." ] },
  { name: "Pirate Ship", family: "web-rpg", prompts: [
    "A three.js engine demo for an angled top-down RPG: player + mates walk a galleon (helm, galley, powder magazine, sick bay/cockpit, captain's cabin); entering a cabin turns the camera-side wall transparent. Longboat to a shore outpost.",
    "Add NPC crew with watches, hammocks/cannons/galley stove, deck types (gun deck, orlop, crow's nest), grog rationing, parley dialogue.",
    "Rolling sea (non-flat deck); two holds joined by a smuggler tunnel; island mountain + cave + kraken. Buried-treasure search: 3 map pieces (one in crow's nest, one in kraken cave, one behind a locked brig door).",
    "Turn it into a game: 4x ship+island, rigging + cannon + parley + grog morale, cursed-gold mutiny plot with twists, sea-magic, really good. Win: all map pieces + kraken parleyed or beaten." ] },
  { name: "Zelda-like", family: "web-zelda", prompts: [
    "Top-down Zelda-like: 64x64 overworld, 3 shrines (push stone onto switch for key), locked boss door, hearts, 1 boss.",
    "Win: all shrines + boss. BFS-provable. Thumb: boss door." ] },
  { name: "Sokoban 9x9", family: "web-sokoban", prompts: [
    "Sokoban 9x9 vault, 5 crates, undo + par. Crate-push BFS solvable. Thumb: solved vault." ] },
];

function manifestResponse(slug: string | null): Response {
  return new Response(JSON.stringify(manifest(slug), null, 2), {
    headers: { ...NO_STORE, "content-type": "application/manifest+json" },
  });
}

export function startServer(port: number = DEFAULT_PORT) {
  if (!existsSync(THREE_BUILD)) {
    throw new Error("three.js not installed: run `bun install` in gamecraft/web first");
  }
  return Bun.serve({
    port,
    hostname: process.env.HOST || "0.0.0.0",
    async fetch(req) {
      const { pathname } = new URL(req.url);
      if (pathname === "/" || pathname === "/index.html") return page(join(ROOT, "index.html"), null);
      if (pathname === "/new" || pathname === "/new.html") return page(join(ROOT, "new.html"), null);
      if (pathname === "/api/presets") return Response.json(PRESETS);
      if (pathname === "/api/new/start" && req.method === "POST") {
        // Stub: job runner per new-game-page-DOSSIER §5 (gamecraft-cli new + session runner + budget guard).
        return Response.json({ error: "not implemented — wire /api/new/start per DOSSIER §5" }, { status: 501 });
      }
      if (pathname === "/api/games") return Response.json(listGames());
      if (pathname === "/sw.js") {
        return new Response(serviceWorker(), { headers: { ...NO_STORE, "content-type": "text/javascript" } });
      }
      if (pathname === "/pwa.js") return file(join(ROOT, "pwa.js"));
      if (pathname === "/manifest.webmanifest") return manifestResponse(null);
      if (pathname.startsWith("/icons/")) return file(safeJoin(join(ROOT, "icons"), pathname.slice("/icons/".length)));
      const game = pathname.match(/^\/games\/([a-z0-9-]+)\/(index\.html|manifest\.webmanifest)?$/);
      if (game && gameSlugs().includes(game[1])) {
        if (game[2] === "manifest.webmanifest") return manifestResponse(game[1]);
        return page(join(GAMES, game[1], "index.html"), game[1]);
      }
      if (pathname.startsWith("/vendor/three/")) {
        return file(safeJoin(THREE_BUILD, pathname.slice("/vendor/three/".length)));
      }
      if (pathname.startsWith("/vendor/three-addons/")) {
        return file(safeJoin(THREE_ADDONS, pathname.slice("/vendor/three-addons/".length)));
      }
      if (pathname.startsWith("/games/")) {
        let rel = pathname.slice("/games/".length);
        if (rel.endsWith("/")) rel += "index.html";
        return file(safeJoin(GAMES, rel));
      }
      return new Response("not found", { status: 404 });
    },
  });
}

if (import.meta.main) {
  const s = startServer(Number(process.env.PORT ?? DEFAULT_PORT));
  console.log(`gamecraft web arcade: http://127.0.0.1:${s.port}/`);
}
