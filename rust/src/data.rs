//! Static game data: star classes, planets, ships, buildings, tech, empires,
//! anomalies and the ship designer. Pure tables — no state, no logic.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Stars
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct StarClass {
    pub key: &'static str,
    pub name: &'static str,
    pub color: &'static str,
    pub radius: f32,
}

pub fn star_classes() -> &'static [StarClass] {
    static T: OnceLock<Vec<StarClass>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            StarClass { key: "O", name: "Blue Giant",        color: "#9bb0ff", radius: 1.55 },
            StarClass { key: "B", name: "Blue-White Star",   color: "#aabfff", radius: 1.45 },
            StarClass { key: "A", name: "White Star",        color: "#cad8ff", radius: 1.35 },
            StarClass { key: "F", name: "Yellow-White Star", color: "#f8f7ff", radius: 1.25 },
            StarClass { key: "G", name: "Yellow Star",       color: "#fff4e0", radius: 1.15 },
            StarClass { key: "K", name: "Orange Star",       color: "#ffd2a1", radius: 1.05 },
            StarClass { key: "M", name: "Red Dwarf",         color: "#ffcc6f", radius: 0.95 },
        ]
    })
}

pub fn star_by_key(key: &str) -> Option<&'static StarClass> {
    star_classes().iter().find(|s| s.key == key)
}

// ---------------------------------------------------------------------------
// Planets
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct PlanetType {
    pub key: &'static str,
    pub name: &'static str,
    pub color: &'static str,
    pub habitability: f32,
    pub hostile: bool,
}

pub fn planet_types() -> &'static [PlanetType] {
    static T: OnceLock<Vec<PlanetType>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            PlanetType { key: "terran",   name: "Terran",      color: "#4caf50", habitability: 1.0,  hostile: false },
            PlanetType { key: "ocean",    name: "Ocean",       color: "#2196f3", habitability: 0.9,  hostile: false },
            PlanetType { key: "desert",   name: "Desert",      color: "#ffb74d", habitability: 0.7,  hostile: false },
            PlanetType { key: "ice",      name: "Ice",         color: "#b3e5fc", habitability: 0.5,  hostile: false },
            PlanetType { key: "barren",   name: "Barren",      color: "#9e9e9e", habitability: 0.3,  hostile: false },
            PlanetType { key: "toxic",    name: "Toxic",       color: "#9ccc65", habitability: 0.2,  hostile: true  },
            PlanetType { key: "volcanic", name: "Volcanic",    color: "#ff7043", habitability: 0.15, hostile: true  },
            PlanetType { key: "gas",      name: "Gas Giant",   color: "#ce93d8", habitability: 0.0,  hostile: true  },
        ]
    })
}

pub fn planet_type(key: &str) -> Option<&'static PlanetType> {
    planet_types().iter().find(|p| p.key == key)
}

// ---------------------------------------------------------------------------
// Stock ship classes
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct ShipClass {
    pub key: &'static str,
    pub name: &'static str,
    pub hp: f32,
    pub attack: f32,
    pub speed: f32,
    pub credits: i32,
    pub minerals: i32,
    pub build_turns: i32,
    pub tech: Option<&'static str>,
    pub colonize: bool,
    pub desc: &'static str,
}

pub fn ship_classes() -> &'static [ShipClass] {
    static T: OnceLock<Vec<ShipClass>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            ShipClass { key: "scout",      name: "Scout",       hp: 25.0,  attack: 5.0,  speed: 420.0, credits: 40,  minerals: 20,  build_turns: 1, tech: None,             colonize: false, desc: "Fast exploration vessel" },
            ShipClass { key: "colony",     name: "Colony Ship", hp: 40.0,  attack: 0.0,  speed: 240.0, credits: 140, minerals: 90,  build_turns: 3, tech: None,             colonize: true,  desc: "Founds new colonies" },
            ShipClass { key: "frigate",    name: "Frigate",     hp: 55.0,  attack: 12.0, speed: 320.0, credits: 80,  minerals: 45,  build_turns: 2, tech: None,             colonize: false, desc: "Light combat ship" },
            ShipClass { key: "destroyer",  name: "Destroyer",   hp: 110.0, attack: 24.0, speed: 270.0, credits: 160, minerals: 100, build_turns: 3, tech: Some("construction1"), colonize: false, desc: "Medium combat ship" },
            ShipClass { key: "cruiser",    name: "Cruiser",     hp: 200.0, attack: 42.0, speed: 230.0, credits: 300, minerals: 200, build_turns: 4, tech: Some("construction2"), colonize: false, desc: "Heavy combat ship" },
            ShipClass { key: "battleship", name: "Battleship",  hp: 380.0, attack: 75.0, speed: 190.0, credits: 560, minerals: 380, build_turns: 6, tech: Some("construction3"), colonize: false, desc: "Capital ship" },
        ]
    })
}

