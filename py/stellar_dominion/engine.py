"""Stellar Dominion — core game state, rules, economy, movement, combat, diplomacy.

The simulation is pure: it never imports pygame and never touches the filesystem
except through save_game / load_game. The UI registers a toast handler for
player-facing notifications.
"""

import json
import math
import os
import random

from . import data
from . import galaxy

SAVE_PATH = os.path.expanduser("~/.stellar_dominion_save.json")
SENSOR_RANGE = 360


# ---------------------------------------------------------------------------
# Module state
# ---------------------------------------------------------------------------
state: dict = None  # type: ignore[assignment]  # populated by new_game() / load_game()
toast_handler = None


def toast(msg):
    if toast_handler:
        toast_handler(msg)


# ---------------------------------------------------------------------------
# Lookup helpers
# ---------------------------------------------------------------------------
def get_system(sid):
    return next((s for s in state["systems"] if s["id"] == sid), None)


def get_empire(eid):
    return next((e for e in state["empires"] if e["id"] == eid), None)


def get_fleet(fid):
    return next((f for f in state["fleets"] if f["id"] == fid), None)


def fleets_at(system_id):
    return [f for f in state["fleets"] if f["systemId"] == system_id]


def empire_fleets(eid):
    return [f for f in state["fleets"] if f["empireId"] == eid]


def empire_colonies(eid):
    return [s for s in state["systems"] if s["planet"]["colony"] and s["planet"]["colony"]["empireId"] == eid]


def empire_systems(eid):
    return [s for s in state["systems"] if s["ownerEmpireId"] == eid]


def player():
    return state["empires"][0]


def slots(system):
    return system["planet"]["size"] + 2


def is_explored(system, eid):
    return eid in system["exploredBy"]


def at_war(a, b):
    ea = get_empire(a)
    eb = get_empire(b)
    return bool(ea and eb and (b in ea["atWarWith"] or a in eb["atWarWith"]))


# ---------------------------------------------------------------------------
# Ship definitions (stock classes plus the player's custom designs)
# ---------------------------------------------------------------------------
def ship_def(cls):
    """Return the stat dict for a stock class key or a 'custom:<empire>:<n>' design."""
    if isinstance(cls, str) and cls.startswith("custom:"):
        parts = cls.split(":")
        if len(parts) == 3:
            eid = parts[1]
            try:
                idx = int(parts[2])
            except ValueError:
                idx = -1
            designs = state["designs"].get(eid, [])
            if 0 <= idx < len(designs):
                return designs[idx]
        return data.SHIPS["frigate"]
    return data.SHIPS[cls]


def list_designs(eid):
    return ["custom:%s:%d" % (eid, i) for i in range(len(state["designs"].get(str(eid), [])))]


def add_design(empire_id, name, hull_key, weapon_key, armor_key, engine_key):
    eid = str(empire_id)
    designs = state["designs"].setdefault(eid, [])
    d = data.compute_design(name, hull_key, weapon_key, armor_key, engine_key)
    designs.append(d)
    return "custom:%s:%d" % (eid, len(designs) - 1)


def remove_design(cls):
    parts = cls.split(":")
    if len(parts) != 3:
        return False
    eid = parts[1]
    try:
        idx = int(parts[2])
    except ValueError:
        return False
    designs = state["designs"].get(eid, [])
    if not (0 <= idx < len(designs)):
        return False
    del designs[idx]
    return True


# ---------------------------------------------------------------------------
# Tech modifiers and effective ship stats
# ---------------------------------------------------------------------------
def tech_modifiers(empire):
    t = empire["techs"]
    m = {
        "attackMul": 1.0,
        "hpMul": 1.0,
        "speedBonus": 0,
        "costMul": 1.0,
        "maxPopBonus": 0,
        "canHostile": False,
        "colonyCostMul": 1.0,
    }
    if t.get("weapons1"): m["attackMul"] *= 1.15
    if t.get("weapons2"): m["attackMul"] *= 1.15
    if t.get("weapons3"): m["attackMul"] *= 1.25
    if t.get("shields1"): m["hpMul"] *= 1.15
    if t.get("shields2"): m["hpMul"] *= 1.15
    if t.get("shields3"): m["hpMul"] *= 1.25
    if t.get("engines1"): m["speedBonus"] += 40
    if t.get("engines2"): m["speedBonus"] += 40
    if t.get("engines3"): m["speedBonus"] += 80
    if t.get("construction1"): m["costMul"] *= 0.9
    if t.get("colonization1"): m["maxPopBonus"] += 3
    if t.get("colonization2"): m["canHostile"] = True
    if t.get("colonization3"):
        m["maxPopBonus"] += 4
        m["colonyCostMul"] *= 0.75
    return m


