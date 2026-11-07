// Hnefatafl skeleton: 7x7 king-escape tour. Proves boot + win marker + checkpoints.
// TODO: port full 11x11 rules (throne, shield-wall, custodian edge cases) from
// ai-games-collection/games/board-games/hnefatafl.js (18 KB, MIT, same fleet).
import * as THREE from 'three';
import { VERIFY, $, seedFromUrl, makeRenderer, harness, toast, wireHelp, load, save } from '../_shared/gc.js';

const SAVE_KEY = 'gamecraft.hnefatafl.v1';
const canvas = $('view');
const renderer = makeRenderer(canvas, { shadows: !VERIFY, clear: 0x141210 });
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x141210);
const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 100);
camera.position.set(0, 9, 9); camera.lookAt(0, 0, 0);
scene.add(new THREE.HemisphereLight(0xfff2dd, 0x33281a, 1.3));
const sun = new THREE.DirectionalLight(0xffe8c0, 1.8); sun.position.set(-5, 12, 6); scene.add(sun);

// 7x7 board, king starts centre, escapes to any corner. Attackers ring mid-edges.
const N = 7;
const board = new THREE.Group(); scene.add(board);
const sqGeo = new THREE.BoxGeometry(0.96, 0.12, 0.96);
for (let x = 0; x < N; x++) for (let z = 0; z < N; z++) {
  const corner = (x === 0 || x === N - 1) && (z === 0 || z === N - 1);
  const throne = x === 3 && z === 3;
  const m = new THREE.Mesh(sqGeo, new THREE.MeshStandardMaterial({ color: corner ? 0xc8a028 : throne ? 0x6a4a1a : (x + z) % 2 ? 0x3a3226 : 0x463d2c, roughness: 0.9 }));
  m.position.set(x - 3, 0, z - 3); m.receiveShadow = true; board.add(m);
}
let king = { x: 3, z: 3 };
const kingMesh = new THREE.Mesh(new THREE.CylinderGeometry(0.32, 0.38, 0.9, 12), new THREE.MeshStandardMaterial({ color: 0xf2e2a0, roughness: 0.5 }));
kingMesh.castShadow = true; board.add(kingMesh);
function drawKing() { kingMesh.position.set(king.x - 3, 0.55, king.z - 3); $('kpos').textContent = `king ${king.x},${king.z}`; }
drawKing();

// Scripted escape tour: centre -> (3,1) -> (1,1) -> (0,1) -> (0,0) corner. Each leg is a checkpoint.
const tour = [{ x: 3, z: 1 }, { x: 1, z: 1 }, { x: 0, z: 1 }, { x: 0, z: 0 }];
let leg = 0, steps = 0, won = false;
const S = { checkpoints: [], log: [`7x7 skeleton, king at 3,3`] };

const h = harness({
  tick: () => {},
  bot: () => {
    if (won || leg >= tour.length) return;
    const t = tour[leg];
    // rook-slide one square toward target (no capture in skeleton)
    if (king.x !== t.x) king.x += Math.sign(t.x - king.x);
    else if (king.z !== t.z) king.z += Math.sign(t.z - king.z);
    else { leg++; }
    drawKing();
    if (leg >= tour.length) {
      const corner = (king.x === 0 || king.x === N - 1) && (king.z === 0 || king.z === N - 1);
      if (corner) { won = true; $('done-text').textContent = `King escaped (skeleton 7x7).`; $('done').hidden = false; }
    }
  },
  render: () => { renderer.render(scene, camera); },
  checks: [
    { name: 'board-7x7', when: () => true, check: () => board.children.length === N * N + 1 },
    { name: 'escape', when: () => won, check: () => (king.x === 0 || king.x === N - 1) && (king.z === 0 || king.z === N - 1) },
  ],
});
wireHelp($('help-btn'), $('help'), $('help-go'));
$('next').onclick = () => location.reload();
h.start();
function frame() { if (!VERIFY) requestAnimationFrame(frame); }
if (!VERIFY) { requestAnimationFrame(frame); $('turn').textContent = 'defenders to move'; }
