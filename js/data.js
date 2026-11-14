/* Stellar Dominion — static game data: stars, planets, ships, buildings, techs, empires */
window.SS = window.SS || {};
(function () {
  const U = SS.util;
  const D = (SS.data = {});

  /* ---------------- star classes ---------------- */
  D.STAR_CLASSES = [
    { key: 'O', name: 'Blue Giant',        color: '#9bb0ff', radius: 1.55 },
    { key: 'B', name: 'Blue-White Star',   color: '#aabfff', radius: 1.45 },
    { key: 'A', name: 'White Star',        color: '#cad8ff', radius: 1.35 },
    { key: 'F', name: 'Yellow-White Star', color: '#f8f7ff', radius: 1.25 },
    { key: 'G', name: 'Yellow Star',       color: '#fff4e0', radius: 1.15 },
    { key: 'K', name: 'Orange Star',       color: '#ffd2a1', radius: 1.05 },
    { key: 'M', name: 'Red Dwarf',         color: '#ffcc6f', radius: 0.95 },
  ];

  /* ---------------- planet types ---------------- */
  D.PLANET_TYPES = {
    terran:   { name: 'Terran',    color: '#4caf50', habitability: 1.0,  hostile: false },
    ocean:    { name: 'Ocean',     color: '#2196f3', habitability: 0.9,  hostile: false },
    desert:   { name: 'Desert',    color: '#ffb74d', habitability: 0.7,  hostile: false },
    ice:      { name: 'Ice',       color: '#b3e5fc', habitability: 0.5,  hostile: false },
    barren:   { name: 'Barren',    color: '#9e9e9e', habitability: 0.3,  hostile: false },
    toxic:    { name: 'Toxic',     color: '#9ccc65', habitability: 0.2,  hostile: true  },
    volcanic: { name: 'Volcanic',  color: '#ff7043', habitability: 0.15, hostile: true  },
    gas:      { name: 'Gas Giant', color: '#ce93d8', habitability: 0.0,  hostile: true  },
  };

  /* ---------------- ships ---------------- */
  D.SHIPS = {
    scout: {
      name: 'Scout', hp: 25, attack: 5, speed: 420,
      credits: 40, minerals: 20, buildTurns: 1,
      tech: null, colonize: false, desc: 'Fast exploration vessel',
    },
    colony: {
      name: 'Colony Ship', hp: 40, attack: 0, speed: 240,
      credits: 140, minerals: 90, buildTurns: 3,
      tech: null, colonize: true, desc: 'Founds new colonies',
    },
    frigate: {
      name: 'Frigate', hp: 55, attack: 12, speed: 320,
      credits: 80, minerals: 45, buildTurns: 2,
      tech: null, colonize: false, desc: 'Light combat ship',
    },
    destroyer: {
      name: 'Destroyer', hp: 110, attack: 24, speed: 270,
      credits: 160, minerals: 100, buildTurns: 3,
      tech: 'construction1', colonize: false, desc: 'Medium combat ship',
    },
    cruiser: {
      name: 'Cruiser', hp: 200, attack: 42, speed: 230,
      credits: 300, minerals: 200, buildTurns: 4,
      tech: 'construction2', colonize: false, desc: 'Heavy combat ship',
    },
    battleship: {
      name: 'Battleship', hp: 380, attack: 75, speed: 190,
      credits: 560, minerals: 380, buildTurns: 6,
      tech: 'construction3', colonize: false, desc: 'Capital ship',
    },
  };

  /* ---------------- buildings ---------------- */
  D.BUILDINGS = {
    mine:     { name: 'Mine',             cost: 60,  repeat: true,  desc: '+3 minerals/turn (scaled by richness)' },
    market:   { name: 'Market',           cost: 60,  repeat: true,  desc: '+4 credits/turn' },
    lab:      { name: 'Research Lab',     cost: 60,  repeat: true,  desc: '+5 research/turn' },
    habitat:  { name: 'Habitat',          cost: 80,  repeat: true,  desc: '+50% population growth' },
    shipyard: { name: 'Shipyard',         cost: 120, repeat: false, desc: 'Enables ship construction' },
    shield:   { name: 'Planetary Shield', cost: 100, repeat: false, desc: 'Defense platforms protect the colony' },
  };

  /* ---------------- technology tree ---------------- */
  D.TECHS = {
    // Weapons
    weapons1: { name: 'Laser Cannons I',   cat: 'Weapons',    cost: 30,  requires: [],             desc: '+15% fleet attack' },
    weapons2: { name: 'Laser Cannons II',  cat: 'Weapons',    cost: 55,  requires: ['weapons1'],   desc: '+15% fleet attack' },
    weapons3: { name: 'Plasma Cannons',    cat: 'Weapons',    cost: 90,  requires: ['weapons2'],   desc: '+25% fleet attack' },
    // Defense
    shields1: { name: 'Deflector Shields I',   cat: 'Defense', cost: 30,  requires: [],              desc: '+15% ship hit points' },
    shields2: { name: 'Deflector Shields II',  cat: 'Defense', cost: 55,  requires: ['shields1'],   desc: '+15% ship hit points' },
    shields3: { name: 'Advanced Shields',      cat: 'Defense', cost: 90,  requires: ['shields2'],   desc: '+25% ship hit points' },
    // Propulsion
    engines1: { name: 'Ion Engines I',   cat: 'Propulsion', cost: 30,  requires: [],              desc: '+40 fleet speed' },
    engines2: { name: 'Ion Engines II',  cat: 'Propulsion', cost: 55,  requires: ['engines1'],   desc: '+40 fleet speed' },
    engines3: { name: 'Warp Drive',      cat: 'Propulsion', cost: 90,  requires: ['engines2'],   desc: '+80 fleet speed' },
    // Industry
    construction1: { name: 'Advanced Construction', cat: 'Industry', cost: 35,  requires: [],                        desc: 'Unlock Destroyer, -10% ship cost' },
    construction2: { name: 'Capital Shipyard',      cat: 'Industry', cost: 70,  requires: ['construction1'],        desc: 'Unlock Cruiser' },
    construction3: { name: 'Titan Foundries',       cat: 'Industry', cost: 120, requires: ['construction2'],        desc: 'Unlock Battleship' },
    // Expansion
    colonization1: { name: 'Terraforming',          cat: 'Expansion', cost: 35,  requires: [],                        desc: '+3 max population in all colonies' },
    colonization2: { name: 'Xeno Colonization',     cat: 'Expansion', cost: 60,  requires: ['colonization1'],        desc: 'Colonize hostile worlds (Toxic / Volcanic)' },
    colonization3: { name: 'Galactic Colonization', cat: 'Expansion', cost: 100, requires: ['colonization2'],        desc: '+4 max population, -25% colony ship cost' },
  };
  D.TECH_CATS = ['Weapons', 'Defense', 'Propulsion', 'Industry', 'Expansion'];

  /* ---------------- empires ---------------- */
  D.EMPIRE_DEFS = [
    { name: 'Terran Consortium',  color: '#4dd0e1', desc: "Humanity's united frontier command" },
    { name: 'Vex Imperium',       color: '#ef5350', desc: 'Expansionist machine intelligence' },
    { name: 'Aurelian Dominion',  color: '#ffd54f', desc: 'Ancient noble houses' },
    { name: 'Krell Syndicate',    color: '#ab47bc', desc: 'Ruthless trade cartel' },
    { name: 'Nova Collective',    color: '#81c784', desc: 'Hive-minded flora' },
  ];

  /* ---------------- anomalies ---------------- */
  // One-time rewards granted to the first empire that scouts a system by fleet.
  D.ANOMALIES = [
    { name: 'Derelict Freighter', desc: 'A pre-collapse hauler, intact enough to strip.', reward: { credits: 120 } },
    { name: 'Abandoned Mine',     desc: 'A rich mineral seam left by an unknown civilisation.', reward: { minerals: 150 } },
    { name: 'Ancient Data Core',  desc: 'Research data of immense value.', reward: { research: 80 } },
    { name: 'Ice Comet',          desc: 'Water and volatiles ready to exploit.', reward: { credits: 60, minerals: 70 } },
    { name: 'Strange Signal',     desc: 'Unusual readings from a nearby source.', reward: { research: 50, credits: 60 } },
    { name: 'Pirate Cache',       desc: 'A hidden stash of plundered goods.', reward: { credits: 100, minerals: 80 } },
  ];

  /* ---------------- name generator ---------------- */
  const PRE = ['Al', 'Be', 'Cas', 'De', 'Er', 'Fom', 'Gal', 'Hy', 'Io', 'Ja', 'Ka', 'Ly',
    'Me', 'Nu', 'Om', 'Pro', 'Qu', 'Rho', 'Sig', 'Tau', 'Ul', 'Veg', 'Wol', 'Xi', 'Ygg',
    'Zet', 'Alph', 'Bet', 'Gamm', 'Delt', 'Eps', 'Thet', 'Iot', 'Kap', 'Lam', 'Nix', 'Oxi',
    'Rig', 'Sir', 'Ups', 'Phi', 'Chi', 'Psi', 'Ome', 'Vex', 'Krell', 'Aur', 'Nym', 'Zor'];
  const SUF = ['ara', 'eon', 'is', 'os', 'us', 'ar', 'en', 'or', 'ax', 'ix', 'on', ' Prime',
    ' Major', ' Minor', ' Nova', ' Cluster', ' Drift', ' Reach', ' Gate', ' Haven', ' Fall',
    ' Rise', ' Deep', ' Span', ' Veil', ' Expanse', ' Hollow', ' Crown', ' Shard', ' Rest'];
  D.systemName = function (used) {
    for (let i = 0; i < 100; i++) {
      const n = U.choice(PRE) + U.choice(SUF);
      if (!used.has(n)) { used.add(n); return n; }
    }
    return 'System ' + U.randInt(100, 999);
  };
})();
