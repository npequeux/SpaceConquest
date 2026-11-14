/* Stellar Dominion — AI empire behaviour */
window.SS = window.SS || {};
(function () {
  const U = SS.util;
  const D = SS.data;
  const G = SS.game;

  const RESEARCH_PRIORITY = [
    'construction1', 'weapons1', 'engines1', 'colonization1', 'shields1',
    'construction2', 'weapons2', 'engines2', 'colonization2', 'shields2',
    'construction3', 'weapons3', 'engines3', 'colonization3', 'shields3',
  ];

  const AI = (SS.ai = {});

  AI.takeTurn = function (empire) {
    AI.chooseResearch(empire);
    AI.manageColonies(empire);
    AI.manageFleets(empire);
    AI.diplomacy(empire);
  };

  /* ---------------- research ---------------- */
  AI.chooseResearch = function (empire) {
    if (empire.currentTech) return;
    for (const key of RESEARCH_PRIORITY) {
      const t = D.TECHS[key];
      if (empire.techs[key]) continue;
      if (!t.requires.every((r) => empire.techs[r])) continue;
      AI.setResearch(empire, key);
      return;
    }
  };

  AI.setResearch = function (empire, key) {
    empire.currentTech = key;
    empire.techProgress = 0;
  };

  /* ---------------- colonies ---------------- */
  AI.manageColonies = function (empire) {
    for (const sys of G.empireColonies(empire.id)) {
      const col = sys.planet.colony;

      // Shipyard first — without it the colony cannot build ships
      if (!col.buildings.includes('shipyard') && col.buildings.length < G.slots(sys)) {
        if (G.buildBuilding(sys, 'shipyard')) continue;
      }

      // Economy / support buildings, one per colony per turn, keep a small reserve
      const desired = ['mine', 'lab', 'market', 'mine', 'lab', 'market', 'habitat', 'shield'];
      for (const key of desired) {
        if (col.buildings.length >= G.slots(sys)) break;
        const b = D.BUILDINGS[key];
        if (!b.repeat && col.buildings.includes(key)) continue;
        if (empire.minerals >= b.cost + 30) {
          G.buildBuilding(sys, key);
          break;
        }
      }

      // Ship production
      if (col.buildings.includes('shipyard') && col.queue.length < 3) {
        const key = AI.chooseShip(empire);
        if (key) G.queueShip(sys, key);
      }
    }
  };

  AI.chooseShip = function (empire) {
    const g = G.state;
    let colonyShips = 0;
    for (const f of G.empireFleets(empire.id)) {
      colonyShips += f.ships.filter((s) => s.cls === 'colony').length;
    }
    for (const s of g.systems) {
      const c = s.planet.colony;
      if (c && c.empireId === empire.id) {
        colonyShips += c.queue.filter((k) => k === 'colony').length;
      }
    }
    const targets = g.systems.filter((s) => !s.planet.colony && G.canColonize(empire, s));
    if (targets.length > 0 && colonyShips < 2 && Math.random() < 0.65) return 'colony';

    // best affordable military ship
    for (const key of ['battleship', 'cruiser', 'destroyer', 'frigate']) {
      const ship = D.SHIPS[key];
      if (ship.tech && !empire.techs[ship.tech]) continue;
      const cost = G.shipCost(empire, key);
      if (empire.credits >= cost.credits && empire.minerals >= cost.minerals) return key;
    }
    return null;
  };

  /* ---------------- fleets ---------------- */
  AI.manageFleets = function (empire) {
    const g = G.state;
    const home = G.getSystem(empire.homeSystemId);
    const idle = G.empireFleets(empire.id).filter((f) => !f.moves);

    // 1) Colony ships expand
    for (const f of idle) {
      if (f.ships.some((s) => D.SHIPS[s.cls].colonize)) {
        const target = AI.bestColonizationTarget(f, empire);
        if (target) G.issueMove(f, target);
      }
    }

    const military = idle.filter((f) => f.ships.some((s) => D.SHIPS[s.cls].attack > 0));
    const enemies = g.empires.filter(
      (e) => e.alive && e.id !== empire.id && G.atWar(empire.id, e.id)
    );

    // 2) Defense — repel fleets intruding in our systems
    for (const s of G.empireColonies(empire.id)) {
      const intruders = G.fleetsAt(s.id).filter(
        (x) => x.empireId !== empire.id && G.atWar(empire.id, x.empireId)
      );
      if (!intruders.length) continue;
      const defStr = intruders.reduce((a, x) => a + G.fleetStrength(x), 0);
      const candidates = military
        .filter((f) => !f.moves)
        .sort((a, b) => {
          const sa = G.getSystem(a.systemId);
          const sb = G.getSystem(b.systemId);
          return U.dist(sa.x, sa.y, s.x, s.y) - U.dist(sb.x, sb.y, s.x, s.y);
        });
      let acc = 0;
      for (const f of candidates) {
        if (acc >= defStr * 1.2) break;
        G.issueMove(f, s);
        acc += G.fleetStrength(f);
      }
    }

    // 3) Keep a garrison at home while at war
    if (enemies.length && military.length >= 2 && home) {
      const weakest = military
        .filter((f) => !f.moves)
        .sort((a, b) => G.fleetStrength(a) - G.fleetStrength(b))[0];
      if (weakest && weakest.systemId !== home.id) G.issueMove(weakest, home);
    }

    // 4) Offense — only attack with a credible force
    const enemySystems = [];
    for (const e of enemies) {
      for (const s of G.empireColonies(e.id)) enemySystems.push(s);
    }

    if (enemySystems.length) {
      for (const f of military) {
        if (f.moves) continue;
        const str = G.fleetStrength(f);
        if (str < 70) {
          if (home && f.systemId !== home.id) G.issueMove(f, home);
          continue;
        }
        let best = null;
        let bestD = Infinity;
        for (const s of enemySystems) {
          const cur = G.getSystem(f.systemId);
          const d = U.dist(cur.x, cur.y, s.x, s.y);
          if (d < bestD) {
            bestD = d;
            best = s;
          }
        }
        if (!best) continue;
        const defStr = G.fleetsAt(best.id)
          .filter((x) => G.atWar(empire.id, x.empireId))
          .reduce((a, x) => a + G.fleetStrength(x), 0);
        if (defStr > 0 && str < defStr * 1.1) {
          if (home && f.systemId !== home.id && Math.random() < 0.6) G.issueMove(f, home);
          continue;
        }
        G.issueMove(f, best);
      }
    } else if (home) {
      // peacetime: drift back toward home
      for (const f of military) {
        if (!f.moves && f.systemId !== home.id && Math.random() < 0.25) {
          G.issueMove(f, home);
        }
      }
    }
  };

  AI.bestColonizationTarget = function (fleet, empire) {
    const from = G.getSystem(fleet.systemId);
    let best = null;
    let bestScore = -Infinity;
    for (const s of G.state.systems) {
      if (s.planet.colony) continue;
      if (!G.canColonize(empire, s)) continue;
      const d = U.dist(from.x, from.y, s.x, s.y);
      const pt = D.PLANET_TYPES[s.planet.type];
      const score = pt.habitability * 100 - d * 0.05 + s.planet.richness * 5;
      if (score > bestScore) {
        bestScore = score;
        best = s;
      }
    }
    return best;
  };

  AI.nearestSystem = function (fleet, systems) {
    const from = G.getSystem(fleet.systemId);
    let best = null;
    let bestD = Infinity;
    for (const s of systems) {
      const d = U.dist(from.x, from.y, s.x, s.y);
      if (d < bestD) {
        bestD = d;
        best = s;
      }
    }
    return best;
  };

  /* ---------------- diplomacy ---------------- */
  AI.diplomacy = function (empire) {
    const g = G.state;
    for (const other of g.empires) {
      if (other.id === empire.id || !other.alive) continue;
      const atWar = empire.atWarWith.has(other.id);
      const myStr = G.empireStrength(empire.id);
      const theirStr = G.empireStrength(other.id);

      if (!atWar) {
        // Avoid dogpiling: only one rival may be at war with the player at a time.
        if (other.isPlayer) {
          const alreadyFightingPlayer = g.empires.some(
            (e) => e.alive && !e.isPlayer && e.id !== empire.id && G.atWar(e.id, other.id)
          );
          if (alreadyFightingPlayer) continue;
        }
        if (
          g.turn > 14 &&
          myStr > theirStr * 1.3 &&
          Math.random() < empire.aggression * 0.1
        ) {
          G.declareWar(empire.id, other.id);
        }
      } else if (myStr < theirStr * 0.35 && Math.random() < 0.18) {
        if (other.isPlayer) G.offerPeace(empire.id, other.id);
        else G.makePeace(empire.id, other.id);
      }
    }
  };
})();
