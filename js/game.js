/* Stellar Dominion — core game state, rules, economy, movement, combat, diplomacy */
window.SS = window.SS || {};
(function () {
  const U = SS.util;
  const D = SS.data;
  const G = (SS.game = { state: null });

  const SAVE_KEY = 'stellar_dominion_save_v1';
  const SENSOR_RANGE = 360;
  G.SENSOR_RANGE = SENSOR_RANGE;

  /* ============================================================
   * Lookup helpers
   * ============================================================ */
  G.getSystem = (id) => G.state.systems.find((s) => s.id === id);
  G.getEmpire = (id) => G.state.empires.find((e) => e.id === id);
  G.getFleet = (id) => G.state.fleets.find((f) => f.id === id);
  G.fleetsAt = (systemId) => G.state.fleets.filter((f) => f.systemId === systemId);
  G.empireFleets = (eid) => G.state.fleets.filter((f) => f.empireId === eid);
  G.empireColonies = (eid) =>
    G.state.systems.filter((s) => s.planet.colony && s.planet.colony.empireId === eid);
  G.empireSystems = (eid) => G.state.systems.filter((s) => s.ownerEmpireId === eid);
  G.player = () => G.state.empires[0];
  G.slots = (sys) => sys.planet.size + 2; // small 3, medium 4, large 5
  G.isExplored = (sys, eid) => sys.exploredBy.has(eid);
  G.atWar = (a, b) => {
    const ea = G.getEmpire(a);
    const eb = G.getEmpire(b);
    return !!(ea && eb && (ea.atWarWith.has(b) || eb.atWarWith.has(a)));
  };

  /* ============================================================
   * Tech modifiers & effective ship stats
   * ============================================================ */
  G.techModifiers = function (empire) {
    const t = empire.techs;
    const m = {
      attackMul: 1,
      hpMul: 1,
      speedBonus: 0,
      costMul: 1,
      maxPopBonus: 0,
      canHostile: false,
      colonyCostMul: 1,
    };
    if (t.weapons1) m.attackMul *= 1.15;
    if (t.weapons2) m.attackMul *= 1.15;
    if (t.weapons3) m.attackMul *= 1.25;
    if (t.shields1) m.hpMul *= 1.15;
    if (t.shields2) m.hpMul *= 1.15;
    if (t.shields3) m.hpMul *= 1.25;
    if (t.engines1) m.speedBonus += 40;
    if (t.engines2) m.speedBonus += 40;
    if (t.engines3) m.speedBonus += 80;
    if (t.construction1) m.costMul *= 0.9;
    if (t.colonization1) m.maxPopBonus += 3;
    if (t.colonization2) m.canHostile = true;
    if (t.colonization3) { m.maxPopBonus += 4; m.colonyCostMul *= 0.75; }
    return m;
  };

  G.effectiveShipStats = function (empire, cls) {
    const base = D.SHIPS[cls];
    const m = G.techModifiers(empire);
    return {
      hp: Math.round(base.hp * m.hpMul),
      attack: Math.round(base.attack * m.attackMul),
      speed: base.speed + m.speedBonus,
    };
  };

  G.shipCost = function (empire, cls) {
    const base = D.SHIPS[cls];
    const m = G.techModifiers(empire);
    const mul = cls === 'colony' ? m.colonyCostMul : m.costMul;
    return {
      credits: Math.round(base.credits * mul),
      minerals: Math.round(base.minerals * mul),
    };
  };

  G.fleetSpeed = function (fleet) {
    const e = G.getEmpire(fleet.empireId);
    if (!fleet.ships.length) return 100;
    return Math.min(...fleet.ships.map((s) => G.effectiveShipStats(e, s.cls).speed));
  };

  G.fleetStrength = function (fleet) {
    const e = G.getEmpire(fleet.empireId);
    return fleet.ships.reduce((sum, s) => {
      const st = G.effectiveShipStats(e, s.cls);
      return sum + st.attack * 2 + s.hp * 0.5;
    }, 0);
  };

  G.empireStrength = (eid) =>
    G.empireFleets(eid).reduce((a, f) => a + G.fleetStrength(f), 0);

  G.empireShipCount = (eid) =>
    G.empireFleets(eid).reduce((a, f) => a + f.ships.length, 0);

  G.maxPop = (sys, empire) =>
    sys.planet.maxPop + (empire ? G.techModifiers(empire).maxPopBonus : 0);

  /* ============================================================
   * Colony economy
   * ============================================================ */
  G.colonyProduction = function (sys, col, empire) {
    const b = col.buildings;
    const count = (k) => b.filter((x) => x === k).length;
    const minerals = 2 + count('mine') * 3 * sys.planet.richness;
    const credits = 2 + count('market') * 4 + col.population * 0.8;
    const research = 1 + count('lab') * 5 + col.population * 0.3;
    return { minerals, credits, research };
  };

  G.colonyGrowthRate = (col) => 0.05 * (col.buildings.includes('habitat') ? 1.5 : 1);

  G.canColonize = function (empire, sys) {
    const pt = D.PLANET_TYPES[sys.planet.type];
    if (!pt || pt.habitability <= 0) return false;
    if (pt.hostile && !G.techModifiers(empire).canHostile) return false;
    return true;
  };

  /* ============================================================
   * New game
   * ============================================================ */
  G.newGame = function (opts) {
    const numAI = opts.numAI != null ? opts.numAI : 3;
    const numSystems = opts.numSystems || 46;
    const difficulty = opts.difficulty || 'normal';

    const { systems, homes } = SS.galaxy.generate({ numSystems, numEmpires: numAI + 1 });

    const empires = [];
    for (let i = 0; i < numAI + 1; i++) {
      const def = D.EMPIRE_DEFS[i % D.EMPIRE_DEFS.length];
      empires.push({
        id: i,
        name: opts.playerName && i === 0 ? opts.playerName : def.name,
        color: def.color,
        desc: def.desc,
        isPlayer: i === 0,
        credits: 250,
        minerals: 200,
        techProgress: 0,
        researchPool: 0,
        techs: {},
        currentTech: null,
        homeSystemId: null,
        alive: true,
        atWarWith: new Set(),
        difficulty,
        aggression: U.rand(0.35, 0.9),
        lastIncome: { minerals: 0, credits: 0, research: 0 },
        peaceOffers: [], // empire ids offering peace to this empire
      });
    }

    G.state = {
      turn: 1,
      systems,
      empires,
      fleets: [],
      logs: [],
      battles: [],
      selected: null, // {type:'system'|'fleet', id}
      hover: null,
      over: false,
      winner: null,
      difficulty,
    };

    // Scatter anomalies on unclaimed worlds (homes are exempt)
    for (const s of G.state.systems) {
      if (s.ownerEmpireId === null && Math.random() < 0.28) {
        s.anomaly = U.choice(D.ANOMALIES);
      }
    }

    empires.forEach((e, i) => {
      const home = homes[i];
      e.homeSystemId = home.id;
      SS.galaxy.makeHomeSystem(home);
      home.ownerEmpireId = e.id;
      home.planet.colony = {
        empireId: e.id,
        population: 8,
        buildings: ['mine', 'market', 'lab', 'shipyard'],
        queue: [],
        queueProgress: 0,
      };
      home.exploredBy.add(e.id);

      const fleet = makeFleet(e.id, home.id);
      addShipToFleet(fleet, e, 'colony');
      addShipToFleet(fleet, e, 'frigate');
      addShipToFleet(fleet, e, 'scout');
      G.state.fleets.push(fleet);
    });

    G.updateExploration();
    G.log('Your empire rises. Expand, research, and conquer.', 'info');
    G.log('Build ships at colonies with a Shipyard. Select a fleet and click a destination to move.', 'info');
    return G.state;
  };

  function makeFleet(empireId, systemId) {
    const sys = G.getSystem(systemId);
    return {
      id: U.uid(),
      empireId,
      systemId,
      x: sys ? sys.x : 0,
      y: sys ? sys.y : 0,
      ships: [],
      moves: null, // {toSystemId, turnsLeft, totalTurns}
    };
  }

  function addShipToFleet(fleet, empire, cls) {
    const st = G.effectiveShipStats(empire, cls);
    fleet.ships.push({ cls, hp: st.hp });
  }

  /* ============================================================
   * Logging
   * ============================================================ */
  G.log = function (msg, type) {
    const g = G.state;
    if (!g) return;
    g.logs.push({ turn: g.turn, msg, type: type || 'info' });
    if (g.logs.length > 300) g.logs.shift();
    if (SS.ui && SS.ui.refreshLog) SS.ui.refreshLog();
  };

  G.toast = function (msg) {
    if (SS.ui && SS.ui.toast) SS.ui.toast(msg);
  };

  G.recordBattle = function (rec) {
    const g = G.state;
    if (!g) return;
    g.battles.push(rec);
    if (g.battles.length > 30) g.battles.shift();
  };

  /* ============================================================
   * Exploration
   * ============================================================ */
  G.updateExploration = function () {
    const g = G.state;
    if (!g) return;
    for (const e of g.empires) {
      if (!e.alive) continue;
      const fleetSources = [];
      for (const f of G.empireFleets(e.id)) fleetSources.push({ x: f.x, y: f.y });
      const colonySources = [];
      for (const sys of G.empireColonies(e.id)) colonySources.push({ x: sys.x, y: sys.y });
      for (const s of g.systems) {
        if (s.exploredBy.has(e.id)) continue;
        const nearFleet = fleetSources.some((src) => U.dist(src.x, src.y, s.x, s.y) < SENSOR_RANGE);
        const nearColony = colonySources.some((src) => U.dist(src.x, src.y, s.x, s.y) < SENSOR_RANGE);
        if (!nearFleet && !nearColony) continue;
        s.exploredBy.add(e.id);
        // Anomalies are only uncovered by an actual fleet survey
        if (nearFleet && s.anomaly) G.grantAnomaly(e, s);
      }
    }
  };

  G.grantAnomaly = function (empire, sys) {
    const a = sys.anomaly;
    sys.anomaly = null;
    const r = a.reward || {};
    empire.credits += r.credits || 0;
    empire.minerals += r.minerals || 0;
    if (r.research) empire.researchPool = (empire.researchPool || 0) + r.research;
    const bits = [];
    if (r.credits) bits.push(`${r.credits} credits`);
    if (r.minerals) bits.push(`${r.minerals} minerals`);
    if (r.research) bits.push(`${r.research} research`);
    G.log(
      `${empire.name} surveyed ${sys.name}: ${a.name} — ${a.desc} (+${bits.join(', ')})`,
      empire.isPlayer ? 'good' : 'info'
    );
    if (empire.isPlayer) G.toast(`${a.name} discovered at ${sys.name}`);
  };

  /* ============================================================
   * Player / shared actions
   * ============================================================ */
  G.select = function (type, id) {
    G.state.selected = type ? { type, id } : null;
    if (SS.ui && SS.ui.refreshSelection) SS.ui.refreshSelection();
  };

  G.issueMove = function (fleet, targetSystem) {
    if (!fleet || !targetSystem) return false;
    if (fleet.systemId === targetSystem.id && !fleet.moves) return false;
    const from = G.getSystem(fleet.systemId);
    const speed = G.fleetSpeed(fleet);
    const d = U.dist(from.x, from.y, targetSystem.x, targetSystem.y);
    const turns = Math.max(1, Math.ceil(d / speed));
    fleet.moves = { toSystemId: targetSystem.id, turnsLeft: turns, totalTurns: turns };
    return true;
  };

  G.stopFleet = function (fleet) {
    fleet.moves = null;
    const sys = G.getSystem(fleet.systemId);
    fleet.x = sys.x;
    fleet.y = sys.y;
  };

  G.splitFleet = function (fleet) {
    if (fleet.ships.length < 2) return false;
    const g = G.state;
    const half = Math.ceil(fleet.ships.length / 2);
    const newShips = fleet.ships.splice(0, half);
    const nf = makeFleet(fleet.empireId, fleet.systemId);
    nf.x = fleet.x;
    nf.y = fleet.y;
    nf.ships = newShips;
    nf.moves = fleet.moves ? { ...fleet.moves } : null;
    g.fleets.push(nf);
    G.log(`Fleet split at ${G.getSystem(fleet.systemId).name}.`, 'info');
    return true;
  };

  // Merge one idle fleet into another (same empire, same system, neither moving).
  G.mergeFleets = function (targetId, sourceId) {
    const g = G.state;
    const target = G.getFleet(targetId);
    const source = G.getFleet(sourceId);
    if (!target || !source || target === source) return false;
    if (target.empireId !== source.empireId || target.systemId !== source.systemId) return false;
    if (target.moves || source.moves) return false;
    target.ships.push(...source.ships);
    g.fleets = g.fleets.filter((f) => f !== source);
    if (target.empireId === 0) {
      G.log(`Fleets merged at ${G.getSystem(target.systemId).name}.`, 'info');
    }
    return true;
  };

  // Merge every idle fleet of an empire at a system into the first one.
  G.mergeAllFleetsAt = function (systemId, empireId) {
    const g = G.state;
    const list = g.fleets.filter(
      (f) => f.empireId === empireId && f.systemId === systemId && !f.moves
    );
    if (list.length < 2) return false;
    const target = list[0];
    for (let i = 1; i < list.length; i++) target.ships.push(...list[i].ships);
    g.fleets = g.fleets.filter((f) => f === target || !list.includes(f));
    if (empireId === 0) G.log(`All fleets merged at ${G.getSystem(systemId).name}.`, 'info');
    return true;
  };

  G.buildBuilding = function (system, key) {
    const col = system.planet.colony;
    if (!col) return false;
    const b = D.BUILDINGS[key];
    const empire = G.getEmpire(col.empireId);
    if (!b || !empire) return false;
    if (col.buildings.length >= G.slots(system)) {
      if (empire.isPlayer) G.toast('No free building slots');
      return false;
    }
    if (!b.repeat && col.buildings.includes(key)) return false;
    if (empire.minerals < b.cost) {
      if (empire.isPlayer) G.toast('Not enough minerals');
      return false;
    }
    empire.minerals -= b.cost;
    col.buildings.push(key);
    if (empire.isPlayer) G.log(`${b.name} built at ${system.name}.`, 'build');
    return true;
  };

  G.queueShip = function (system, key) {
    const col = system.planet.colony;
    if (!col) return false;
    const empire = G.getEmpire(col.empireId);
    const ship = D.SHIPS[key];
    if (!col.buildings.includes('shipyard')) {
      if (empire.isPlayer) G.toast('Requires a Shipyard');
      return false;
    }
    if (col.queue.length >= 6) {
      if (empire.isPlayer) G.toast('Queue is full');
      return false;
    }
    if (ship.tech && !empire.techs[ship.tech]) {
      if (empire.isPlayer) G.toast('Technology not researched');
      return false;
    }
    const cost = G.shipCost(empire, key);
    if (empire.credits < cost.credits || empire.minerals < cost.minerals) {
      if (empire.isPlayer) G.toast('Not enough resources');
      return false;
    }
    empire.credits -= cost.credits;
    empire.minerals -= cost.minerals;
    col.queue.push(key);
    if (empire.isPlayer) G.log(`${ship.name} queued at ${system.name}.`, 'build');
    return true;
  };

  // Player asks an AI empire for peace; the AI accepts if it is losing or by chance.
  G.playerSuePeace = function (other) {
    if (!other) return false;
    const me = G.player();
    if (!me.atWarWith.has(other.id)) return false;
    const myStr = G.empireStrength(me.id);
    const theirStr = G.empireStrength(other.id);
    if (theirStr < myStr * 0.7 || Math.random() < 0.5) {
      G.makePeace(me.id, other.id);
      G.toast(`${other.name} accepts peace`);
      return true;
    }
    G.log(`${other.name} rejects your peace offer.`, 'bad');
    G.toast(`${other.name} rejects peace`);
    return false;
  };

  G.setResearch = function (empire, techKey) {
    const tech = D.TECHS[techKey];
    if (!tech) return false;
    if (empire.techs[techKey]) return false;
    if (!tech.requires.every((r) => empire.techs[r])) return false;
    empire.currentTech = techKey;
    empire.techProgress = 0;
    G.log(`Research started: ${tech.name}.`, 'research');
    return true;
  };

  G.declareWar = function (a, b) {
    const ea = G.getEmpire(a);
    const eb = G.getEmpire(b);
    if (!ea || !eb || a === b) return;
    const already = ea.atWarWith.has(b);
    ea.atWarWith.add(b);
    eb.atWarWith.add(a);
    // a declaration clears any pending peace offers between them
    ea.peaceOffers = ea.peaceOffers.filter((x) => x !== b);
    eb.peaceOffers = eb.peaceOffers.filter((x) => x !== a);
    if (!already) {
      G.log(`${ea.name} declares war on ${eb.name}!`, 'war');
      if (ea.isPlayer || eb.isPlayer) G.toast(`War! ${ea.name} ↔ ${eb.name}`);
    }
  };

  G.makePeace = function (a, b) {
    const ea = G.getEmpire(a);
    const eb = G.getEmpire(b);
    if (!ea || !eb) return;
    ea.atWarWith.delete(b);
    eb.atWarWith.delete(a);
    ea.peaceOffers = ea.peaceOffers.filter((x) => x !== b);
    eb.peaceOffers = eb.peaceOffers.filter((x) => x !== a);
    G.log(`Peace signed between ${ea.name} and ${eb.name}.`, 'good');
  };

  G.offerPeace = function (from, to) {
    const efrom = G.getEmpire(from);
    const eto = G.getEmpire(to);
    if (!efrom || !eto) return false;
    if (!efrom.atWarWith.has(to)) return false;
    if (!eto.peaceOffers.includes(from)) eto.peaceOffers.push(from);
    G.log(`${efrom.name} offers peace to ${eto.name}.`, 'info');
    return true;
  };

  /* ============================================================
   * Turn processing
   * ============================================================ */
  G.endTurn = function () {
    const g = G.state;
    if (!g || g.over) return;

    // 1) AI empires take their turns
    for (const e of g.empires) {
      if (!e.isPlayer && e.alive) SS.ai.takeTurn(e);
    }

    // 2) Move every fleet one step
    for (const f of g.fleets) {
      if (f.moves) {
        f.moves.turnsLeft--;
        const from = G.getSystem(f.systemId);
        const to = G.getSystem(f.moves.toSystemId);
        const t = 1 - f.moves.turnsLeft / f.moves.totalTurns;
        f.x = from.x + (to.x - from.x) * t;
        f.y = from.y + (to.y - from.y) * t;
        if (f.moves.turnsLeft <= 0) {
          f.systemId = to.id;
          f.x = to.x;
          f.y = to.y;
          f.moves = null;
          G.arriveFleet(f);
        }
      } else {
        const sys = G.getSystem(f.systemId);
        f.x = sys.x;
        f.y = sys.y;
      }
    }

    // 3) Merge idle fleets of the same empire at the same system
    G.autoMergeFleets();

    // 4) Combat
    G.resolveCombat();

    // 5) Income, population growth, research
    for (const e of g.empires) {
      if (e.alive) G.applyIncome(e);
    }

    // 6) Ship construction progress
    G.progressQueues();

    // 7) Remove empty fleets
    g.fleets = g.fleets.filter((f) => f.ships.length > 0);

    // 8) Exploration
    G.updateExploration();

    // 9) Empire elimination check
    G.updateAlive();

    // 10) Advance turn and check victory
    g.turn++;
    G.checkVictory();

    if (SS.ui && SS.ui.refreshAll) SS.ui.refreshAll();
  };

  G.autoMergeFleets = function () {
    const g = G.state;
    const groups = {};
    for (const f of g.fleets) {
      if (f.moves) continue;
      const k = f.empireId + '@' + f.systemId;
      (groups[k] = groups[k] || []).push(f);
    }
    for (const key of Object.keys(groups)) {
      const list = groups[key];
      if (list.length < 2) continue;
      const target = list[0];
      for (let i = 1; i < list.length; i++) {
        target.ships.push(...list[i].ships);
        g.fleets = g.fleets.filter((f) => f !== list[i]);
      }
    }
  };

  G.arriveFleet = function (fleet) {
    const sys = G.getSystem(fleet.systemId);
    const empire = G.getEmpire(fleet.empireId);
    // Colonize if carrying a colony ship
    const idx = fleet.ships.findIndex((s) => D.SHIPS[s.cls].colonize);
    if (idx >= 0 && !sys.planet.colony && G.canColonize(empire, sys)) {
      fleet.ships.splice(idx, 1);
      sys.planet.colony = {
        empireId: empire.id,
        population: 2,
        buildings: [],
        queue: [],
        queueProgress: 0,
      };
      sys.ownerEmpireId = empire.id;
      sys.exploredBy.add(empire.id);
      G.log(`${empire.name} founded a colony at ${sys.name}.`, empire.isPlayer ? 'good' : 'info');
      if (empire.isPlayer) G.toast(`Colony founded at ${sys.name}!`);
    }
  };

  G.applyIncome = function (empire) {
    const g = G.state;
    let minerals = 0;
    let credits = 0;
    let research = 0;
    for (const sys of g.systems) {
      const col = sys.planet.colony;
      if (!col || col.empireId !== empire.id) continue;
      const p = G.colonyProduction(sys, col, empire);
      minerals += p.minerals;
      credits += p.credits;
      research += p.research;
      const cap = G.maxPop(sys, empire);
      col.population = Math.min(cap, col.population * (1 + G.colonyGrowthRate(col)));
    }
    // difficulty bonus for AI
    if (!empire.isPlayer && empire.difficulty === 'hard') {
      minerals *= 1.25; credits *= 1.25; research *= 1.25;
    } else if (!empire.isPlayer && empire.difficulty === 'easy') {
      minerals *= 0.8; credits *= 0.8; research *= 0.8;
    }
    empire.minerals += minerals;
    empire.credits += credits;
    empire.lastIncome = { minerals, credits, research };

    empire.researchPool = (empire.researchPool || 0) + research;

    if (empire.currentTech) {
      empire.techProgress += empire.researchPool;
      empire.researchPool = 0;
      const tech = D.TECHS[empire.currentTech];
      if (empire.techProgress >= tech.cost) {
        empire.techs[empire.currentTech] = true;
        G.log(`${empire.name} completed research: ${tech.name}.`, empire.isPlayer ? 'research' : 'info');
        if (empire.isPlayer) G.toast(`Research complete: ${tech.name}`);
        empire.currentTech = null;
        empire.techProgress = 0;
      }
    }
  };

  G.progressQueues = function () {
    const g = G.state;
    for (const sys of g.systems) {
      const col = sys.planet.colony;
      if (!col || col.queue.length === 0) continue;
      col.queueProgress += 1;
      const key = col.queue[0];
      const ship = D.SHIPS[key];
      if (col.queueProgress >= ship.buildTurns) {
        col.queue.shift();
        col.queueProgress = 0;
        G.deliverShip(sys, col.empireId, key);
      }
    }
  };

  G.deliverShip = function (sys, empireId, key) {
    const empire = G.getEmpire(empireId);
    // add to an existing idle fleet of this empire at this system, else create one
    let fleet = G.state.fleets.find(
      (f) => f.empireId === empireId && f.systemId === sys.id && !f.moves
    );
    if (!fleet) {
      fleet = makeFleet(empireId, sys.id);
      G.state.fleets.push(fleet);
    }
    addShipToFleet(fleet, empire, key);
    G.log(`${D.SHIPS[key].name} completed at ${sys.name}.`, empire.isPlayer ? 'build' : 'info');
  };

  /* ============================================================
   * Combat
   * ============================================================ */
  function mkShipRef(empireId, fleet, index) {
    const s = fleet.ships[index];
    return { cls: s.cls, hp: s.hp, empireId, refFleet: fleet.id, refIndex: index };
  }

  function mkPlatform(empireId) {
    return { cls: '__platform', hp: 30, attack: 10, empireId, refFleet: null, refIndex: -1 };
  }

  // teams: [{empireId, ships:[{cls,hp,attack?,empireId,refFleet,refIndex}], team, isPlatforms?}]
  // Returns {winnerTeam, losses:{empireId:n}, survivors:{empireId:n}}
  G.battle = function (teams) {
    const sim = teams.map((t) => ({
      empireId: t.empireId,
      team: t.team,
      isPlatforms: !!t.isPlatforms,
      ships: t.ships.map((s) => ({ ...s })),
    }));
    const startCount = {};
    for (const t of sim) startCount[t.empireId] = (startCount[t.empireId] || 0) + t.ships.length;

    let rounds = 0;
    while (true) {
      const alive = sim.filter((t) => t.ships.length > 0);
      if (alive.length <= 1) break;
      rounds++;
      if (rounds > 300) break;

      const attacks = [];
      for (const t of sim) {
        if (!t.ships.length) continue;
        for (const s of t.ships) {
          const enemies = sim.filter((o) => o.team !== t.team && o.ships.length > 0);
          if (!enemies.length) continue;
          const targetTeam = U.choice(enemies);
          const target = U.choice(targetTeam.ships);
          let atk = s.attack;
          if (atk === undefined) {
            atk = G.effectiveShipStats(G.getEmpire(s.empireId), s.cls).attack;
          }
          attacks.push({ target, dmg: atk * (0.75 + Math.random() * 0.5) });
        }
      }
      for (const a of attacks) a.target.hp -= a.dmg;
      for (const t of sim) t.ships = t.ships.filter((s) => s.hp > 0);
    }

    const alive = sim.filter((t) => t.ships.length > 0);
    const winnerTeam = alive.length === 1 ? alive[0].team : null;

    // Apply losses back to real fleets
    for (const t of sim) {
      if (t.isPlatforms) continue;
      const byFleet = {};
      for (const s of t.ships) {
        if (s.refFleet == null) continue;
        byFleet[s.refFleet] = (byFleet[s.refFleet] || 0) + 1;
      }
      for (const fid of Object.keys(byFleet)) {
        const fleet = G.getFleet(Number(fid));
        if (fleet) fleet.ships = fleet.ships.slice(0, byFleet[fid]);
      }
    }

    // Destroy shield building if its platforms died
    for (const t of sim) {
      if (t.isPlatforms && t.ships.length === 0 && t.platformSystemId != null) {
        const sys = G.getSystem(t.platformSystemId);
        if (sys && sys.planet.colony) {
          sys.planet.colony.buildings = sys.planet.colony.buildings.filter((b) => b !== 'shield');
        }
      }
    }

    const survivors = {};
    for (const t of sim) {
      survivors[t.empireId] = (survivors[t.empireId] || 0) + t.ships.length;
    }
    const losses = {};
    for (const eid of Object.keys(startCount)) {
      losses[eid] = startCount[eid] - (survivors[eid] || 0);
    }
    return { winnerTeam, losses, survivors, rounds };
  };

  G.resolveCombat = function () {
    const g = G.state;
    for (const sys of g.systems) {
      const fleets = G.fleetsAt(sys.id);
      if (!fleets.length) continue;
      const owner = sys.ownerEmpireId;
      const byEmpire = U.groupBy(fleets, (f) => f.empireId);
      const present = Object.keys(byEmpire).map(Number);
      const colony = sys.planet.colony;

      // ----- Case 1: system owned, attackers present -----
      if (owner !== null && G.getEmpire(owner) && G.getEmpire(owner).alive) {
        const attackers = present.filter((e) => e !== owner && G.atWar(e, owner));
        if (attackers.length > 0) {
          const defenders = [];
          for (const f of byEmpire[owner] || []) {
            for (let i = 0; i < f.ships.length; i++) defenders.push(mkShipRef(owner, f, i));
          }
          let platforms = null;
          if (colony && colony.buildings.includes('shield')) {
            platforms = {
              empireId: owner,
              team: 'def',
              isPlatforms: true,
              platformSystemId: sys.id,
              ships: [mkPlatform(owner), mkPlatform(owner), mkPlatform(owner), mkPlatform(owner)],
            };
          }
          if (defenders.length === 0 && !platforms) {
            // undefended colony captured
            const strongest = attackers.reduce((a, b) =>
              G.empireStrength(a) >= G.empireStrength(b) ? a : b
            );
            G.captureColony(sys, strongest);
            continue;
          }
          const teams = [];
          if (defenders.length > 0) {
            teams.push({ empireId: owner, team: 'def', ships: defenders });
          }
          if (platforms) teams.push(platforms);
          for (const e of attackers) {
            const ships = [];
            for (const f of byEmpire[e]) {
              for (let i = 0; i < f.ships.length; i++) ships.push(mkShipRef(e, f, i));
            }
            teams.push({ empireId: e, team: 'atk' + e, ships });
          }
          const res = G.battle(teams);
          const atkTeams = teams.filter((t) => String(t.team).startsWith('atk'));
          const attackerWon = res.winnerTeam != null && String(res.winnerTeam).startsWith('atk');
          const winnerTeamObj =
            res.winnerTeam == null
              ? null
              : teams.find((t) => String(t.team) === String(res.winnerTeam));
          const winnerName = winnerTeamObj ? G.getEmpire(winnerTeamObj.empireId).name : null;
          const totalLosses = Object.values(res.losses).reduce((a, b) => a + b, 0);
          G.log(
            `Battle at ${sys.name}: ${G.getEmpire(owner).name} vs ${attackers
              .map((a) => G.getEmpire(a).name)
              .join(', ')} — ${winnerName ? winnerName + ' victorious' : 'stalemate'}. Losses: ${totalLosses} ships.`,
            owner === 0 || attackers.includes(0) ? 'war' : 'info'
          );
          G.recordBattle({
            turn: g.turn,
            system: sys.name,
            kind: 'colony',
            participants: [G.getEmpire(owner).name, ...attackers.map((a) => G.getEmpire(a).name)],
            winner: winnerName,
            losses: totalLosses,
          });
          if (attackerWon && colony) {
            // strongest surviving attacker captures
            const survivors = atkTeams
              .filter((t) => t.ships.length > 0)
              .map((t) => t.empireId);
            const capturer = survivors.length
              ? survivors.reduce((a, b) => (G.empireStrength(a) >= G.empireStrength(b) ? a : b))
              : null;
            if (capturer != null) G.captureColony(sys, capturer);
          }
          continue;
        }
      }

      // ----- Case 2: free-for-all among hostile fleets -----
      const conflict = present.filter((e) =>
        present.some((o) => o !== e && G.atWar(e, o))
      );
      if (conflict.length >= 2) {
        const teams = conflict.map((e) => {
          const ships = [];
          for (const f of byEmpire[e]) {
            for (let i = 0; i < f.ships.length; i++) ships.push(mkShipRef(e, f, i));
          }
          return { empireId: e, team: 'f' + e, ships };
        });
        const res = G.battle(teams);
        const winnerTeamObj =
          res.winnerTeam == null
            ? null
            : teams.find((t) => String(t.team) === String(res.winnerTeam));
        const winnerName = winnerTeamObj ? G.getEmpire(winnerTeamObj.empireId).name : null;
        G.log(
          `Space battle at ${sys.name}: ${conflict
            .map((c) => G.getEmpire(c).name)
            .join(' vs ')} — ${winnerName ? winnerName + ' wins' : 'no survivor'}.`,
          conflict.includes(0) ? 'war' : 'info'
        );
        G.recordBattle({
          turn: g.turn,
          system: sys.name,
          kind: 'space',
          participants: conflict.map((c) => G.getEmpire(c).name),
          winner: winnerName,
          losses: Object.values(res.losses).reduce((a, b) => a + b, 0),
        });
      }
    }
  };

  G.captureColony = function (sys, empireId) {
    const col = sys.planet.colony;
    if (!col) return;
    const prev = G.getEmpire(col.empireId);
    const next = G.getEmpire(empireId);
    col.empireId = empireId;
    col.population = Math.max(1, Math.round(col.population * 0.5));
    // damage: remove one random building
    if (col.buildings.length > 0) {
      const idx = U.randInt(0, col.buildings.length - 1);
      col.buildings.splice(idx, 1);
    }
    col.queue = [];
    col.queueProgress = 0;
    sys.ownerEmpireId = empireId;
    sys.exploredBy.add(empireId);
    G.log(`${next.name} captured ${sys.name} from ${prev.name}.`, next.isPlayer ? 'good' : prev.isPlayer ? 'bad' : 'war');
    if (next.isPlayer) G.toast(`Captured ${sys.name}!`);
    if (prev.isPlayer) G.toast(`Lost ${sys.name}!`);
  };

  /* ============================================================
   * Elimination & victory
   * ============================================================ */
  G.updateAlive = function () {
    const g = G.state;
    for (const e of g.empires) {
      if (!e.alive) continue;
      const hasColony = G.empireColonies(e.id).length > 0;
      const hasFleet = G.empireFleets(e.id).length > 0;
      const hasColonyShip =
        G.empireFleets(e.id).some((f) => f.ships.some((s) => D.SHIPS[s.cls].colonize)) ||
        g.systems.some(
          (s) =>
            s.planet.colony &&
            s.planet.colony.empireId === e.id &&
            s.planet.colony.queue.includes('colony')
        );
      if (!hasColony && !hasColonyShip) {
        e.alive = false;
        // disperse any remaining fleets so a dead empire stops acting
        g.fleets = g.fleets.filter((f) => f.empireId !== e.id);
        G.log(`${e.name} has been eliminated.`, e.isPlayer ? 'bad' : 'war');
        if (e.isPlayer) G.toast('Your empire has fallen.');
      }
    }
  };

  G.checkVictory = function () {
    const g = G.state;
    if (g.over) return;
    const alive = g.empires.filter((e) => e.alive);
    const player = G.player();

    if (!player.alive) {
      g.over = true;
      g.winner = alive.find((e) => e !== player) || null;
      G.log('Defeat. Your empire is no more.', 'bad');
      if (SS.ui && SS.ui.showGameOver) SS.ui.showGameOver(false);
      return;
    }
    if (alive.length === 1 && alive[0].isPlayer) {
      g.over = true;
      g.winner = player;
      G.log('Victory! The galaxy is yours.', 'good');
      if (SS.ui && SS.ui.showGameOver) SS.ui.showGameOver(true);
      return;
    }
    // domination: control >= 55% of colonizable systems after turn 20
    const colonizable = g.systems.filter((s) => G.canColonize(player, s)).length;
    const mine = G.empireColonies(player.id).length;
    if (g.turn > 20 && colonizable > 0 && mine / colonizable >= 0.55) {
      g.over = true;
      g.winner = player;
      G.log('Galactic domination achieved!', 'good');
      if (SS.ui && SS.ui.showGameOver) SS.ui.showGameOver(true);
    }
  };

  /* ============================================================
   * Save / load
   * ============================================================ */
  G.serialize = function () {
    const g = G.state;
    return JSON.stringify({
      turn: g.turn,
      difficulty: g.difficulty,
      over: g.over,
      winner: g.winner ? g.winner.id : null,
      systems: g.systems.map((s) => ({
        ...s,
        exploredBy: [...s.exploredBy],
        planet: {
          ...s.planet,
          colony: s.planet.colony
            ? {
                ...s.planet.colony,
                queue: [...s.planet.colony.queue],
                buildings: [...s.planet.colony.buildings],
              }
            : null,
        },
      })),
      empires: g.empires.map((e) => ({
        ...e,
        atWarWith: [...e.atWarWith],
        peaceOffers: [...e.peaceOffers],
        techs: { ...e.techs },
        lastIncome: { ...e.lastIncome },
      })),
      fleets: g.fleets.map((f) => ({
        ...f,
        ships: f.ships.map((s) => ({ ...s })),
        moves: f.moves ? { ...f.moves } : null,
      })),
      battles: g.battles.map((b) => ({ ...b })),
      logs: g.logs.slice(-100),
    });
  };

  G.saveGame = function () {
    try {
      localStorage.setItem(SAVE_KEY, G.serialize());
      G.toast('Game saved');
      return true;
    } catch (err) {
      G.toast('Save failed: ' + err.message);
      return false;
    }
  };

  G.hasSave = function () {
    try {
      return !!localStorage.getItem(SAVE_KEY);
    } catch (e) {
      return false;
    }
  };

  G.loadGame = function () {
    try {
      const raw = localStorage.getItem(SAVE_KEY);
      if (!raw) return false;
      const data = JSON.parse(raw);
      data.systems.forEach((s) => {
        s.exploredBy = new Set(s.exploredBy);
      });
      data.empires.forEach((e) => {
        e.atWarWith = new Set(e.atWarWith || []);
        e.peaceOffers = e.peaceOffers || [];
      });
      G.state = data;
      G.state.selected = null;
      return true;
    } catch (e) {
      return false;
    }
  };
})();
