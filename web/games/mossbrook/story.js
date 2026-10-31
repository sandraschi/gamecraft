// The Quiet Well: Mossbrook's story, its three-flame lantern, and the stir meter.
// Pure game logic over the world; main.js owns rendering and input.
//
// Every lantern in Mossbrook is lit from the glow that rises from the old well at
// dusk. Tonight it did not rise. Captain Osk blames the troll. The truth is older.

export const FLAMES = {
  ember: { name: 'Ember', color: 0xff8a3a, css: '#ff8a3a', desc: 'Hearth fire. Burns what is in the way.' },
  glimmer: { name: 'Glimmer', color: 0x5ac8ff, css: '#5ac8ff', desc: 'Crystal light. Shows what was hidden.' },
  hush: { name: 'Hush', color: 0xb08aff, css: '#b08aff', desc: 'Moonring quiet. Sleepers stay asleep.' },
};
export const HEARTHS = { kettle: 'the Copper Kettle', forge: "Bertil's forge", oven: "Pim's oven" };
const TROLL = { x: -67, z: -11 };
const STIR_RADIUS = 16;

// Stage list: objective text and where the compass points.
const STAGES = [
  { id: 'mags', text: 'Ask Mags at the Copper Kettle what happened to the well', target: { npc: 'Mags' } },
  { id: 'wren', text: 'Find Sister Wren at the Lantern Archive', target: { npc: 'Sister Wren' } },
  { id: 'cellars', text: 'Search the cellars under the Copper Kettle', target: { x: 2.25, z: -8, space: 'below' } },
  { id: 'forge-cellar', text: 'Follow the tunnel to the forge cellar and see what Bertil keeps there', target: { x: 24.4, z: 6.4, space: 'below' } },
  { id: 'glyph', text: 'Hold up Glimmer (Q) in the tunnel beneath the well and look for a hidden way', target: { x: 5.8, z: 2.3, space: 'below' } },
  { id: 'cradle', text: 'See what lies beneath the well', target: { x: 7.5, z: -3.2, space: 'below' } },
  { id: 'bertil', text: 'Ask Bertil about the crystals', target: { npc: 'Bertil' } },
  { id: 'moonring', text: 'Draw the Hush from the Moonring, south of the village', target: { x: -8, z: 72, space: 'outdoor' } },
  { id: 'roots', text: 'Slip past Old Knuckles with Hush, and find what he sleeps in front of', target: { x: -64.5, z: 5.8, space: 'cave' } },
  { id: 'tablet', text: 'Explore the hollow behind the roots', target: { x: -64.5, z: 16.9, space: 'cave' } },
  { id: 'hearths', text: 'Light your lantern at three hearths: the Copper Kettle, the forge, the bakery oven', target: { hearth: true } },
  { id: 'wake', text: 'Wake Old Knuckles with a lantern of three hearths, and a song', target: { x: -66, z: -11, space: 'cave' } },
  { id: 'set', text: "Set the heart crystal in the Keeper's cradle beneath the well", target: { x: 7.5, z: -3.2, space: 'below' } },
  { id: 'end', text: 'The well is lit. Mossbrook is yours to wander.', target: null },
];
const HEARTH_SPOTS = {
  kettle: { x: 7.45, z: -13, space: 'outdoor', range: 2.4 },
  forge: { x: 17.5, z: 0.75, space: 'outdoor', range: 2.8 },
  oven: { x: -9.6, z: 19.4, space: 'outdoor', range: 2.4 },
};

