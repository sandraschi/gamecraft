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
//
// Only the web/ tree is served, never node_modules wholesale, never ..

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";

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

export function startServer(port: number = DEFAULT_PORT) {
  if (!existsSync(THREE_BUILD)) {
    throw new Error("three.js not installed: run `bun install` in gamecraft/web first");
  }
  return Bun.serve({
    port,
    hostname: "127.0.0.1",
    async fetch(req) {
      const { pathname } = new URL(req.url);
      if (pathname === "/" || pathname === "/index.html") return file(join(ROOT, "index.html"));
      if (pathname === "/api/games") return Response.json(listGames());
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
