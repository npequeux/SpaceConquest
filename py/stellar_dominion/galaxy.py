"""Stellar Dominion — procedural galaxy generation."""

import random

from . import data

WIDTH = 3600
HEIGHT = 2400


def _make_planet(rng):
    r = rng.random()
    if r < 0.10:
        ptype = "terran"
    elif r < 0.22:
        ptype = "ocean"
    elif r < 0.36:
        ptype = "desert"
    elif r < 0.50:
        ptype = "ice"
    elif r < 0.64:
        ptype = "barren"
    elif r < 0.74:
        ptype = "toxic"
    elif r < 0.84:
        ptype = "volcanic"
    else:
        ptype = "gas"

    size_roll = rng.random()
    size = 1 if size_roll < 0.33 else 2 if size_roll < 0.7 else 3
    richness = rng.uniform(0.5, 2.0)
    pt = data.PLANET_TYPES[ptype]
    max_pop = round(size * pt["habitability"] * 12)
    return {"type": ptype, "size": size, "richness": richness, "maxPop": max_pop, "colony": None}


def _make_system(sid, x, y, used_names, rng):
    star = rng.choice(data.STAR_CLASSES)
    return {
        "id": sid,
        "name": data.system_name(used_names),
        "x": x,
        "y": y,
        "starClass": star["key"],
        "starColor": star["color"],
        "starRadius": star["radius"],
        "planet": _make_planet(rng),
        "ownerEmpireId": None,
        "exploredBy": set(),
        "anomaly": None,
    }


def _pick_homes(systems, n, rng):
    homes = [rng.choice(systems)]
    while len(homes) < n and len(homes) < len(systems):
        best = None
        best_d = -1.0
        for s in systems:
            if s in homes:
                continue
            d = min(((s["x"] - h["x"]) ** 2 + (s["y"] - h["y"]) ** 2) ** 0.5 for h in homes)
            if d > best_d:
                best_d = d
                best = s
        homes.append(best)
    return homes


def generate(num_systems=46, num_empires=4, seed=None):
    """Return (systems, homes)."""
    rng = random.Random(seed)
    systems = []
    used_names = set()
    min_dist = 340
    attempts = 0

    while len(systems) < num_systems and attempts < num_systems * 400:
        attempts += 1
        x = rng.uniform(160, WIDTH - 160)
        y = rng.uniform(160, HEIGHT - 160)
        if all(((x - s["x"]) ** 2 + (y - s["y"]) ** 2) ** 0.5 >= min_dist for s in systems):
            systems.append(_make_system(len(systems), x, y, used_names, rng))

    homes = _pick_homes(systems, num_empires, rng)
    return systems, homes


def make_home_system(system):
    system["planet"] = {
        "type": "terran",
        "size": 3,
        "richness": 1.5,
        "maxPop": 36,
        "colony": None,
    }
