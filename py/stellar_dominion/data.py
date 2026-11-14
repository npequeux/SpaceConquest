"""Stellar Dominion — static game data.

Pure tables and the system-name generator. No logic, no state.
"""

# --------------------------------------------------------------------------
# Star classes
# --------------------------------------------------------------------------
STAR_CLASSES = [
    {"key": "O", "name": "Blue Giant",        "color": "#9bb0ff", "radius": 1.55},
    {"key": "B", "name": "Blue-White Star",   "color": "#aabfff", "radius": 1.45},
    {"key": "A", "name": "White Star",        "color": "#cad8ff", "radius": 1.35},
    {"key": "F", "name": "Yellow-White Star", "color": "#f8f7ff", "radius": 1.25},
    {"key": "G", "name": "Yellow Star",       "color": "#fff4e0", "radius": 1.15},
    {"key": "K", "name": "Orange Star",       "color": "#ffd2a1", "radius": 1.05},
    {"key": "M", "name": "Red Dwarf",         "color": "#ffcc6f", "radius": 0.95},
]
STAR_BY_KEY = {s["key"]: s for s in STAR_CLASSES}

# --------------------------------------------------------------------------
# Planet types
# --------------------------------------------------------------------------
PLANET_TYPES = {
    "terran":   {"name": "Terran",    "color": "#4caf50", "habitability": 1.0,  "hostile": False},
    "ocean":    {"name": "Ocean",     "color": "#2196f3", "habitability": 0.9,  "hostile": False},
    "desert":   {"name": "Desert",    "color": "#ffb74d", "habitability": 0.7,  "hostile": False},
    "ice":      {"name": "Ice",       "color": "#b3e5fc", "habitability": 0.5,  "hostile": False},
    "barren":   {"name": "Barren",    "color": "#9e9e9e", "habitability": 0.3,  "hostile": False},
    "toxic":    {"name": "Toxic",     "color": "#9ccc65", "habitability": 0.2,  "hostile": True},
    "volcanic": {"name": "Volcanic",  "color": "#ff7043", "habitability": 0.15, "hostile": True},
    "gas":      {"name": "Gas Giant", "color": "#ce93d8", "habitability": 0.0,  "hostile": True},
}

# --------------------------------------------------------------------------
# Ships
# --------------------------------------------------------------------------
SHIPS = {
    "scout": {
        "name": "Scout", "hp": 25, "attack": 5, "speed": 420,
        "credits": 40, "minerals": 20, "buildTurns": 1,
        "tech": None, "colonize": False, "desc": "Fast exploration vessel",
    },
    "colony": {
        "name": "Colony Ship", "hp": 40, "attack": 0, "speed": 240,
        "credits": 140, "minerals": 90, "buildTurns": 3,
        "tech": None, "colonize": True, "desc": "Founds new colonies",
    },
    "frigate": {
        "name": "Frigate", "hp": 55, "attack": 12, "speed": 320,
        "credits": 80, "minerals": 45, "buildTurns": 2,
        "tech": None, "colonize": False, "desc": "Light combat ship",
    },
    "destroyer": {
        "name": "Destroyer", "hp": 110, "attack": 24, "speed": 270,
        "credits": 160, "minerals": 100, "buildTurns": 3,
        "tech": "construction1", "colonize": False, "desc": "Medium combat ship",
    },
    "cruiser": {
        "name": "Cruiser", "hp": 200, "attack": 42, "speed": 230,
        "credits": 300, "minerals": 200, "buildTurns": 4,
        "tech": "construction2", "colonize": False, "desc": "Heavy combat ship",
    },
    "battleship": {
        "name": "Battleship", "hp": 380, "attack": 75, "speed": 190,
        "credits": 560, "minerals": 380, "buildTurns": 6,
        "tech": "construction3", "colonize": False, "desc": "Capital ship",
    },
}

# --------------------------------------------------------------------------
# Buildings
# --------------------------------------------------------------------------
BUILDINGS = {
    "mine":     {"name": "Mine",             "cost": 60,  "repeat": True,  "desc": "+3 minerals/turn (scaled by richness)"},
    "market":   {"name": "Market",           "cost": 60,  "repeat": True,  "desc": "+4 credits/turn"},
    "lab":      {"name": "Research Lab",     "cost": 60,  "repeat": True,  "desc": "+5 research/turn"},
    "habitat":  {"name": "Habitat",          "cost": 80,  "repeat": True,  "desc": "+50% population growth"},
    "shipyard": {"name": "Shipyard",         "cost": 120, "repeat": False, "desc": "Enables ship construction"},
    "shield":   {"name": "Planetary Shield", "cost": 100, "repeat": False, "desc": "Defense platforms protect the colony"},
}