def effective_ship_stats(empire, cls):
    base = ship_def(cls)
    m = tech_modifiers(empire)
    return {
        "hp": round(base["hp"] * m["hpMul"]),
        "attack": round(base["attack"] * m["attackMul"]),
        "speed": base["speed"] + m["speedBonus"],
    }


def ship_cost(empire, cls):
    base = ship_def(cls)
    m = tech_modifiers(empire)
    mul = m["colonyCostMul"] if cls == "colony" else m["costMul"]
    return {
        "credits": round(base["credits"] * mul),
        "minerals": round(base["minerals"] * mul),
    }


def fleet_speed(fleet):
    e = get_empire(fleet["empireId"])
    if not fleet["ships"]:
        return 100
    return min(effective_ship_stats(e, s["cls"])["speed"] for s in fleet["ships"])


def fleet_strength(fleet):
    e = get_empire(fleet["empireId"])
    total = 0.0
    for s in fleet["ships"]:
        st = effective_ship_stats(e, s["cls"])
        total += st["attack"] * 2 + s["hp"] * 0.5
    return total


def empire_strength(eid):
    return sum(fleet_strength(f) for f in empire_fleets(eid))


def empire_ship_count(eid):
    return sum(len(f["ships"]) for f in empire_fleets(eid))


def max_pop(system, empire):
    return system["planet"]["maxPop"] + (tech_modifiers(empire)["maxPopBonus"] if empire else 0)


# ---------------------------------------------------------------------------
# Colony economy
# ---------------------------------------------------------------------------
def colony_production(system, col, empire):
    b = col["buildings"]
    minerals = 2 + b.count("mine") * 3 * system["planet"]["richness"]
    credits = 2 + b.count("market") * 4 + col["population"] * 0.8
    research = 1 + b.count("lab") * 5 + col["population"] * 0.3
    return {"minerals": minerals, "credits": credits, "research": research}


def colony_growth_rate(col):
    return 0.05 * (1.5 if "habitat" in col["buildings"] else 1.0)


def can_colonize(empire, system):
    pt = data.PLANET_TYPES.get(system["planet"]["type"])
    if not pt or pt["habitability"] <= 0:
        return False
    if pt["hostile"] and not tech_modifiers(empire)["canHostile"]:
        return False
    return True


# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
def log(msg, kind="info"):
    state["logs"].append({"turn": state["turn"], "msg": msg, "type": kind})
    if len(state["logs"]) > 300:
        del state["logs"][0]


def record_battle(rec):
    state["battles"].append(rec)
    if len(state["battles"]) > 30:
        del state["battles"][0]


# ---------------------------------------------------------------------------
# New game
# ---------------------------------------------------------------------------
def new_game(opts):
    global state

    num_ai = opts.get("numAI", 3)
    num_systems = opts.get("numSystems", 46)
    difficulty = opts.get("difficulty", "normal")
    player_name = opts.get("playerName", "Terran Consortium")
    seed = opts.get("seed")

    rng = random.Random(seed)
    systems, homes = galaxy.generate(num_systems, num_ai + 1, seed=seed)

    empires = []
    for i in range(num_ai + 1):
        defn = data.EMPIRE_DEFS[i % len(data.EMPIRE_DEFS)]
        empires.append({
            "id": i,
            "name": player_name if i == 0 else defn["name"],
            "color": defn["color"],
            "desc": defn["desc"],
            "isPlayer": i == 0,
            "credits": 250,
            "minerals": 200,
            "techProgress": 0,
            "researchPool": 0,
            "techs": {},
            "currentTech": None,
            "homeSystemId": None,
            "alive": True,
            "atWarWith": set(),
            "difficulty": difficulty,
            "aggression": rng.uniform(0.35, 0.9),
            "lastIncome": {"minerals": 0, "credits": 0, "research": 0},
            "peaceOffers": [],
        })

    state = {
        "turn": 1,
        "systems": systems,
        "empires": empires,
        "fleets": [],
        "logs": [],
        "battles": [],
        "designs": {},
        "selected": None,
        "over": False,
        "winner": None,
        "difficulty": difficulty,
    }

    # Scatter anomalies on unclaimed worlds (homes are exempt)
    for s in state["systems"]:
        if s["ownerEmpireId"] is None and rng.random() < 0.28:
            s["anomaly"] = rng.choice(data.ANOMALIES)

    _uid = [0]

    def make_fleet(empire_id, system_id):
        sys = get_system(system_id)
        _uid[0] += 1
        return {
            "id": _uid[0],
            "empireId": empire_id,
            "systemId": system_id,
            "x": sys["x"] if sys else 0,
            "y": sys["y"] if sys else 0,
            "ships": [],
            "moves": None,
        }

    def add_ship(fleet, empire, cls):
        fleet["ships"].append({"cls": cls, "hp": effective_ship_stats(empire, cls)["hp"]})

    for i, e in enumerate(empires):
        home = homes[i]
        e["homeSystemId"] = home["id"]
        galaxy.make_home_system(home)
        home["ownerEmpireId"] = e["id"]
        home["planet"]["colony"] = {
            "empireId": e["id"],
            "population": 8,
            "buildings": ["mine", "market", "lab", "shipyard"],
            "queue": [],
            "queueProgress": 0,
        }
        home["exploredBy"].add(e["id"])

        fleet = make_fleet(e["id"], home["id"])
        add_ship(fleet, e, "colony")
        add_ship(fleet, e, "frigate")
        add_ship(fleet, e, "scout")
        state["fleets"].append(fleet)

    update_exploration()
    log("Your empire rises. Expand, research, and conquer.", "info")
    log("Build ships at colonies with a Shipyard. Select a fleet and click a destination to move.", "info")
    return state


