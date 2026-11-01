// Validate glTF/GLB files with the Khronos reference validator (gltf-validator).
// Usage: node scripts/gltf-validate.ts <dir-or-file>...   (exit 1 on any error)
// Warnings and infos are printed but do not fail the gate.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, extname } from "node:path";
// @ts-ignore - the package ships no types
import validator from "gltf-validator";

function files(args: string[]): string[] {
  const out: string[] = [];
  for (const a of args) {
    if (statSync(a).isDirectory()) {
      for (const f of readdirSync(a).sort()) {
        if ([".glb", ".gltf"].includes(extname(f).toLowerCase())) out.push(join(a, f));
      }
    } else out.push(a);
  }
  return out;
}

const list = files(process.argv.slice(2));
if (list.length === 0) {
  console.error("no .glb/.gltf files given");
  process.exit(2);
}
let failed = 0;
for (const path of list) {
  const bytes = new Uint8Array(readFileSync(path));
  const report = await validator.validateBytes(bytes, { maxIssues: 50, uri: path });
  const n = report.issues;
  const ok = n.numErrors === 0;
  if (!ok) failed++;
  console.log(
    `${ok ? "ok  " : "FAIL"} ${path}: ${n.numErrors} errors, ${n.numWarnings} warnings, ${n.numInfos} infos` +
      ` (${report.info?.totalTriangleCount ?? "?"} triangles, ${report.info?.drawCallCount ?? "?"} draw calls)`,
  );
  for (const m of n.messages.filter((m: { severity: number }) => m.severity <= 1).slice(0, 8)) {
    console.log(`     ${m.severity === 0 ? "E" : "W"} ${m.code} ${m.pointer ?? ""}: ${m.message}`);
  }
}
console.log(`${list.length - failed}/${list.length} valid`);
process.exit(failed ? 1 : 0);
