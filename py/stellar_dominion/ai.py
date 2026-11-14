"""Stellar Dominion — AI empire behaviour.

Reuses the player's action functions from ``engine``: the AI is just another caller.
"""

import random

from . import data
from . import engine as g


RESEARCH_PRIORITY = [
    "construction1", "weapons1", "engines1", "colonization1", "shields1",
    "construction2", "weapons2", "engines2", "colonization2", "shields2",
    "construction3", "weapons3", "engines3", "colonization3", "shields3",
]


def _sys(sid):
    """get_system with a non-None guarantee (ids always exist during a game)."""
    s = g.get_system(sid)
    assert s is not None, "missing system %r" % sid
    return s


def take_turn(empire):
    maybe_design(empire)
    choose_research(empire)
    manage_colonies(empire)
    manage_fleets(empire)
    diplomacy(empire)


# --------------------------------------------------------------------------
# Research
# --------------------------------------------------------------------------
def choose_research(empire):
    if empire["currentTech"]:
        return
    for key in RESEARCH_PRIORITY:
        tech = data.TECHS[key]
        if key in empire["techs"]:
            continue
        if not all(r in empire["techs"] for r in tech["requires"]):
            continue
        empire["currentTech"] = key
        empire["techProgress"] = 0
        return


# --------------------------------------------------------------------------
# Colonies
# --------------------------------------------------------------------------
def manage_colonies(empire):
    colonies = list(g.empire_colonies(empire["id"]))

    # 1) Ship production first — capital ships take several turns, so start them early
    for sys in colonies:
        col = sys["planet"]["colony"]
        if "shipyard" in col["buildings"] and len(col["queue"]) < 3:
            key = choose_ship(empire)
            if key:
                g.queue_ship(sys, key)

    # 2) Buildings — a couple per turn at most, so minerals keep accumulating
    builds = 0
    for sys in colonies:
        if builds >= 2:
            break
        col = sys["planet"]["colony"]

        # Shipyard first — without it the colony cannot build ships
        if "shipyard" not in col["buildings"] and len(col["buildings"]) < g.slots(sys):
            if g.build_building(sys, "shipyard"):
                builds += 1
                continue

        desired = ["mine", "lab", "market", "mine", "lab", "market", "habitat", "shield"]
        for key in desired:
            if len(col["buildings"]) >= g.slots(sys):
                break
            b = data.BUILDINGS[key]
            if not b["repeat"] and key in col["buildings"]:
                continue
            if empire["minerals"] >= b["cost"] + 40:
                if g.build_building(sys, key):
                    builds += 1
                break


def choose_ship(empire):
    fleets = g.empire_fleets(empire["id"])
    colony_ships = sum(1 for f in fleets for s in f["ships"] if s["cls"] == "colony")
    military = sum(1 for f in fleets for s in f["ships"] if g.ship_def(s["cls"])["attack"] > 0)
    for s in g.state["systems"]:
        col = s["planet"]["colony"]
        if col and col["empireId"] == empire["id"]:
            colony_ships += col["queue"].count("colony")

    colonies = len(g.empire_colonies(empire["id"]))
    targets = [s for s in g.state["systems"] if not s["planet"]["colony"] and g.can_colonize(empire, s)]

    # Expand only once the empire fields a credible fleet (roughly one warship per colony)
    if (targets and colony_ships < 2 and military >= colonies
            and random.random() < 0.7):
        return "colony"

    # Otherwise build the best affordable option across stock classes and designs
    scored = []
    candidates = [(k, g.ship_def(k)) for k in data.SHIPS]
    candidates += [(k, g.ship_def(k)) for k in g.list_designs(empire["id"])]
    for key, ship in candidates:
        if ship["tech"] and not empire["techs"].get(ship["tech"]):
            continue
        cost = g.ship_cost(empire, key)
        score = ship["attack"] * 2 + ship["hp"] * 0.5 + ship["speed"] * 0.05
        scored.append((score, key, cost))
    if not scored:
        return None
    scored.sort(key=lambda x: -x[0])
    top_score, top_key, top_cost = scored[0]
    if empire["credits"] >= top_cost["credits"] and empire["minerals"] >= top_cost["minerals"]:
        return top_key
    # Bootstrap the fleet with whatever is affordable, then save toward the best
    if military < min(colonies * 2, 8):
        for score, key, cost in scored:
            if empire["credits"] >= cost["credits"] and empire["minerals"] >= cost["minerals"]:
                return key
    return None


def _best_module(modules, empire):
    best = modules[0]
    for m in modules:
        if m["tech"] and not empire["techs"].get(m["tech"]):
            continue
        best = m
    return best