# ---------------------------------------------------------------------------
# Exploration
# ---------------------------------------------------------------------------
def update_exploration():
    for e in state["empires"]:
        if not e["alive"]:
            continue
        fleet_sources = [{"x": f["x"], "y": f["y"]} for f in empire_fleets(e["id"])]
        colony_sources = [{"x": s["x"], "y": s["y"]} for s in empire_colonies(e["id"])]
        for s in state["systems"]:
            if e["id"] in s["exploredBy"]:
                continue
            near_fleet = any(dist(src["x"], src["y"], s["x"], s["y"]) < SENSOR_RANGE for src in fleet_sources)
            near_colony = any(dist(src["x"], src["y"], s["x"], s["y"]) < SENSOR_RANGE for src in colony_sources)
            if not near_fleet and not near_colony:
                continue
            s["exploredBy"].add(e["id"])
            if near_fleet and s["anomaly"]:
                grant_anomaly(e, s)


def grant_anomaly(empire, system):
    a = system["anomaly"]
    system["anomaly"] = None
    r = a.get("reward", {})
    empire["credits"] += r.get("credits", 0)
    empire["minerals"] += r.get("minerals", 0)
    if r.get("research"):
        empire["researchPool"] += r["research"]
    bits = []
    if r.get("credits"): bits.append("%d credits" % r["credits"])
    if r.get("minerals"): bits.append("%d minerals" % r["minerals"])
    if r.get("research"): bits.append("%d research" % r["research"])
    log("%s surveyed %s: %s — %s (+%s)" % (empire["name"], system["name"], a["name"], a["desc"], ", ".join(bits)),
        "good" if empire["isPlayer"] else "info")
    if empire["isPlayer"]:
        toast("%s discovered at %s" % (a["name"], system["name"]))


def dist(ax, ay, bx, by):
    return math.hypot(ax - bx, ay - by)


# ---------------------------------------------------------------------------
# Actions
# ---------------------------------------------------------------------------
def select(sel):
    state["selected"] = sel


def issue_move(fleet, target_system):
    if not fleet or not target_system:
        return False
    if fleet["systemId"] == target_system["id"] and not fleet["moves"]:
        return False
    from_sys = get_system(fleet["systemId"])
    speed = fleet_speed(fleet)
    d = dist(from_sys["x"], from_sys["y"], target_system["x"], target_system["y"])
    turns = max(1, math.ceil(d / speed))
    fleet["moves"] = {"toSystemId": target_system["id"], "turnsLeft": turns, "totalTurns": turns}
    return True


def stop_fleet(fleet):
    fleet["moves"] = None
    sys = get_system(fleet["systemId"])
    fleet["x"] = sys["x"]
    fleet["y"] = sys["y"]


def split_fleet(fleet):
    if len(fleet["ships"]) < 2:
        return False
    half = math.ceil(len(fleet["ships"]) / 2)
    new_ships = fleet["ships"][:half]
    fleet["ships"] = fleet["ships"][half:]
    _uid = max((f["id"] for f in state["fleets"]), default=0)
    nf = {
        "id": _uid + 1,
        "empireId": fleet["empireId"],
        "systemId": fleet["systemId"],
        "x": fleet["x"],
        "y": fleet["y"],
        "ships": new_ships,
        "moves": dict(fleet["moves"]) if fleet["moves"] else None,
    }
    state["fleets"].append(nf)
    log("Fleet split at %s." % get_system(fleet["systemId"])["name"], "info")
    return True


