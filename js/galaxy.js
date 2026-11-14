/* Stellar Dominion — procedural galaxy generation */
window.SS = window.SS || {};
(function () {
  const U = SS.util;
  const D = SS.data;

  const WIDTH = 3600;
  const HEIGHT = 2400;

  function makePlanet() {
    const r = Math.random();
    let type;
    if (r < 0.10) type = 'terran';
    else if (r < 0.22) type = 'ocean';
    else if (r < 0.36) type = 'desert';
    else if (r < 0.50) type = 'ice';
    else if (r < 0.64) type = 'barren';
    else if (r < 0.74) type = 'toxic';
    else if (r < 0.84) type = 'volcanic';
    else type = 'gas';

    const sizeRoll = Math.random();
    const size = sizeRoll < 0.33 ? 1 : sizeRoll < 0.7 ? 2 : 3; // 1 small, 2 medium, 3 large
    const richness = U.rand(0.5, 2.0);
    const pt = D.PLANET_TYPES[type];
    const maxPop = Math.round(size * pt.habitability * 12);
    return { type, size, richness, maxPop, colony: null };
  }

  function makeSystem(id, x, y, usedNames) {
    const star = U.choice(D.STAR_CLASSES);
    return {
      id,
      name: D.systemName(usedNames),
      x, y,
      starClass: star.key,
      starColor: star.color,
      starRadius: star.radius,
      planet: makePlanet(),
      ownerEmpireId: null,
      exploredBy: new Set(),
    };
  }

  function pickHomes(systems, n) {
    const homes = [];
    // start from a random system, then farthest-point sampling
    homes.push(U.choice(systems));
    while (homes.length < n && homes.length < systems.length) {
      let best = null;
      let bestD = -1;
      for (const s of systems) {
        if (homes.includes(s)) continue;
        const d = Math.min(...homes.map((h) => U.dist(s.x, s.y, h.x, h.y)));
        if (d > bestD) { bestD = d; best = s; }
      }
      homes.push(best);
    }
    return homes;
  }

  SS.galaxy = {
    WIDTH,
    HEIGHT,

    generate(opts) {
      const numSystems = opts.numSystems || 46;
      const numEmpires = opts.numEmpires || 4;
      const systems = [];
      const usedNames = new Set();
      const minDist = 340;
      let attempts = 0;

      while (systems.length < numSystems && attempts < numSystems * 400) {
        attempts++;
        const x = U.rand(160, WIDTH - 160);
        const y = U.rand(160, HEIGHT - 160);
        if (systems.every((s) => U.dist(x, y, s.x, s.y) >= minDist)) {
          systems.push(makeSystem(systems.length, x, y, usedNames));
        }
      }

      const homes = pickHomes(systems, numEmpires);
      return { systems, homes };
    },

    makeHomeSystem(system) {
      system.planet = {
        type: 'terran',
        size: 3,
        richness: 1.5,
        maxPop: 36,
        colony: null,
      };
    },
  };
})();