// Dialogue that changes as the story moves. Keys: npc name -> stage id -> lines.
// The first stage at or before the current one wins.
const TALK = {
  Mags: {
    mags: ['The well went dark at dusk. Every lantern in the house is cold, and so is the stew.',
      'Osk is out sharpening his spear for the troll. Before somebody does something stupid, find out what really happened.',
      'Start with Sister Wren at the Archive. If anyone knows how the well works, it is her and her old paper.'],
    wren: ['Wren is at the Archive, past the green. Go on.'],
    hearths: ['Light your lantern at my fire? For the troll? ...Take it. Take the good poker too.'],
    end: ['The stew is hot again. On the house. Do not tell Odo.'],
  },
  'Sister Wren': {
    wren: ['The well is not a well. The founding ledger calls it the Keeper\'s Cradle, and it sits in a cellar this village does not admit it has.',
      'Someone tore out the pages about the compact. What is left mentions "the light lent by the Crag".',
      'Your lantern still holds an Ember from Mags\' hearth. A lantern can carry three flames, if you know where to fetch them. Try the cellars.'],
    end: ['I have rewritten the torn pages from the tablet. Nobody will forget the compact again. I will see to it personally.'],
  },
  Bertil: {
    bertil: ['...You found the cradle. Then you know.',
      'I am not stealing that crystal. I am carrying it. Six years, every month, up the crag and back through the tunnel, because the well has been dying since before you were born.',
      'Tonight the last shard ran out, and I could not get past him. He sleeps lighter every year.',
      'The heart crystals grow behind him. I only ever got past with the Hush, from the Moonring south of the village. Stand in the ring and it comes to you.'],
    hearths: ['My forge? Of course. Hold the lantern close. Closer. There.'],
    end: ['Six years of hauling rocks, and all it needed was a lantern and a terrible song. I need a drink.'],
  },
  'Captain Osk': {
    mags: ['The troll took the light. Mark my words. I have marked them myself, twice.'],
    tablet: ['A compact? With the troll? ...Nobody told the watch.'],
    end: ['Turns out the watch was guarding the wrong side of the road for forty years.'],
  },
  Corwin: {
    mags: ['Dark well. Last time that happened I was a boy. We walked up the crag with lanterns, and somebody sang. Badly.', 'Funny, I had forgotten that until just now.'],
    end: ['Lanterns and a bad song. I told you. Did I tell you? I meant to.'],
  },
  Ysolde: {
    mags: ['The draught in my room comes up from under the green, near the well. My compass spins near the smithy. Make of that what you will.'],
    end: ['I am adding a hollow behind the troll to my map. Labelled: do not wake, unless singing.'],
  },
  'Old Wenzel': {
    mags: ['Lights on the bridge at night. Blue ones. Somebody big carries them, every new moon, from the crag road to the smithy.', 'I mind my nets. Nets do not ask questions.'],
    end: ['The blue lights stopped. Now there is a gold one in the middle of the village. Good for fishing.'],
  },
};