# --------------------------------------------------------------------------
# Technology tree
# --------------------------------------------------------------------------
TECHS = {
    "weapons1":      {"name": "Laser Cannons I",       "cat": "Weapons",    "cost": 30,  "requires": [],           "desc": "+15% fleet attack"},
    "weapons2":      {"name": "Laser Cannons II",      "cat": "Weapons",    "cost": 55,  "requires": ["weapons1"],  "desc": "+15% fleet attack"},
    "weapons3":      {"name": "Plasma Cannons",        "cat": "Weapons",    "cost": 90,  "requires": ["weapons2"],  "desc": "+25% fleet attack"},
    "shields1":      {"name": "Deflector Shields I",   "cat": "Defense",    "cost": 30,  "requires": [],            "desc": "+15% ship hit points"},
    "shields2":      {"name": "Deflector Shields II",  "cat": "Defense",    "cost": 55,  "requires": ["shields1"],  "desc": "+15% ship hit points"},
    "shields3":      {"name": "Advanced Shields",      "cat": "Defense",    "cost": 90,  "requires": ["shields2"],  "desc": "+25% ship hit points"},
    "engines1":      {"name": "Ion Engines I",         "cat": "Propulsion", "cost": 30,  "requires": [],            "desc": "+40 fleet speed"},
    "engines2":      {"name": "Ion Engines II",        "cat": "Propulsion", "cost": 55,  "requires": ["engines1"],  "desc": "+40 fleet speed"},
    "engines3":      {"name": "Warp Drive",            "cat": "Propulsion", "cost": 90,  "requires": ["engines2"],  "desc": "+80 fleet speed"},
    "construction1": {"name": "Advanced Construction", "cat": "Industry",   "cost": 35,  "requires": [],            "desc": "Unlock Destroyer, -10% ship cost"},
    "construction2": {"name": "Capital Shipyard",      "cat": "Industry",   "cost": 70,  "requires": ["construction1"], "desc": "Unlock Cruiser"},
    "construction3": {"name": "Titan Foundries",       "cat": "Industry",   "cost": 120, "requires": ["construction2"], "desc": "Unlock Battleship"},
    "colonization1": {"name": "Terraforming",          "cat": "Expansion",  "cost": 35,  "requires": [],            "desc": "+3 max population in all colonies"},
    "colonization2": {"name": "Xeno Colonization",     "cat": "Expansion",  "cost": 60,  "requires": ["colonization1"], "desc": "Colonize hostile worlds (Toxic / Volcanic)"},
    "colonization3": {"name": "Galactic Colonization",  "cat": "Expansion",  "cost": 100, "requires": ["colonization2"], "desc": "+4 max population, -25% colony ship cost"},
}
TECH_CATS = ["Weapons", "Defense", "Propulsion", "Industry", "Expansion"]

# --------------------------------------------------------------------------
# Empires
# --------------------------------------------------------------------------
EMPIRE_DEFS = [
    {"name": "Terran Consortium", "color": "#4dd0e1", "desc": "Humanity's united frontier command"},
    {"name": "Vex Imperium",      "color": "#ef5350", "desc": "Expansionist machine intelligence"},
    {"name": "Aurelian Dominion", "color": "#ffd54f", "desc": "Ancient noble houses"},
    {"name": "Krell Syndicate",   "color": "#ab47bc", "desc": "Ruthless trade cartel"},
    {"name": "Nova Collective",   "color": "#81c784", "desc": "Hive-minded flora"},
]

# --------------------------------------------------------------------------
# Anomalies — one-time rewards for the first empire that surveys a system
# --------------------------------------------------------------------------
ANOMALIES = [
    {"name": "Derelict Freighter", "desc": "A pre-collapse hauler, intact enough to strip.", "reward": {"credits": 120}},
    {"name": "Abandoned Mine",     "desc": "A rich mineral seam left by an unknown civilisation.", "reward": {"minerals": 150}},
    {"name": "Ancient Data Core",  "desc": "Research data of immense value.", "reward": {"research": 80}},
    {"name": "Ice Comet",          "desc": "Water and volatiles ready to exploit.", "reward": {"credits": 60, "minerals": 70}},
    {"name": "Strange Signal",     "desc": "Unusual readings from a nearby source.", "reward": {"research": 50, "credits": 60}},
    {"name": "Pirate Cache",       "desc": "A hidden stash of plundered goods.", "reward": {"credits": 100, "minerals": 80}},
]