pub fn ship_class(key: &str) -> Option<&'static ShipClass> {
    ship_classes().iter().find(|s| s.key == key)
}

// ---------------------------------------------------------------------------
// Buildings
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Building {
    pub key: &'static str,
    pub name: &'static str,
    pub cost: i32,
    pub repeat: bool,
    pub desc: &'static str,
}

pub fn buildings() -> &'static [Building] {
    static T: OnceLock<Vec<Building>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            Building { key: "mine",     name: "Mine",             cost: 60,  repeat: true,  desc: "+3 minerals/turn (scaled by richness)" },
            Building { key: "market",   name: "Market",           cost: 60,  repeat: true,  desc: "+4 credits/turn" },
            Building { key: "lab",      name: "Research Lab",     cost: 60,  repeat: true,  desc: "+5 research/turn" },
            Building { key: "habitat",  name: "Habitat",          cost: 80,  repeat: true,  desc: "+50% population growth" },
            Building { key: "shipyard", name: "Shipyard",         cost: 120, repeat: false, desc: "Enables ship construction" },
            Building { key: "shield",   name: "Planetary Shield", cost: 100, repeat: false, desc: "Defense platforms protect the colony" },
        ]
    })
}

pub fn building(key: &str) -> Option<&'static Building> {
    buildings().iter().find(|b| b.key == key)
}

// ---------------------------------------------------------------------------
// Technology
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Tech {
    pub key: &'static str,
    pub name: &'static str,
    pub cat: &'static str,
    pub cost: i32,
    /// indices into the tech table, resolved by `tech_requires`
    pub requires: &'static [&'static str],
    pub desc: &'static str,
}

pub fn techs() -> &'static [Tech] {
    static T: OnceLock<Vec<Tech>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            Tech { key: "weapons1",      name: "Laser Cannons I",       cat: "Weapons",    cost: 30,  requires: &[],                        desc: "+15% fleet attack" },
            Tech { key: "weapons2",      name: "Laser Cannons II",      cat: "Weapons",    cost: 55,  requires: &["weapons1"],             desc: "+15% fleet attack" },
            Tech { key: "weapons3",      name: "Plasma Cannons",        cat: "Weapons",    cost: 90,  requires: &["weapons2"],             desc: "+25% fleet attack" },
            Tech { key: "shields1",      name: "Deflector Shields I",   cat: "Defense",    cost: 30,  requires: &[],                        desc: "+15% ship hit points" },
            Tech { key: "shields2",      name: "Deflector Shields II",  cat: "Defense",    cost: 55,  requires: &["shields1"],             desc: "+15% ship hit points" },
            Tech { key: "shields3",      name: "Advanced Shields",      cat: "Defense",    cost: 90,  requires: &["shields2"],             desc: "+25% ship hit points" },
            Tech { key: "engines1",      name: "Ion Engines I",         cat: "Propulsion", cost: 30,  requires: &[],                        desc: "+40 fleet speed" },
            Tech { key: "engines2",      name: "Ion Engines II",        cat: "Propulsion", cost: 55,  requires: &["engines1"],             desc: "+40 fleet speed" },
            Tech { key: "engines3",      name: "Warp Drive",            cat: "Propulsion", cost: 90,  requires: &["engines2"],             desc: "+80 fleet speed" },
            Tech { key: "construction1", name: "Advanced Construction", cat: "Industry",   cost: 35,  requires: &[],                        desc: "Unlock Destroyer, -10% ship cost" },
            Tech { key: "construction2", name: "Capital Shipyard",      cat: "Industry",   cost: 70,  requires: &["construction1"],        desc: "Unlock Cruiser" },
            Tech { key: "construction3", name: "Titan Foundries",       cat: "Industry",   cost: 120, requires: &["construction2"],        desc: "Unlock Battleship" },
            Tech { key: "colonization1", name: "Terraforming",          cat: "Expansion",  cost: 35,  requires: &[],                        desc: "+3 max population in all colonies" },
            Tech { key: "colonization2", name: "Xeno Colonization",     cat: "Expansion",  cost: 60,  requires: &["colonization1"],        desc: "Colonize hostile worlds (Toxic / Volcanic)" },
            Tech { key: "colonization3", name: "Galactic Colonization", cat: "Expansion",  cost: 100, requires: &["colonization2"],        desc: "+4 max population, -25% colony ship cost" },
        ]
    })
}