def merge_fleets(target_id, source_id):
    target = get_fleet(target_id)
    source = get_fleet(source_id)
    if not target or not source or target is source:
        return False
    if target["empireId"] != source["empireId"] or target["systemId"] != source["systemId"]:
        return False
    if target["moves"] or source["moves"]:
        return False
    target["ships"].extend(source["ships"])
    state["fleets"] = [f for f in state["fleets"] if f is not source]
    if target["empireId"] == 0:
        log("Fleets merged at %s." % get_system(target["systemId"])["name"], "info")
    return True


def merge_all_fleets_at(system_id, empire_id):
    lst = [f for f in state["fleets"]
           if f["empireId"] == empire_id and f["systemId"] == system_id and not f["moves"]]
    if len(lst) < 2:
        return False
    target = lst[0]
    for f in lst[1:]:
        target["ships"].extend(f["ships"])
    state["fleets"] = [f for f in state["fleets"] if f is target or f not in lst]
    if empire_id == 0:
        log("All fleets merged at %s." % get_system(system_id)["name"], "info")
    return True


def build_building(system, key):
    col = system["planet"]["colony"]
    if not col:
        return False
    b = data.BUILDINGS.get(key)
    empire = get_empire(col["empireId"])
    if not b or not empire:
        return False
    if len(col["buildings"]) >= slots(system):
        if empire["isPlayer"]:
            toast("No free building slots")
        return False
    if not b["repeat"] and key in col["buildings"]:
        return False
    if empire["minerals"] < b["cost"]:
        if empire["isPlayer"]:
            toast("Not enough minerals")
        return False
    empire["minerals"] -= b["cost"]
    col["buildings"].append(key)
    if empire["isPlayer"]:
        log("%s built at %s." % (b["name"], system["name"]), "build")
    return True


def queue_ship(system, key):
    col = system["planet"]["colony"]
    if not col:
        return False
    empire = get_empire(col["empireId"])
    ship = ship_def(key)
    if "shipyard" not in col["buildings"]:
        if empire["isPlayer"]:
            toast("Requires a Shipyard")
        return False
    if len(col["queue"]) >= 6:
        if empire["isPlayer"]:
            toast("Queue is full")
        return False
    if ship["tech"] and not empire["techs"].get(ship["tech"]):
        if empire["isPlayer"]:
            toast("Technology not researched")
        return False
    cost = ship_cost(empire, key)
    if empire["credits"] < cost["credits"] or empire["minerals"] < cost["minerals"]:
        if empire["isPlayer"]:
            toast("Not enough resources")
        return False
    empire["credits"] -= cost["credits"]
    empire["minerals"] -= cost["minerals"]
    col["queue"].append(key)
    if empire["isPlayer"]:
        log("%s queued at %s." % (ship["name"], system["name"]), "build")
    return True


def set_research(empire, tech_key):
    tech = data.TECHS.get(tech_key)
    if not tech or tech_key in empire["techs"]:
        return False
    if not all(r in empire["techs"] for r in tech["requires"]):
        return False
    empire["currentTech"] = tech_key
    empire["techProgress"] = 0
    log("Research started: %s." % tech["name"], "research")
    return True


def declare_war(a, b):
    ea = get_empire(a)
    eb = get_empire(b)
    if not ea or not eb or a == b:
        return
    already = b in ea["atWarWith"]
    ea["atWarWith"].add(b)
    eb["atWarWith"].add(a)
    ea["peaceOffers"] = [x for x in ea["peaceOffers"] if x != b]
    eb["peaceOffers"] = [x for x in eb["peaceOffers"] if x != a]
    if not already:
        log("%s declares war on %s!" % (ea["name"], eb["name"]), "war")
        if ea["isPlayer"] or eb["isPlayer"]:
            toast("War! %s ↔ %s" % (ea["name"], eb["name"]))


def make_peace(a, b):
    ea = get_empire(a)
    eb = get_empire(b)
    if not ea or not eb:
        return
    ea["atWarWith"].discard(b)
    eb["atWarWith"].discard(a)
    ea["peaceOffers"] = [x for x in ea["peaceOffers"] if x != b]
    eb["peaceOffers"] = [x for x in eb["peaceOffers"] if x != a]
    log("Peace signed between %s and %s." % (ea["name"], eb["name"]), "good")


def offer_peace(from_id, to_id):
    efrom = get_empire(from_id)
    eto = get_empire(to_id)
    if not efrom or not eto or to_id not in efrom["atWarWith"]:
        return False
    if from_id not in eto["peaceOffers"]:
        eto["peaceOffers"].append(from_id)
    log("%s offers peace to %s." % (efrom["name"], eto["name"]), "info")
    return True


