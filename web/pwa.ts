// PWA layer for the web arcade: manifests, the service worker, and the <head>
// tags serve.ts injects into every page. Nothing here is per-game hand work: a
// new game folder with game.json + icons is installable and offline-capable.
//
// - One service worker at /sw.js (scope /) for the whole arcade. Network-first,
//   so a running dev server always wins and edits show immediately; the cache
//   answers only when the network fails (phone offline, Goliath asleep).
// - On install it precaches every game, _shared, three.js and the addons the
//   games import, so a game never opened online still plays offline.
// - The cache name carries a hash of every precached file's path, size and
//   mtime: any change installs a fresh cache and drops the old one.
// - Each game has its own manifest (scope /games/<slug>/), so it installs as its
//   own app; the arcade index installs as "GameCraft".
// - Service workers need a secure context: http://127.0.0.1 counts, a phone needs
//   HTTPS (e.g. `tailscale serve`), see web/README.md.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";

const ROOT = import.meta.dir;
const GAMES = join(ROOT, "games");
const THREE_BUILD = join(ROOT, "node_modules", "three", "build");
const THREE_ADDONS = join(ROOT, "node_modules", "three", "examples", "jsm");
export const THEME = "#101418";

/** Files inside a game (or _shared) folder that a page can load. */
function servable(name: string): boolean {
  return /\.(html|js|css|png|json|webmanifest)$/i.test(name) && !/\.bak$|verify\.json$/i.test(name);
}

function walk(dir: string, skipDirs: string[] = ["shots"]): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!skipDirs.includes(entry.name)) out.push(...walk(full, skipDirs));
    } else if (servable(entry.name)) out.push(full);
  }
  return out;
}

function readJson(path: string): Record<string, unknown> {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return {};
  }
}

export function gameSlugs(): string[] {
  if (!existsSync(GAMES)) return [];
  return readdirSync(GAMES, { withFileTypes: true })
    .filter((d) => d.isDirectory() && existsSync(join(GAMES, d.name, "game.json")))
    .map((d) => d.name)
    .sort();
}

/** three/addons modules the games import, plus what those import in turn. */
function addonClosure(): string[] {
  const seen = new Set<string>();
  const queue: string[] = [];
  const scan = (text: string, base: string | null) => {
    for (const m of text.matchAll(/from\s+['"]([^'"]+)['"]|import\(\s*['"]([^'"]+)['"]\s*\)/g)) {
      const spec = m[1] ?? m[2];
      let rel: string | null = null;
      if (spec.startsWith("three/addons/")) rel = spec.slice("three/addons/".length);
      else if (base !== null && spec.startsWith(".")) rel = join(base, spec).split(sep).join("/");
      if (rel && !seen.has(rel) && existsSync(join(THREE_ADDONS, rel))) {
        seen.add(rel);
        queue.push(rel);
      }
    }
  };
  for (const file of walk(GAMES).filter((f) => f.endsWith(".js"))) scan(readFileSync(file, "utf8"), null);
  while (queue.length) {
    const rel = queue.shift()!;
    const dir = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
    scan(readFileSync(join(THREE_ADDONS, rel), "utf8"), dir);
  }
  return [...seen].sort();
}

/** Every URL the service worker precaches, with the disk file behind it (null = dynamic). */
export function precache(): { url: string; file: string | null }[] {
  const list: { url: string; file: string | null }[] = [
    { url: "/", file: join(ROOT, "index.html") },
    { url: "/api/games", file: null },
    { url: "/manifest.webmanifest", file: null },
    { url: "/pwa.js", file: join(ROOT, "pwa.js") },
    { url: "/icons/icon-192.png", file: join(ROOT, "icons", "icon-192.png") },
    { url: "/icons/icon-512.png", file: join(ROOT, "icons", "icon-512.png") },
  ];
  for (const name of ["three.module.js", "three.core.js"]) {
    if (existsSync(join(THREE_BUILD, name))) list.push({ url: `/vendor/three/${name}`, file: join(THREE_BUILD, name) });
  }
  for (const rel of addonClosure()) list.push({ url: `/vendor/three-addons/${rel}`, file: join(THREE_ADDONS, rel) });
  for (const file of walk(join(GAMES, "_shared"))) {
    list.push({ url: "/games/" + relative(GAMES, file).split(sep).join("/"), file });
  }
  for (const slug of gameSlugs()) {
    const dir = join(GAMES, slug);
    list.push({ url: `/games/${slug}/`, file: join(dir, "index.html") });
    list.push({ url: `/games/${slug}/manifest.webmanifest`, file: null });
    for (const file of walk(dir)) list.push({ url: "/games/" + relative(GAMES, file).split(sep).join("/"), file });
  }
  return list;
}