export function createStory(api) {
  const { world } = api;
  const st = {
    stage: 0,
    flames: { ember: true, glimmer: false, hush: false },
    flame: 'ember',
    hearths: new Set(),
    items: new Set(),
    stir: 0, tossed: 0,
    trollAwake: false,
    ended: false,
    journal: ['Lantern Eve. The well did not light at dusk.'],
  };
  const sid = () => STAGES[st.stage].id;
  const atLeast = (id) => st.stage >= STAGES.findIndex((s) => s.id === id);
  function advance(toId, note) {
    const i = STAGES.findIndex((s) => s.id === toId);
    if (i > st.stage) {
      st.stage = i;
      if (note) st.journal.push(note);
      api.mark(`story ${toId}`);
      api.toast(STAGES[i].text);
    }
  }
  function learn(flame) {
    if (st.flames[flame]) return;
    st.flames[flame] = true;
    st.flame = flame;
    api.toast(`New flame: ${FLAMES[flame].name}. ${FLAMES[flame].desc} Q cycles flames.`);
  }

  // World objects the player can use. enabled() hides them from the prompt.
  const objects = [
    { id: 'note', name: 'A damp note', verb: 'Read', x: 2.25, z: -8, space: 'below', range: 1.8,
      use: () => { advance('forge-cellar', 'A note in the tunnel: "Third barrel from the left is not wine. Bertil knows."');
        return ['"Third barrel from the left is not wine. Bertil knows. Mags does not need to."', 'The tunnel runs on, past the well, towards the forge.']; } },
    { id: 'crates', name: 'Crates packed in coal', verb: 'Search', x: 24.4, z: 6.4, space: 'below', range: 2.6,
      use: () => { learn('glimmer'); advance('glyph', 'Bertil\'s cellar: crates of blue cave crystal, hidden in coal.');
        return ['Under the coal: crystal, blue and humming, the kind that grows on the crag.', 'You hold your lantern close, and it drinks a little of the light. Glimmer.']; } },
    { id: 'glyph', name: 'A mark on the wall', verb: 'Touch', x: 5.8, z: 2.3, space: 'below', range: 2.0,
      enabled: () => st.flame === 'glimmer' && !st.items.has('door-open'),
      use: () => { st.items.add('door-open'); world.col.remove('below', world.story.falseWall.col); world.story.falseWall.mesh.visible = false;
        advance('cradle', 'Glimmer showed a glyph in the tunnel wall. The wall was not a wall.');
        return ['Under Glimmer the stones show a glyph: a lantern over a crag. You press it, and the wall folds away like a curtain.']; } },
    { id: 'cradle', name: "The Keeper's cradle", verb: 'Examine', x: 7.5, z: -3.2, space: 'below', range: 2.0,
      use: () => {
        if (st.items.has('heart')) {
          st.items.delete('heart'); world.story.cradle.heart.visible = true; world.story.cradle.splinter.visible = false;
          advance('end', 'The heart crystal sits in the cradle. The well is lit.');
          st.ended = true;
          return ['The crystal settles into the cradle as if it remembered the shape. Light runs up the brass pipes, and somewhere above, the whole village gasps.'];
        }
        if (atLeast('cradle') && !atLeast('bertil')) advance('bertil', 'Beneath the well: a brass cradle, empty but for a dim splinter. Bertil\'s tools lie beside it.');
        return ['A brass cradle wired into pipes that climb into the well shaft. It holds only a dim splinter of crystal.', 'Beside it: a smith\'s tongs, a coal sack, and a tally of six years of new moons.'];
      } },
    { id: 'altar', name: 'The Moonring altar', verb: 'Stand in', x: -8, z: 72, space: 'outdoor', range: 2.8,
      use: () => {
        if (!atLeast('moonring')) return ['The stones hum, very softly. Nothing here needs you yet.'];
        learn('hush'); advance('roots', 'The Moonring gave the Hush. Sleepers stay asleep around it.');
        return ['You stand in the ring. The stones hum, then the hum stops, and the quiet pours into your lantern. Hush.'];
      } },
    { id: 'roots', name: 'A curtain of roots', verb: 'Burn', x: -64.5, z: 5.8, space: 'cave', range: 2.6,
      enabled: () => !st.items.has('roots-burnt'),
      use: () => {
        if (st.flame !== 'ember') return ['Thick roots, wet with cave water. A flame would get through them. The right one.'];
        st.items.add('roots-burnt'); world.col.remove('cave', world.story.roots.col); world.story.roots.group.visible = false;
        advance('tablet', 'Ember burned through the roots behind the troll.');
        return ['Ember catches the roots and they curl back from the light with a hiss, as if they had been waiting.'];
      } },
    { id: 'tablet', name: 'A stone tablet', verb: 'Read', x: -64.5, z: 16.9, space: 'cave', range: 2.6,
      use: () => {
        advance('hearths', 'The tablet: the Crag lends its light to the well. Each Lantern Eve the village brings the Keeper a lantern lit from every hearth, and a song.');
        return ['"The Crag lends its light to the well of Mossbrook, so long as the Keeper is remembered.',
          'Each Lantern Eve the village shall climb to the Keeper with a lantern lit from every hearth, and sing to him, however badly."',
          'Old Knuckles is not guarding the crystal. He is the Keeper. And nobody has come in forty years.'];
      } },
    ...Object.entries(HEARTH_SPOTS).map(([key, h]) => ({
      id: `hearth-${key}`, name: `The hearth of ${HEARTHS[key]}`, verb: 'Light lantern at', ...h,
      enabled: () => sid() === 'hearths' && !st.hearths.has(key),
      use: () => {
        st.hearths.add(key);
        api.toast(`Lantern lit at ${HEARTHS[key]} (${st.hearths.size}/3).`);
        if (st.hearths.size === 3) advance('wake', 'The lantern carries fire from three hearths.');
        return [`You hold the lantern to ${HEARTHS[key]}. The Ember takes on a new colour, warmer.`];
      },
    })),
    { id: 'troll', name: 'Old Knuckles', verb: 'Wake', x: -66, z: -11, space: 'cave', range: 6,
      use: () => {
        if (st.trollAwake) return ['"Next year, bring the baker. Bakers sing in tune."'];
        if (sid() !== 'wake') return ['He snores like a rockslide in slow motion.', 'A smell of old cheese and wet stone. One eye twitches. Best not.'];
        st.trollAwake = true; st.items.add('heart');
        world.story.heart.mesh.visible = false; world.story.heart.light.intensity = 0;
        api.wakeTroll();
        advance('set', 'Old Knuckles woke to the lantern and Odo\'s song. He gave the heart crystal freely.');
        return ['Odo clears his throat and sings. It is dreadful. It is the worst thing the cave has ever heard.',
          'An eye the size of a cartwheel opens. "...A lantern. And a song. Forty years. I thought you had all died."',
          '"The smith has been carrying my light down in sacks. Tell him he may stop. Take this, and put it where it belongs."',
          'He holds out the heart crystal, warm as bread.'];
      } },
  ];

  return {
    st, STAGES, FLAMES,
    objects,
    objectiveText() {
      const s = STAGES[st.stage];
      if (s.id === 'hearths') return `${s.text} (${st.hearths.size}/3)`;
      return s.text;
    },
    objectiveTarget(npcs) {
      const t = STAGES[st.stage].target;
      if (!t) return null;
      if (t.npc) { const n = npcs.find((o) => o.name === t.npc); return n ? { x: n.x, z: n.z, space: n.space } : null; }
      if (t.hearth) { const k = Object.keys(HEARTH_SPOTS).find((h) => !st.hearths.has(h)); return k ? HEARTH_SPOTS[k] : null; }
      return t;
    },
    // returns lines to show, or null to fall back to the NPC's default lines
    talk(name) {
      if (name === 'Mags' && sid() === 'mags') { const lines = TALK.Mags.mags; advance('wren', 'Mags: the well went dark at dusk. Osk blames the troll.'); return lines; }
      if (name === 'Sister Wren' && sid() === 'wren') { const lines = TALK['Sister Wren'].wren; advance('cellars', 'Wren: the well is the Keeper\'s Cradle, in a cellar the village does not admit to. Pages about "the compact" are torn out.'); return lines; }
      if (name === 'Bertil' && sid() === 'bertil') { const lines = TALK.Bertil.bertil; advance('moonring', 'Bertil has carried crystal to the well for six years. The heart crystals grow behind the troll. Hush comes from the Moonring.'); return lines; }
      const table = TALK[name];
      if (!table) return null;
      for (let i = st.stage; i >= 0; i--) if (table[STAGES[i].id]) return table[STAGES[i].id];
      return null;
    },
    cycleFlame() {
      const known = Object.keys(FLAMES).filter((k) => st.flames[k]);
      st.flame = known[(known.indexOf(st.flame) + 1) % known.length];
      return st.flame;
    },
    setFlame(k) { if (st.flames[k]) st.flame = k; return st.flame; },
    // stir meter: walking near the sleeping Keeper without Hush wakes him, briefly and badly
    update(dt, player) {
      for (const m of world.glimmer) m.visible = st.flame === 'glimmer' && player.space === 'below' && Math.hypot(player.x - m.position.x, player.z - m.position.z) < 10 && !st.items.has('door-open');
      if (st.trollAwake || player.space !== 'cave') { st.stir = Math.max(0, st.stir - dt * 0.3); return null; }
      const d = Math.hypot(player.x - TROLL.x, player.z - TROLL.z);
      const running = player.speed > 5;
      if (d < STIR_RADIUS) {
        const near = 1 - d / STIR_RADIUS;
        const rate = st.flame === 'hush' ? (running ? 0.12 : 0) : (0.15 + 0.6 * near) * (running ? 1.6 : 1);
        st.stir += rate * dt;
        if (rate === 0) st.stir = Math.max(0, st.stir - dt * 0.2);
      } else st.stir = Math.max(0, st.stir - dt * 0.2);
      if (st.stir >= 1) { st.stir = 0; st.tossed++; return 'toss'; }
      return null;
    },
  };
}