def player_sue_peace(other):
    if not other:
        return False
    me = player()
    if other["id"] not in me["atWarWith"]:
        return False
    my_str = empire_strength(me["id"])
    their_str = empire_strength(other["id"])
    if their_str < my_str * 0.7 or random.random() < 0.5:
        make_peace(me["id"], other["id"])
        toast("%s accepts peace" % other["name"])
        return True
    log("%s rejects your peace offer." % other["name"], "bad")
    toast("%s rejects peace" % other["name"])
    return False


# ---------------------------------------------------------------------------
# Turn processing
# ---------------------------------------------------------------------------
def end_turn(ai_module):
    """Resolve one turn. ``ai_module`` must expose take_turn(empire)."""
    if state["over"]:
        return

    # 1) AI empires take their turns
    for e in state["empires"]:
        if not e["isPlayer"] and e["alive"]:
            ai_module.take_turn(e)

    # 2) Move every fleet one step
    for f in state["fleets"]:
        if f["moves"]:
            f["moves"]["turnsLeft"] -= 1
            from_sys = get_system(f["systemId"])
            to_sys = get_system(f["moves"]["toSystemId"])
            t = 1 - f["moves"]["turnsLeft"] / f["moves"]["totalTurns"]
            f["x"] = from_sys["x"] + (to_sys["x"] - from_sys["x"]) * t
            f["y"] = from_sys["y"] + (to_sys["y"] - from_sys["y"]) * t
            if f["moves"]["turnsLeft"] <= 0:
                f["systemId"] = to_sys["id"]
                f["x"] = to_sys["x"]
                f["y"] = to_sys["y"]
                f["moves"] = None
                arrive_fleet(f)
        else:
            sys = get_system(f["systemId"])
            f["x"] = sys["x"]
            f["y"] = sys["y"]

    # 3) Merge idle fleets of the same empire at the same system
    auto_merge_fleets()

    # 4) Combat
    resolve_combat()

    # 5) Income, growth, research
    for e in state["empires"]:
        if e["alive"]:
            apply_income(e)

    # 6) Construction
    progress_queues()

    # 7) Drop empty fleets
    state["fleets"] = [f for f in state["fleets"] if f["ships"]]

    # 8) Exploration
    update_exploration()

    # 9) Elimination
    update_alive()

    # 10) Advance and check victory
    state["turn"] += 1
    check_victory()


def auto_merge_fleets():
    groups = {}
    for f in state["fleets"]:
        if f["moves"]:
            continue
        groups.setdefault("%s@%s" % (f["empireId"], f["systemId"]), []).append(f)
    for lst in groups.values():
        if len(lst) < 2:
            continue
        target = lst[0]
        for f in lst[1:]:
            target["ships"].extend(f["ships"])
            state["fleets"] = [x for x in state["fleets"] if x is not f]


def arrive_fleet(fleet):
    sys = get_system(fleet["systemId"])
    empire = get_empire(fleet["empireId"])
    idx = next((i for i, s in enumerate(fleet["ships"]) if ship_def(s["cls"])["colonize"]), None)
    if idx is not None and not sys["planet"]["colony"] and can_colonize(empire, sys):
        del fleet["ships"][idx]
        sys["planet"]["colony"] = {
            "empireId": empire["id"],
            "population": 2,
            "buildings": [],
            "queue": [],
            "queueProgress": 0,
        }
        sys["ownerEmpireId"] = empire["id"]
        sys["exploredBy"].add(empire["id"])
        log("%s founded a colony at %s." % (empire["name"], sys["name"]),
            "good" if empire["isPlayer"] else "info")
        if empire["isPlayer"]:
            toast("Colony founded at %s!" % sys["name"])


def apply_income(empire):
    minerals = credits = research = 0.0
    for sys in state["systems"]:
        col = sys["planet"]["colony"]
        if not col or col["empireId"] != empire["id"]:
            continue
        p = colony_production(sys, col, empire)
        minerals += p["minerals"]
        credits += p["credits"]
        research += p["research"]
        cap = max_pop(sys, empire)
        col["population"] = min(cap, col["population"] * (1 + colony_growth_rate(col)))

    if not empire["isPlayer"]:
        if empire["difficulty"] == "hard":
            minerals *= 1.25
            credits *= 1.25
            research *= 1.25
        elif empire["difficulty"] == "easy":
            minerals *= 0.8
            credits *= 0.8
            research *= 0.8

    empire["minerals"] += minerals
    empire["credits"] += credits
    empire["lastIncome"] = {"minerals": minerals, "credits": credits, "research": research}

    empire["researchPool"] += research
    if empire["currentTech"]:
        empire["techProgress"] += empire["researchPool"]
        empire["researchPool"] = 0
        tech = data.TECHS[empire["currentTech"]]
        if empire["techProgress"] >= tech["cost"]:
            empire["techs"][empire["currentTech"]] = True
            log("%s completed research: %s." % (empire["name"], tech["name"]),
                "research" if empire["isPlayer"] else "info")
            if empire["isPlayer"]:
                toast("Research complete: %s" % tech["name"])
            empire["currentTech"] = None
            empire["techProgress"] = 0