# --------------------------------------------------------------------------
# System name generator
# --------------------------------------------------------------------------
_PRE = [
    "Al", "Be", "Cas", "De", "Er", "Fom", "Gal", "Hy", "Io", "Ja", "Ka", "Ly",
    "Me", "Nu", "Om", "Pro", "Qu", "Rho", "Sig", "Tau", "Ul", "Veg", "Wol", "Xi",
    "Ygg", "Zet", "Alph", "Bet", "Gamm", "Delt", "Eps", "Thet", "Iot", "Kap",
    "Lam", "Nix", "Oxi", "Rig", "Sir", "Ups", "Phi", "Chi", "Psi", "Ome", "Vex",
    "Krell", "Aur", "Nym", "Zor",
]
_SUF = [
    "ara", "eon", "is", "os", "us", "ar", "en", "or", "ax", "ix", "on", " Prime",
    " Major", " Minor", " Nova", " Cluster", " Drift", " Reach", " Gate", " Haven",
    " Fall", " Rise", " Deep", " Span", " Veil", " Expanse", " Hollow", " Crown",
    " Shard", " Rest",
]


def system_name(used):
    """Return a unique system name, adding it to ``used``."""
    import random

    for _ in range(100):
        n = random.choice(_PRE) + random.choice(_SUF)
        if n not in used:
            used.add(n)
            return n
    return "System %d" % random.randint(100, 999)


# --------------------------------------------------------------------------
# Ship designer
# --------------------------------------------------------------------------
HULLS = [
    {"key": "frigate",    "name": "Frigate Hull",    "hp": 60,  "speed": 320, "cost": 70,  "slots": 2, "tech": None,            "desc": "Fast and cheap"},
    {"key": "destroyer",  "name": "Destroyer Hull",  "hp": 130, "speed": 280, "cost": 150, "slots": 3, "tech": "construction1", "desc": "Balanced escort"},
    {"key": "cruiser",    "name": "Cruiser Hull",    "hp": 250, "speed": 240, "cost": 280, "slots": 4, "tech": "construction2", "desc": "Heavy line ship"},
    {"key": "battleship", "name": "Battleship Hull", "hp": 430, "speed": 200, "cost": 450, "slots": 5, "tech": "construction3", "desc": "Capital ship"},
]
HULL_BY_KEY = {h["key"]: h for h in HULLS}

WEAPON_MODULES = [
    {"key": "w0", "name": "Unarmed",       "attack": 0,  "cost": 0,   "tech": None,      "desc": "No weaponry"},
    {"key": "w1", "name": "Laser Battery", "attack": 18, "cost": 45,  "tech": None,      "desc": "+18 attack"},
    {"key": "w2", "name": "Plasma Cannon", "attack": 38, "cost": 110, "tech": "weapons3", "desc": "+38 attack"},
]
WEAPON_BY_KEY = {m["key"]: m for m in WEAPON_MODULES}

ARMOR_MODULES = [
    {"key": "a0", "name": "Light Plating",   "hpMul": 1.0,  "cost": 0,   "tech": None,      "desc": "Stock armour"},
    {"key": "a1", "name": "Reinforced Hull", "hpMul": 1.45, "cost": 55,  "tech": "shields1", "desc": "+45% hit points"},
    {"key": "a2", "name": "Advanced Armour", "hpMul": 2.2,  "cost": 130, "tech": "shields3", "desc": "+120% hit points"},
]
ARMOR_BY_KEY = {m["key"]: m for m in ARMOR_MODULES}

ENGINE_MODULES = [
    {"key": "e0", "name": "Basic Drive", "speed": 0,   "cost": 0,  "tech": None,      "desc": "Stock engines"},
    {"key": "e1", "name": "Ion Drive",   "speed": 50,  "cost": 40, "tech": "engines1", "desc": "+50 speed"},
    {"key": "e2", "name": "Warp Drive",  "speed": 120, "cost": 95, "tech": "engines3", "desc": "+120 speed"},
]
ENGINE_BY_KEY = {m["key"]: m for m in ENGINE_MODULES}


def compute_design(name, hull_key, weapon_key, armor_key, engine_key):
    """Combine a hull and three modules into a concrete ship definition."""
    hull = HULL_BY_KEY[hull_key]
    w = WEAPON_BY_KEY[weapon_key]
    a = ARMOR_BY_KEY[armor_key]
    e = ENGINE_BY_KEY[engine_key]
    credits = hull["cost"] + w["cost"] + a["cost"] + e["cost"]
    return {
        "name": name,
        "hull": hull_key, "weapon": weapon_key, "armor": armor_key, "engine": engine_key,
        "hp": round(hull["hp"] * a["hpMul"]),
        "attack": w["attack"],
        "speed": hull["speed"] + e["speed"],
        "credits": credits,
        "minerals": round(credits * 0.6),
        "buildTurns": hull["slots"] + 1,
        "tech": hull["tech"],
        "colonize": False,
        "desc": "%s · %s · %s · %s" % (hull["name"], w["name"], a["name"], e["name"]),
    }