def maybe_design(empire):
    """Keep the empire's ship designs competitive as it researches better parts."""
    designs = g.list_designs(empire["id"])
    if g.state["turn"] < 8:
        return
    hull = data.HULLS[0]
    for h in data.HULLS:
        if h["tech"] and not empire["techs"].get(h["tech"]):
            continue
        hull = h
    weapon = _best_module(data.WEAPON_MODULES, empire)
    armor = _best_module(data.ARMOR_MODULES, empire)
    engine = _best_module(data.ENGINE_MODULES, empire)
    # A bare hull with neither armour nor an engine isn't worth a design slot yet
    if armor["key"] == "a0" and engine["key"] == "e0":
        return

    candidate = data.compute_design("tmp", hull["key"], weapon["key"], armor["key"], engine["key"])
    score = lambda d: d["attack"] * 2 + d["hp"] * 0.5 + d["speed"] * 0.05

    if designs:
        # replace the weakest design only when the new one is a real improvement
        weakest = min(designs, key=lambda k: score(g.ship_def(k)))
        if score(candidate) <= score(g.ship_def(weakest)) * 1.1:
            return
        if len(designs) >= 4:
            g.remove_design(weakest)
            designs = [d for d in designs if d != weakest]

    name = "%s %s Mk%d" % (empire["name"].split()[0],
                           hull["name"].replace(" Hull", ""), len(designs) + 1)
    g.add_design(empire["id"], name, hull["key"], weapon["key"], armor["key"], engine["key"])


# --------------------------------------------------------------------------
# Fleets
# --------------------------------------------------------------------------
def manage_fleets(empire):
    home = g.get_system(empire["homeSystemId"])
    idle = [f for f in g.empire_fleets(empire["id"]) if not f["moves"]]

    # 1) Colony ships expand
    for f in idle:
        if any(g.ship_def(s["cls"])["colonize"] for s in f["ships"]):
            target = best_colonization_target(f, empire)
            if target:
                g.issue_move(f, target)

    military = [f for f in idle if any(g.ship_def(s["cls"])["attack"] > 0 for s in f["ships"])]
    enemies = [e for e in g.state["empires"]
               if e["alive"] and e["id"] != empire["id"] and g.at_war(empire["id"], e["id"])]

    # 2) Defense — repel fleets intruding in our systems
    for sys in g.empire_colonies(empire["id"]):
        intruders = [x for x in g.fleets_at(sys["id"])
                     if x["empireId"] != empire["id"] and g.at_war(empire["id"], x["empireId"])]
        if not intruders:
            continue
        def_str = sum(g.fleet_strength(x) for x in intruders)
        candidates = sorted(
            [f for f in military if not f["moves"]],
            key=lambda f: g.dist(_sys(f["systemId"])["x"], _sys(f["systemId"])["y"],
                                 sys["x"], sys["y"]),
        )
        acc = 0.0
        for f in candidates:
            if acc >= def_str * 1.2:
                break
            g.issue_move(f, sys)
            acc += g.fleet_strength(f)

    # 3) Keep a garrison at home while at war
    if enemies and len(military) >= 2 and home:
        weakest = min((f for f in military if not f["moves"]),
                      key=lambda f: g.fleet_strength(f), default=None)
        if weakest and weakest["systemId"] != home["id"]:
            g.issue_move(weakest, home)

    # 4) Offense — only attack with a credible force
    enemy_systems = []
    for e in enemies:
        enemy_systems.extend(g.empire_colonies(e["id"]))

    if enemy_systems:
        for f in military:
            if f["moves"]:
                continue
            strength = g.fleet_strength(f)
            if strength < 70:
                if home and f["systemId"] != home["id"]:
                    g.issue_move(f, home)
                continue
            cur = _sys(f["systemId"])
            best = min(enemy_systems,
                       key=lambda s: g.dist(cur["x"], cur["y"], s["x"], s["y"]), default=None)
            if not best:
                continue
            def_str = sum(g.fleet_strength(x) for x in g.fleets_at(best["id"]) if g.at_war(empire["id"], x["empireId"]))
            if def_str > 0 and strength < def_str * 1.1:
                if home and f["systemId"] != home["id"] and random.random() < 0.6:
                    g.issue_move(f, home)
                continue
            g.issue_move(f, best)
    elif home:
        for f in military:
            if not f["moves"] and f["systemId"] != home["id"] and random.random() < 0.25:
                g.issue_move(f, home)


def best_colonization_target(fleet, empire):
    from_sys = _sys(fleet["systemId"])
    best = None
    best_score = float("-inf")
    for s in g.state["systems"]:
        if s["planet"]["colony"] or not g.can_colonize(empire, s):
            continue
        d = g.dist(from_sys["x"], from_sys["y"], s["x"], s["y"])
        pt = data.PLANET_TYPES[s["planet"]["type"]]
        score = pt["habitability"] * 100 - d * 0.05 + s["planet"]["richness"] * 5
        if score > best_score:
            best_score = score
            best = s
    return best


# --------------------------------------------------------------------------
# Diplomacy
# --------------------------------------------------------------------------
def diplomacy(empire):
    for other in g.state["empires"]:
        if other["id"] == empire["id"] or not other["alive"]:
            continue
        at_war = other["id"] in empire["atWarWith"]
        my_str = g.empire_strength(empire["id"])
        their_str = g.empire_strength(other["id"])

        if not at_war:
            # Avoid dogpiling: only one rival may be at war with the player at a time.
            if other["isPlayer"]:
                already = any(
                    e["alive"] and not e["isPlayer"] and e["id"] != empire["id"]
                    and g.at_war(e["id"], other["id"])
                    for e in g.state["empires"]
                )
                if already:
                    continue
            if (g.state["turn"] > 14 and my_str > their_str * 1.3
                    and random.random() < empire["aggression"] * 0.1):
                g.declare_war(empire["id"], other["id"])
        elif my_str < their_str * 0.35 and random.random() < 0.18:
            if other["isPlayer"]:
                g.offer_peace(empire["id"], other["id"])
            else:
                g.make_peace(empire["id"], other["id"])