def progress_queues():
    for sys in state["systems"]:
        col = sys["planet"]["colony"]
        if not col or not col["queue"]:
            continue
        col["queueProgress"] += 1
        key = col["queue"][0]
        ship = ship_def(key)
        if col["queueProgress"] >= ship["buildTurns"]:
            col["queue"].pop(0)
            col["queueProgress"] = 0
            deliver_ship(sys, col["empireId"], key)


def deliver_ship(sys, empire_id, key):
    empire = get_empire(empire_id)
    fleet = next((f for f in state["fleets"]
                  if f["empireId"] == empire_id and f["systemId"] == sys["id"] and not f["moves"]), None)
    if not fleet:
        _uid = max((f["id"] for f in state["fleets"]), default=0)
        fleet = {
            "id": _uid + 1,
            "empireId": empire_id,
            "systemId": sys["id"],
            "x": sys["x"],
            "y": sys["y"],
            "ships": [],
            "moves": None,
        }
        state["fleets"].append(fleet)
    fleet["ships"].append({"cls": key, "hp": effective_ship_stats(empire, key)["hp"]})
    log("%s completed at %s." % (ship_def(key)["name"], sys["name"]),
        "build" if empire["isPlayer"] else "info")


# ---------------------------------------------------------------------------
# Combat
# ---------------------------------------------------------------------------
def _ship_ref(empire_id, fleet, index):
    s = fleet["ships"][index]
    return {"cls": s["cls"], "hp": s["hp"], "empireId": empire_id,
            "refFleet": fleet["id"], "refIndex": index}


def _platform(empire_id):
    return {"cls": "__platform", "hp": 30, "attack": 10, "empireId": empire_id,
            "refFleet": None, "refIndex": -1}


def battle(teams):
    """Simulate combat. Returns dict with winnerTeam, losses, survivors."""
    sim = [{"empireId": t["empireId"], "team": t["team"], "isPlatforms": t.get("isPlatforms", False),
            "platformSystemId": t.get("platformSystemId"),
            "ships": [dict(s) for s in t["ships"]]} for t in teams]
    start_count = {}
    for t in sim:
        start_count[t["empireId"]] = start_count.get(t["empireId"], 0) + len(t["ships"])

    rounds = 0
    while True:
        alive = [t for t in sim if t["ships"]]
        if len(alive) <= 1:
            break
        rounds += 1
        if rounds > 300:
            break

        attacks = []
        for t in sim:
            if not t["ships"]:
                continue
            for s in t["ships"]:
                enemies = [o for o in sim if o["team"] != t["team"] and o["ships"]]
                if not enemies:
                    continue
                target_team = random.choice(enemies)
                target = random.choice(target_team["ships"])
                atk = s.get("attack")
                if atk is None:
                    atk = effective_ship_stats(get_empire(s["empireId"]), s["cls"])["attack"]
                attacks.append((target, atk * random.uniform(0.75, 1.25)))
        for target, dmg in attacks:
            target["hp"] -= dmg
        for t in sim:
            t["ships"] = [s for s in t["ships"] if s["hp"] > 0]

    alive = [t for t in sim if t["ships"]]
    winner_team = alive[0]["team"] if len(alive) == 1 else None

    # Apply losses back to real fleets
    for t in sim:
        if t["isPlatforms"]:
            continue
        by_fleet = {}
        for s in t["ships"]:
            if s["refFleet"] is None:
                continue
            by_fleet[s["refFleet"]] = by_fleet.get(s["refFleet"], 0) + 1
        for fid, count in by_fleet.items():
            fleet = get_fleet(fid)
            if fleet:
                fleet["ships"] = fleet["ships"][:count]

    # Destroy shield building if its platforms died
    for t in sim:
        if t["isPlatforms"] and not t["ships"] and t.get("platformSystemId") is not None:
            sys = get_system(t["platformSystemId"])
            if sys and sys["planet"]["colony"]:
                sys["planet"]["colony"]["buildings"] = [
                    b for b in sys["planet"]["colony"]["buildings"] if b != "shield"
                ]

    survivors = {}
    for t in sim:
        survivors[t["empireId"]] = survivors.get(t["empireId"], 0) + len(t["ships"])
    losses = {eid: start_count[eid] - survivors.get(eid, 0) for eid in start_count}
    return {"winnerTeam": winner_team, "losses": losses, "survivors": survivors, "rounds": rounds}


