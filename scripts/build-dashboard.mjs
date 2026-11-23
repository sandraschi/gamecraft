// GameCraft dashboard generator.
//
//   bun scripts/build-dashboard.mjs        (or: node scripts/build-dashboard.mjs)
//   -> docs/dashboard.html   (self-contained, open it from disk)
//
// Scans depot/games (Godot), web/games (three.js) and bevy/games (Bevy), reads the
// gate results already on disk (trace.json, lint.json, verify.json) and bakes them
// into scripts/dashboard.template.html. Read-only on the games; writes one file.

import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..");
const OUT = join(ROOT, "docs", "dashboard.html");
const TEMPLATE = join(HERE, "dashboard.template.html");

// Same display groups as the desktop sidebar (crates/gamecraft-mcp/src/catalog.rs),
// merged into one taxonomy so the three tracks can share columns and sections.
const GODOT_GROUP = {
  breakout: "Arcade", flappy: "Arcade", pong: "Arcade", runner: "Arcade", "topdown-shooter": "Arcade",
  platformer: "Action", driving: "Action", flight: "Action",
  maze: "Puzzles", sokoban: "Puzzles", "sokoban-vault": "Puzzles", "chessboard-move": "Puzzles", "tower-place": "Puzzles",
  "night-forest": "Worlds and sims", "snowy-mountain-pass": "Worlds and sims", "talk-inventory": "Worlds and sims",
};
const GROUP_ALIAS = { Worlds: "Worlds and sims", Puzzle: "Puzzles", Detective: "Mystery" };
const GROUP_ORDER = [
  "Worlds and sims", "Calm puzzles", "Puzzles", "Board classics", "Seek and find",
  "Mystery", "Arcade", "Action", "Physics",
];

const readJson = (p) => {
  try { return JSON.parse(readFileSync(p, "utf8").replace(/^﻿/, "")); } catch { return null; }
};
const readText = (p) => {
  try { return readFileSync(p, "utf8"); } catch { return ""; }
};
const dirs = (p) =>
  existsSync(p) ? readdirSync(p, { withFileTypes: true }).filter((d) => d.isDirectory() && !d.name.startsWith("_")).map((d) => d.name).sort() : [];
const group = (g) => GROUP_ALIAS[g] ?? g ?? "Other";
const rel = (...p) => ["..", ...p].join("/");

function section(md, heading) {
  const m = md.match(new RegExp("^## " + heading + "\\s*\\n([\\s\\S]*?)(?=^## |$(?![\\s\\S]))", "m"));
  return m ? m[1].trim().replace(/\s+/g, " ") : "";
}

const games = [];

// ---- three.js ---------------------------------------------------------------
for (const slug of dirs(join(ROOT, "web", "games"))) {
  const dir = join(ROOT, "web", "games", slug);
  const meta = readJson(join(dir, "game.json")) ?? readJson(join(dir, "game.plan.json"));
  if (!meta) continue;
  const planned = !existsSync(join(dir, "game.json"));
  const hasEntry = existsSync(join(dir, meta.entry ?? "index.html"));
  const v = readJson(join(dir, "verify.json"));
  let status = "progress";
  let progress = "";
  if (planned) status = "planned";
  else if (v && v.pass) { status = "verified"; progress = (v.checkpoints?.length ?? 0) + " checkpoints, won in " + (v.steps ?? "?") + " steps"; }
  else if (v) { status = "attention"; progress = "failing: " + (v.failed ?? "console errors"); }
  else if (!hasEntry) progress = "no playable entry yet";
  else progress = "playable, not yet verified";
  games.push({
    id: "web:" + slug, slug, track: "threejs", title: meta.title ?? slug,
    group: group(meta.category), status, progress,
    desc: meta.experiment ?? "", author: meta.author ?? "",
    date: v?.date ? String(v.date).slice(0, 10) : "",
    thumb: existsSync(join(dir, "thumb.png")) ? rel("web", "games", slug, "thumb.png") : "",
    cheat: existsSync(join(dir, "CHEATSHEET.md")) ? rel("web", "games", slug, "CHEATSHEET.md") : "",
    playable: !planned && hasEntry,
    selfTest: v?.self_test ? (v.self_test.pass ? "self-test passes" : "self-test failing") : "",
  });
}