/** The service worker source, with the precache list and a content-derived cache name. */
export function serviceWorker(): string {
  const list = precache();
  const hash = createHash("sha1");
  for (const { url, file } of list) {
    hash.update(url);
    if (file && existsSync(file)) {
      const s = statSync(file);
      hash.update(`:${s.size}:${s.mtimeMs}`);
    }
  }
  const cache = `gamecraft-${hash.digest("hex").slice(0, 12)}`;
  return `// GameCraft arcade service worker (generated by web/pwa.ts; do not edit).
const CACHE = ${JSON.stringify(cache)};
const PRECACHE = ${JSON.stringify(list.map((e) => e.url))};
self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((c) => c.addAll(PRECACHE)).then(() => self.skipWaiting()));
});
self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("gamecraft-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});
// Network first: the live server always wins; the cache is the offline fallback.
self.addEventListener("fetch", (event) => {
  const req = event.request;
  const url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== self.location.origin) return;
  event.respondWith((async () => {
    try {
      const res = await fetch(req);
      if (res.ok) (await caches.open(CACHE)).put(req, res.clone());
      return res;
    } catch {
      const hit = (await caches.match(req, { ignoreSearch: true })) ??
        (url.pathname.endsWith("/") ? await caches.match(url.pathname, { ignoreSearch: true }) : undefined);
      return hit ?? new Response("offline, and this page was never cached", { status: 503, headers: { "content-type": "text/plain" } });
    }
  })());
});
`;
}

/** Web app manifest for one game, or the arcade when slug is null. */
export function manifest(slug: string | null): Record<string, unknown> {
  if (slug === null) {
    return {
      id: "/",
      name: "GameCraft arcade",
      short_name: "GameCraft",
      description: "Calm three.js games: seek-and-find, puzzles and small worlds. Plays offline once installed.",
      start_url: "/",
      scope: "/",
      display: "standalone",
      background_color: THEME,
      theme_color: THEME,
      categories: ["games"],
      icons: icons("/icons"),
    };
  }
  const meta = readJson(join(GAMES, slug, "game.json"));
  const title = String(meta.title ?? slug);
  const about = String(meta.experiment ?? "");
  return {
    id: `/games/${slug}/`,
    name: title,
    short_name: title.length <= 12 ? title : title.slice(0, 12).trimEnd(),
    description: about.length > 200 ? about.slice(0, 197).trimEnd() + "..." : about,
    start_url: `/games/${slug}/`,
    scope: `/games/${slug}/`,
    display: "standalone",
    background_color: THEME,
    theme_color: THEME,
    categories: ["games"],
    icons: icons(`/games/${slug}`),
  };
}

function icons(base: string) {
  return [192, 512].map((n) => ({ src: `${base}/icon-${n}.png`, sizes: `${n}x${n}`, type: "image/png", purpose: "any" }));
}

/** Insert the manifest link, icons and the registration script before </head>. */
export function injectHead(html: string, slug: string | null): string {
  const base = slug === null ? "" : `/games/${slug}`;
  const iconBase = slug === null ? "/icons" : base;
  const tags = [
    `<link rel="manifest" href="${base}/manifest.webmanifest">`,
    `<link rel="apple-touch-icon" href="${iconBase}/icon-192.png">`,
    `<meta name="theme-color" content="${THEME}">`,
    `<meta name="mobile-web-app-capable" content="yes">`,
    `<meta name="apple-mobile-web-app-capable" content="yes">`,
    `<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">`,
    `<script src="/pwa.js" defer></script>`,
  ].join("\n");
  return html.includes("</head>") ? html.replace("</head>", `${tags}\n</head>`) : `${tags}\n${html}`;
}