def resolve_combat():
    for sys in state["systems"]:
        fleets = fleets_at(sys["id"])
        if not fleets:
            continue
        owner = sys["ownerEmpireId"]
        by_empire = {}
        for f in fleets:
            by_empire.setdefault(f["empireId"], []).append(f)
        present = list(by_empire.keys())
        colony = sys["planet"]["colony"]

        # Case 1: owned system with attackers present
        if owner is not None and get_empire(owner) and get_empire(owner)["alive"]:
            attackers = [e for e in present if e != owner and at_war(e, owner)]
            if attackers:
                defenders = []
                for f in by_empire.get(owner, []):
                    for i in range(len(f["ships"])):
                        defenders.append(_ship_ref(owner, f, i))
                platforms = None
                if colony and "shield" in colony["buildings"]:
                    platforms = {
                        "empireId": owner, "team": "def", "isPlatforms": True,
                        "platformSystemId": sys["id"],
                        "ships": [_platform(owner) for _ in range(4)],
                    }
                if not defenders and not platforms:
                    strongest = max(attackers, key=lambda a: empire_strength(a))
                    capture_colony(sys, strongest)
                    continue

                teams = []
                if defenders:
                    teams.append({"empireId": owner, "team": "def", "ships": defenders})
                if platforms:
                    teams.append(platforms)
                for e in attackers:
                    ships = []
                    for f in by_empire[e]:
                        for i in range(len(f["ships"])):
                            ships.append(_ship_ref(e, f, i))
                    teams.append({"empireId": e, "team": "atk%s" % e, "ships": ships})

                res = battle(teams)
                atk_teams = [t for t in teams if str(t["team"]).startswith("atk")]
                attacker_won = res["winnerTeam"] is not None and str(res["winnerTeam"]).startswith("atk")
                winner_team = next((t for t in teams if str(t["team"]) == str(res["winnerTeam"])), None)
                winner_name = get_empire(winner_team["empireId"])["name"] if winner_team else None
                total_losses = sum(res["losses"].values())
                log("Battle at %s: %s vs %s — %s. Losses: %d ships." % (
                        sys["name"], get_empire(owner)["name"],
                        ", ".join(get_empire(a)["name"] for a in attackers),
                        (winner_name + " victorious") if winner_name else "stalemate", total_losses),
                    "war" if (owner == 0 or 0 in attackers) else "info")
                record_battle({
                    "turn": state["turn"], "system": sys["name"], "kind": "colony",
                    "participants": [get_empire(owner)["name"]] + [get_empire(a)["name"] for a in attackers],
                    "winner": winner_name, "losses": total_losses,
                })
                if attacker_won and colony:
                    survivors = [t["empireId"] for t in atk_teams if t["ships"]]
                    capturer = max(survivors, key=lambda a: empire_strength(a)) if survivors else None
                    if capturer is not None:
                        capture_colony(sys, capturer)
                continue

        # Case 2: free-for-all among hostile fleets
        conflict = [e for e in present if any(o != e and at_war(e, o) for o in present)]
        if len(conflict) >= 2:
            teams = []
            for e in conflict:
                ships = []
                for f in by_empire[e]:
                    for i in range(len(f["ships"])):
                        ships.append(_ship_ref(e, f, i))
                teams.append({"empireId": e, "team": "f%s" % e, "ships": ships})
            res = battle(teams)
            winner_team = next((t for t in teams if str(t["team"]) == str(res["winnerTeam"])), None)
            winner_name = get_empire(winner_team["empireId"])["name"] if winner_team else None
            log("Space battle at %s: %s — %s." % (
                    sys["name"], " vs ".join(get_empire(c)["name"] for c in conflict),
                    (winner_name + " wins") if winner_name else "no survivor"),
                "war" if 0 in conflict else "info")
            record_battle({
                "turn": state["turn"], "system": sys["name"], "kind": "space",
                "participants": [get_empire(c)["name"] for c in conflict],
                "winner": winner_name, "losses": sum(res["losses"].values()),
            })


def capture_colony(sys, empire_id):
    col = sys["planet"]["colony"]
    if not col:
        return
    prev = get_empire(col["empireId"])
    next_ = get_empire(empire_id)
    col["empireId"] = empire_id
    col["population"] = max(1, round(col["population"] * 0.5))
    if col["buildings"]:
        col["buildings"].pop(random.randrange(len(col["buildings"])))
    col["queue"] = []
    col["queueProgress"] = 0
    sys["ownerEmpireId"] = empire_id
    sys["exploredBy"].add(empire_id)
    log("%s captured %s from %s." % (next_["name"], sys["name"], prev["name"]),
        "good" if next_["isPlayer"] else ("bad" if prev["isPlayer"] else "war"))
    if next_["isPlayer"]:
        toast("Captured %s!" % sys["name"])
    if prev["isPlayer"]:
        toast("Lost %s!" % sys["name"])