pub fn tech(key: &str) -> Option<&'static Tech> {
    techs().iter().find(|t| t.key == key)
}

pub const TECH_CATS: [&str; 5] = ["Weapons", "Defense", "Propulsion", "Industry", "Expansion"];

// ---------------------------------------------------------------------------
// Empires
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct EmpireDef {
    pub name: &'static str,
    pub color: &'static str,
    pub desc: &'static str,
}

pub fn empire_defs() -> &'static [EmpireDef] {
    static T: OnceLock<Vec<EmpireDef>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            EmpireDef { name: "Terran Consortium", color: "#4dd0e1", desc: "Humanity's united frontier command" },
            EmpireDef { name: "Vex Imperium",      color: "#ef5350", desc: "Expansionist machine intelligence" },
            EmpireDef { name: "Aurelian Dominion", color: "#ffd54f", desc: "Ancient noble houses" },
            EmpireDef { name: "Krell Syndicate",   color: "#ab47bc", desc: "Ruthless trade cartel" },
            EmpireDef { name: "Nova Collective",   color: "#81c784", desc: "Hive-minded flora" },
        ]
    })
}

// ---------------------------------------------------------------------------
// Anomalies
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Reward {
    pub credits: f32,
    pub minerals: f32,
    pub research: f32,
}

#[derive(Clone, Copy)]
pub struct AnomalyDef {
    pub name: &'static str,
    pub desc: &'static str,
    pub reward: Reward,
}

pub fn anomalies() -> &'static [AnomalyDef] {
    static T: OnceLock<Vec<AnomalyDef>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            AnomalyDef { name: "Derelict Freighter", desc: "A pre-collapse hauler, intact enough to strip.",        reward: Reward { credits: 120.0, minerals: 0.0,  research: 0.0 } },
            AnomalyDef { name: "Abandoned Mine",     desc: "A rich mineral seam left by an unknown civilisation.", reward: Reward { credits: 0.0,   minerals: 150.0, research: 0.0 } },
            AnomalyDef { name: "Ancient Data Core",  desc: "Research data of immense value.",                       reward: Reward { credits: 0.0,   minerals: 0.0,   research: 80.0 } },
            AnomalyDef { name: "Ice Comet",          desc: "Water and volatiles ready to exploit.",               reward: Reward { credits: 60.0,  minerals: 70.0,  research: 0.0 } },
            AnomalyDef { name: "Strange Signal",     desc: "Unusual readings from a nearby source.",               reward: Reward { credits: 60.0,  minerals: 0.0,   research: 50.0 } },
            AnomalyDef { name: "Pirate Cache",       desc: "A hidden stash of plundered goods.",                   reward: Reward { credits: 100.0, minerals: 80.0,  research: 0.0 } },
        ]
    })
}

// ---------------------------------------------------------------------------
// System names
// ---------------------------------------------------------------------------
const PRE: &[&str] = &[
    "Al", "Be", "Cas", "De", "Er", "Fom", "Gal", "Hy", "Io", "Ja", "Ka", "Ly",
    "Me", "Nu", "Om", "Pro", "Qu", "Rho", "Sig", "Tau", "Ul", "Veg", "Wol", "Xi",
    "Ygg", "Zet", "Alph", "Bet", "Gamm", "Delt", "Eps", "Thet", "Iot", "Kap",
    "Lam", "Nix", "Oxi", "Rig", "Sir", "Ups", "Phi", "Chi", "Psi", "Ome", "Vex",
    "Krell", "Aur", "Nym", "Zor",
];