// The scripted playthrough the verifier drives. Steps: {to}, {use: objectId}, {talk: npcName},
// {flame}, {walk: true|false}. Checkpoints assert state, not just position.
export function tour(s) {
  const out = (x, z) => ({ to: [x, z] });
  return [
    out(0, -8.5), out(0, -11.5), out(1, -18.4),
    { talk: 'Mags', checkpoint: 'mags', check: (g) => g.stage === 'wren' && g.inside('tavern') && !g.roofVisible('tavern') },
    out(0, -11.5), out(0, -8.5), out(10, -6), out(24, -14.6), out(24, -16.5),
    { talk: 'Sister Wren', checkpoint: 'wren', check: (g) => g.stage === 'cellars' },
    out(24, -14.6), out(10, -6), out(0, -8.5), out(0, -11.5), out(1, -16.5), out(5.4, -16.8), out(6.85, -16.8), out(6.85, -23),
    { checkpoint: 'cellar', check: (g) => g.space === 'below' && g.layer === 'below' },
    out(2.25, -22.5), out(2.25, -8.6),
    { use: 'note', checkpoint: 'note', check: (g) => g.stage === 'forge-cellar' },
    out(2.25, 3.2), out(17.8, 3.2), out(22.6, 4.6),
    { use: 'crates', checkpoint: 'glimmer', check: (g) => g.flames.glimmer && g.stage === 'glyph' },
    out(17.8, 3.2), out(5.8, 3.1), { flame: 'glimmer' },
    { use: 'glyph', checkpoint: 'hidden-door', check: (g) => g.stage === 'cradle' && g.canStand('below', 5.8, 1.4) },
    out(5.8, 1.4), out(6.6, -1.6),
    { use: 'cradle', checkpoint: 'keepers-cellar', check: (g) => g.stage === 'bertil' && g.roomName === "The Keeper's Cellar" },
    out(5.8, 1.4), out(5.8, 3.1), out(17.8, 3.2), out(19.5, 0.2), out(25.8, 0.2), out(25.8, 4.0), out(22.5, 6.0),
    { talk: 'Bertil', checkpoint: 'bertil', check: (g) => g.stage === 'moonring' && g.space === 'outdoor' },
    out(22, 9.6), out(22, 12), out(14, 12), out(2, 16), out(0, 28.5), out(-6, 28.5), out(-17, 38), out(-8, 62), out(-8, 69.6),
    { use: 'altar', checkpoint: 'moonring', check: (g) => g.flames.hush && g.stage === 'roots' },
    out(-8, 62), out(-17, 38), out(-6, 28.5), out(0, 28.5), out(2, 16), out(4.5, 3.5), out(-48.8, -6),
    { flame: 'hush' }, { walk: true },
    out(-57.5, -6), out(-58.5, 0), out(-64.5, 3.6),
    { flame: 'ember' },
    { use: 'roots', checkpoint: 'roots', check: (g) => g.stage === 'tablet' && g.tossed === 0 && g.layer === 'cave' },
    out(-64.5, 11.5), out(-64.5, 14.2),
    { use: 'tablet', checkpoint: 'compact', check: (g) => g.stage === 'hearths' },
    { flame: 'hush' }, out(-64.5, 3.6), out(-58.5, 0), out(-57.5, -6), { walk: false }, out(-47, -6),
    out(4.5, 3.5), out(1.5, 3), out(0, -8.5), out(0, -11.5), out(1, -16), out(6.4, -14.6),
    { use: 'hearth-kettle' },
    out(1, -16), out(0, -11.5), out(0, -8.5), out(1.5, 3), out(14, 12), out(22, 12), out(22, 9.6), out(22, 6.5), out(19.3, 2.6),
    { use: 'hearth-forge' },
    out(22, 6.5), out(22, 9.6), out(22, 12), out(14, 12), out(2, 16), out(0, 28.5), out(-6, 28.5), out(-6, 26.4), out(-6, 24.7), out(-8.2, 21.2),
    { use: 'hearth-oven', checkpoint: 'three-hearths', check: (g) => g.stage === 'wake' && g.hearths === 3 },
    out(-6, 24.7), out(-6, 26.4), out(-6, 28.5), out(0, 28.5), out(2, 16), out(4.5, 3.5), out(-48.8, -6),
    { flame: 'hush' }, { walk: true }, out(-57.5, -6), out(-61.5, -9.5),
    { use: 'troll', checkpoint: 'keeper-wakes', check: (g) => g.stage === 'set' && g.trollAwake && g.tossed === 0 },
    { walk: false }, out(-57.5, -6), out(-47, -6),
    out(4.5, 3.5), out(1.5, 3), out(0, -8.5), out(0, -11.5), out(1, -16), out(5.4, -16.8), out(6.85, -16.8), out(6.85, -23),
    out(2.25, -22.5), out(2.25, 3.2), out(5.8, 3.1), out(5.8, 1.4), out(6.6, -1.6),
    { use: 'cradle', checkpoint: 'well-lit', check: (g) => g.stage === 'end' && g.ended, win: true },
  ];
}