# ---------------------------------------------------------------------------
# Elimination and victory
# ---------------------------------------------------------------------------
def update_alive():
    for e in state["empires"]:
        if not e["alive"]:
            continue
        has_colony = len(empire_colonies(e["id"])) > 0
        has_colony_ship = any(
            ship_def(s["cls"])["colonize"] for f in empire_fleets(e["id"]) for s in f["ships"]
        ) or any(
            s["planet"]["colony"] and s["planet"]["colony"]["empireId"] == e["id"]
            and "colony" in s["planet"]["colony"]["queue"]
            for s in state["systems"]
        )
        if not has_colony and not has_colony_ship:
            e["alive"] = False
            state["fleets"] = [f for f in state["fleets"] if f["empireId"] != e["id"]]
            log("%s has been eliminated." % e["name"], "bad" if e["isPlayer"] else "war")
            if e["isPlayer"]:
                toast("Your empire has fallen.")


def check_victory():
    if state["over"]:
        return
    alive = [e for e in state["empires"] if e["alive"]]
    p = player()

    if not p["alive"]:
        state["over"] = True
        state["winner"] = next((e for e in alive if not e["isPlayer"]), None)
        log("Defeat. Your empire is no more.", "bad")
        return
    if len(alive) == 1 and alive[0]["isPlayer"]:
        state["over"] = True
        state["winner"] = p
        log("Victory! The galaxy is yours.", "good")
        return
    colonizable = sum(1 for s in state["systems"] if can_colonize(p, s))
    mine = len(empire_colonies(p["id"]))
    if state["turn"] > 20 and colonizable > 0 and mine / colonizable >= 0.55:
        state["over"] = True
        state["winner"] = p
        log("Galactic domination achieved!", "good")


# ---------------------------------------------------------------------------
# Save / load
# ---------------------------------------------------------------------------
def serialize():
    return json.dumps({
        "turn": state["turn"],
        "difficulty": state["difficulty"],
        "over": state["over"],
        "winner": state["winner"]["id"] if isinstance(state["winner"], dict) else state["winner"],
        "systems": [{
            **{k: v for k, v in s.items() if k not in ("exploredBy", "planet", "anomaly")},
            "exploredBy": sorted(s["exploredBy"]),
            "planet": {
                **{k: v for k, v in s["planet"].items() if k != "colony"},
                "colony": (None if not s["planet"]["colony"] else {
                    "empireId": s["planet"]["colony"]["empireId"],
                    "population": s["planet"]["colony"]["population"],
                    "buildings": list(s["planet"]["colony"]["buildings"]),
                    "queue": list(s["planet"]["colony"]["queue"]),
                    "queueProgress": s["planet"]["colony"]["queueProgress"],
                }),
            },
            "anomaly": s["anomaly"],
        } for s in state["systems"]],
        "empires": [{
            **{k: v for k, v in e.items() if k not in ("atWarWith", "peaceOffers", "techs", "lastIncome")},
            "atWarWith": sorted(e["atWarWith"]),
            "peaceOffers": list(e["peaceOffers"]),
            "techs": dict(e["techs"]),
            "lastIncome": dict(e["lastIncome"]),
        } for e in state["empires"]],
        "fleets": [{
            "id": f["id"], "empireId": f["empireId"], "systemId": f["systemId"],
            "x": f["x"], "y": f["y"],
            "ships": [{"cls": s["cls"], "hp": s["hp"]} for s in f["ships"]],
            "moves": dict(f["moves"]) if f["moves"] else None,
        } for f in state["fleets"]],
        "battles": [dict(b) for b in state["battles"]],
        "designs": {k: [dict(d) for d in v] for k, v in state["designs"].items()},
        "logs": state["logs"][-100:],
    })


def save_game():
    try:
        with open(SAVE_PATH, "w", encoding="utf-8") as fh:
            fh.write(serialize())
        toast("Game saved")
        return True
    except OSError as exc:
        toast("Save failed: %s" % exc)
        return False


def has_save():
    return os.path.exists(SAVE_PATH)


def load_game():
    global state
    try:
        with open(SAVE_PATH, "r", encoding="utf-8") as fh:
            data_raw = json.load(fh)
    except (OSError, ValueError):
        return False

    for s in data_raw["systems"]:
        s["exploredBy"] = set(s["exploredBy"])
    for e in data_raw["empires"]:
        e["atWarWith"] = set(e.get("atWarWith", []))
        e["peaceOffers"] = e.get("peaceOffers", [])
    data_raw.setdefault("designs", {})
    data_raw["selected"] = None
    state = data_raw
    return True