const SUF: &[&str] = &[
    "ara", "eon", "is", "os", "us", "ar", "en", "or", "ax", "ix", "on", " Prime",
    " Major", " Minor", " Nova", " Cluster", " Drift", " Reach", " Gate", " Haven",
    " Fall", " Rise", " Deep", " Span", " Veil", " Expanse", " Hollow", " Crown",
    " Shard", " Rest",
];

pub fn system_name(used: &mut Vec<String>, rng: &mut impl rand::Rng) -> String {
    for _ in 0..100 {
        let n = format!("{}{}", PRE[rng.gen_range(0..PRE.len())], SUF[rng.gen_range(0..SUF.len())]);
        if !used.contains(&n) {
            used.push(n.clone());
            return n;
        }
    }
    let n = format!("System {}", rng.gen_range(100..1000));
    used.push(n.clone());
    n
}

// ---------------------------------------------------------------------------
// Ship designer
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct Hull {
    pub key: &'static str,
    pub name: &'static str,
    pub hp: f32,
    pub speed: f32,
    pub cost: i32,
    pub slots: i32,
    pub tech: Option<&'static str>,
    pub desc: &'static str,
}

pub fn hulls() -> &'static [Hull] {
    static T: OnceLock<Vec<Hull>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            Hull { key: "frigate",    name: "Frigate Hull",    hp: 60.0,  speed: 320.0, cost: 70,  slots: 2, tech: None,             desc: "Fast and cheap" },
            Hull { key: "destroyer",  name: "Destroyer Hull",  hp: 130.0, speed: 280.0, cost: 150, slots: 3, tech: Some("construction1"), desc: "Balanced escort" },
            Hull { key: "cruiser",    name: "Cruiser Hull",    hp: 250.0, speed: 240.0, cost: 280, slots: 4, tech: Some("construction2"), desc: "Heavy line ship" },
            Hull { key: "battleship", name: "Battleship Hull", hp: 430.0, speed: 200.0, cost: 450, slots: 5, tech: Some("construction3"), desc: "Capital ship" },
        ]
    })
}

pub fn hull(key: &str) -> Option<&'static Hull> {
    hulls().iter().find(|h| h.key == key)
}

#[derive(Clone, Copy)]
pub struct Module {
    pub key: &'static str,
    pub name: &'static str,
    pub attack: f32,
    pub hp_mul: f32,
    pub speed: f32,
    pub cost: i32,
    pub tech: Option<&'static str>,
    pub desc: &'static str,
}

pub fn weapon_modules() -> &'static [Module] {
    static T: OnceLock<Vec<Module>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            Module { key: "w0", name: "Unarmed",       attack: 0.0,  hp_mul: 1.0, speed: 0.0,   cost: 0,   tech: None,         desc: "No weaponry" },
            Module { key: "w1", name: "Laser Battery", attack: 18.0, hp_mul: 1.0, speed: 0.0,   cost: 45,  tech: None,         desc: "+18 attack" },
            Module { key: "w2", name: "Plasma Cannon", attack: 38.0, hp_mul: 1.0, speed: 0.0,   cost: 110, tech: Some("weapons3"), desc: "+38 attack" },
        ]
    })
}

pub fn armor_modules() -> &'static [Module] {
    static T: OnceLock<Vec<Module>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            Module { key: "a0", name: "Light Plating",   attack: 0.0, hp_mul: 1.0,  speed: 0.0, cost: 0,   tech: None,          desc: "Stock armour" },
            Module { key: "a1", name: "Reinforced Hull", attack: 0.0, hp_mul: 1.45, speed: 0.0, cost: 55,  tech: Some("shields1"), desc: "+45% hit points" },
            Module { key: "a2", name: "Advanced Armour", attack: 0.0, hp_mul: 2.2,  speed: 0.0, cost: 130, tech: Some("shields3"), desc: "+120% hit points" },
        ]
    })
}

