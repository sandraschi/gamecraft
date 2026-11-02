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