// ---- Godot depot ---------------------------------------------------------------
for (const slug of dirs(join(ROOT, "depot", "games"))) {
  const dir = join(ROOT, "depot", "games", slug);
  const readme = readText(join(dir, "README.md"));
  const spec = readJson(join(dir, "spec.json"));
  const trace = readJson(join(dir, "trace.json"));
  const lint = readJson(join(dir, "lint.json"));
  const title = (readme.match(/^# (.+)$/m)?.[1] ?? spec?.title ?? slug).replace(/^GameCraft\s+/, "");
  const win = section(readme, "Win condition");
  const controls = section(readme, "Controls");
  const clean = Array.isArray(lint) ? lint.length === 0 : false;
  let status = "progress";
  let progress = "no trace yet";
  if (trace) {
    if (trace.win && clean) { status = "verified"; progress = "won in " + trace.frames + " frames" + (trace.score ? ", " + trace.score : "") + ", lint clean"; }
    else { status = "attention"; progress = trace.win ? "lint findings: " + (lint?.length ?? "?") : "trace does not win"; }
  }
  const desc = win
    ? "Win: " + win + (controls ? " Controls: " + controls : "")
    : spec ? "Generated from a prompt (" + (spec.archetype ?? "game") + " archetype) and compiled to a Godot scene." : "";
  games.push({
    id: "godot:" + slug, slug, track: "godot", title,
    group: GODOT_GROUP[slug] ?? "Other", status, progress, desc, author: "", date: "",
    thumb: existsSync(join(dir, "thumb.png")) ? rel("depot", "games", slug, "thumb.png") : "",
    gif: existsSync(join(dir, "play.gif")) ? rel("depot", "games", slug, "play.gif") : "",
    folder: rel("depot", "games", slug) + "/",
    cheat: existsSync(join(dir, "CHEATSHEET.md")) ? rel("depot", "games", slug, "CHEATSHEET.md") : "",
    playable: true,
  });
}

// ---- Bevy ---------------------------------------------------------------------
for (const slug of dirs(join(ROOT, "bevy", "games"))) {
  const dir = join(ROOT, "bevy", "games", slug);
  const meta = readJson(join(dir, "game.json"));
  if (!meta) continue;
  const v = readJson(join(dir, "verify.json"));
  let status = "progress";
  let progress = "not yet verified";
  if (v) {
    status = v.pass ? "verified" : "attention";
    progress = v.pass ? String(v.progress ?? "passes its gate").replace(/^GameCraft:\s*/, "") : "failing: " + (v.failed ?? "see verify.json");
  }
  games.push({
    id: "bevy:" + slug, slug, track: "bevy", title: meta.title ?? slug,
    group: group(meta.category), status, progress, desc: meta.experiment ?? "", author: "",
    date: v?.date ? String(v.date).slice(0, 10) : "",
    thumb: existsSync(join(dir, "thumb.png")) ? rel("bevy", "games", slug, "thumb.png") : "",
    cheat: existsSync(join(dir, "CHEATSHEET.md")) ? rel("bevy", "games", slug, "CHEATSHEET.md") : "",
    cmd: meta.bin ? "cargo run --profile " + (meta.profile ?? "play") + " --bin " + meta.bin : "",
    playable: true,
  });
}

games.sort((a, b) => (GROUP_ORDER.indexOf(a.group) + 99) % 99 - (GROUP_ORDER.indexOf(b.group) + 99) % 99 || a.title.localeCompare(b.title));

const docs = [
  ["README", "README.md"], ["Product requirements", "docs/PRD.md"], ["Tiered plan", "docs/TIERED_PLAN.md"],
  ["Writing a new game", "docs/NEW_GAME.md"], ["Craft contract", "docs/CRAFT_CONTRACT.md"],
  ["LLM as game designer", "docs/LLM_AS_GAME_DESIGNER.md"], ["Toy railway", "docs/TOY_RAILWAY.md"],
  ["Changelog", "CHANGELOG.md"], ["Install", "INSTALL.md"],
].filter(([, p]) => existsSync(join(ROOT, p))).map(([label, p]) => ({ label, href: rel(p) }));

const data = {
  generated: new Date().toISOString(),
  arcade: "http://127.0.0.1:11258",
  groups: GROUP_ORDER.filter((g) => games.some((x) => x.group === g)).concat([...new Set(games.map((x) => x.group))].filter((g) => !GROUP_ORDER.includes(g))),
  games, docs,
};

const html = readFileSync(TEMPLATE, "utf8").replace("/*__DATA__*/null", JSON.stringify(data).replace(/</g, "\\u003c"));
writeFileSync(OUT, html.replace(/\r\n/g, "\n"), "utf8");
console.log("wrote " + OUT + " (" + games.length + " games, " + data.groups.length + " groups)");