pub fn engine_modules() -> &'static [Module] {
    static T: OnceLock<Vec<Module>> = OnceLock::new();
    T.get_or_init(|| {
        vec![
            Module { key: "e0", name: "Basic Drive", attack: 0.0, hp_mul: 1.0, speed: 0.0,   cost: 0,  tech: None,          desc: "Stock engines" },
            Module { key: "e1", name: "Ion Drive",   attack: 0.0, hp_mul: 1.0, speed: 50.0,  cost: 40, tech: Some("engines1"), desc: "+50 speed" },
            Module { key: "e2", name: "Warp Drive",  attack: 0.0, hp_mul: 1.0, speed: 120.0, cost: 95, tech: Some("engines3"), desc: "+120 speed" },
        ]
    })
}

pub fn module_by_key<'a>(mods: &'a [Module], key: &str) -> Option<&'a Module> {
    mods.iter().find(|m| m.key == key)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShipDesign {
    pub name: String,
    pub hull: String,
    pub weapon: String,
    pub armor: String,
    pub engine: String,
    pub hp: f32,
    pub attack: f32,
    pub speed: f32,
    pub credits: i32,
    pub minerals: i32,
    pub build_turns: i32,
    pub tech: Option<String>,
    pub colonize: bool,
    pub desc: String,
}

pub fn compute_design(name: &str, hull_key: &str, weapon_key: &str, armor_key: &str, engine_key: &str) -> ShipDesign {
    let h = hull(hull_key).unwrap_or(&hulls()[0]);
    let w = module_by_key(weapon_modules(), weapon_key).unwrap_or(&weapon_modules()[0]);
    let a = module_by_key(armor_modules(), armor_key).unwrap_or(&armor_modules()[0]);
    let e = module_by_key(engine_modules(), engine_key).unwrap_or(&engine_modules()[0]);
    let credits = h.cost + w.cost + a.cost + e.cost;
    ShipDesign {
        name: name.to_string(),
        hull: h.key.to_string(),
        weapon: w.key.to_string(),
        armor: a.key.to_string(),
        engine: e.key.to_string(),
        hp: (h.hp * a.hp_mul).round(),
        attack: w.attack,
        speed: h.speed + e.speed,
        credits,
        minerals: (credits as f32 * 0.6).round() as i32,
        build_turns: h.slots + 1,
        tech: h.tech.map(|s| s.to_string()),
        colonize: false,
        desc: format!("{} | {} | {} | {}", h.name, w.name, a.name, e.name),
    }
}

/// Resolve a ship class key (stock key, or `custom:<empire>:<index>`) to stats.
pub fn ship_def(cls: &str, designs: &std::collections::HashMap<String, Vec<ShipDesign>>) -> Option<ShipStats> {
    if let Some(rest) = cls.strip_prefix("custom:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            if let Ok(idx) = parts[1].parse::<usize>() {
                if let Some(d) = designs.get(parts[0]).and_then(|v| v.get(idx)) {
                    return Some(ShipStats {
                        name: d.name.clone(),
                        hp: d.hp,
                        attack: d.attack,
                        speed: d.speed,
                        credits: d.credits,
                        minerals: d.minerals,
                        build_turns: d.build_turns,
                        tech: d.tech.clone(),
                        colonize: d.colonize,
                        desc: d.desc.clone(),
                    });
                }
            }
        }
        return ship_class("frigate").map(|c| c.stats());
    }
    ship_class(cls).map(|c| c.stats())
}

#[derive(Clone)]
pub struct ShipStats {
    pub name: String,
    pub hp: f32,
    pub attack: f32,
    pub speed: f32,
    pub credits: i32,
    pub minerals: i32,
    pub build_turns: i32,
    pub tech: Option<String>,
    pub colonize: bool,
    pub desc: String,
}

impl ShipClass {
    pub fn stats(&self) -> ShipStats {
        ShipStats {
            name: self.name.to_string(),
            hp: self.hp,
            attack: self.attack,
            speed: self.speed,
            credits: self.credits,
            minerals: self.minerals,
            build_turns: self.build_turns,
            tech: self.tech.map(|s| s.to_string()),
            colonize: self.colonize,
            desc: self.desc.to_string(),
        }
    }
}
