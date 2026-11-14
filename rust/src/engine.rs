//! Core game state, rules, economy, movement, combat and diplomacy.
//!
//! The simulation is pure: it never touches the windowing layer. Rendering and
//! input live in `ui.rs`, AI in `ai.rs`.

use std::collections::HashMap;
use std::path::PathBuf;

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::data::{self, ShipDesign, ShipStats};
use crate::galaxy::{self, HEIGHT, WIDTH};
use crate::state::{
    BattleRec, Colony, Empire, Fleet, Income, LogEntry, Move, Selection, Ship, State, System,
};

pub const SENSOR_RANGE: f32 = 360.0;

pub fn save_path() -> PathBuf {
    let base = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(".stellar_dominion_save.json")
}

/// Bundles the mutable pieces a rule function needs, so signatures stay short.
pub struct Ctx<'a> {
    pub st: &'a mut State,
    pub rng: &'a mut StdRng,
    pub toasts: &'a mut Vec<String>,
}

impl<'a> Ctx<'a> {
    fn toast(&mut self, msg: impl Into<String>) {
        self.toasts.push(msg.into());
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------
pub fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    (ax - bx).hypot(ay - by)
}

fn log(st: &mut State, kind: &str, msg: impl Into<String>) {
    st.logs.push(LogEntry {
        turn: st.turn,
        msg: msg.into(),
        kind: kind.to_string(),
    });
    if st.logs.len() > 300 {
        let excess = st.logs.len() - 300;
        st.logs.drain(0..excess);
    }
}

fn record_battle(st: &mut State, rec: BattleRec) {
    st.battles.push(rec);
    if st.battles.len() > 30 {
        st.battles.remove(0);
    }
}

pub fn slots(system: &System) -> i32 {
    system.planet.size + 2
}

pub fn at_war(st: &State, a: usize, b: usize) -> bool {
    match (st.empire(a), st.empire(b)) {
        (Some(ea), Some(eb)) => ea.at_war_with.contains(&b) || eb.at_war_with.contains(&a),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Designs
// ---------------------------------------------------------------------------
pub fn list_designs(st: &State, eid: usize) -> Vec<String> {
    let key = eid.to_string();
    match st.designs.get(&key) {
        Some(v) => (0..v.len())
            .map(|i| format!("custom:{}:{}", eid, i))
            .collect(),
        None => Vec::new(),
    }
}

pub fn ship_def(st: &State, cls: &str) -> Option<ShipStats> {
    data::ship_def(cls, &st.designs)
}

pub fn add_design(
    st: &mut State,
    empire_id: usize,
    name: &str,
    hull_key: &str,
    weapon_key: &str,
    armor_key: &str,
    engine_key: &str,
) -> String {
    let d: ShipDesign = data::compute_design(name, hull_key, weapon_key, armor_key, engine_key);
    let key = empire_id.to_string();
    let designs = st.designs.entry(key).or_default();
    designs.push(d);
    format!("custom:{}:{}", empire_id, designs.len() - 1)
}

pub fn remove_design(st: &mut State, cls: &str) -> bool {
    let parts: Vec<&str> = cls.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    let idx: usize = match parts[2].parse() {
        Ok(i) => i,
        Err(_) => return false,
    };
    if let Some(designs) = st.designs.get_mut(parts[1]) {
        if idx < designs.len() {
            designs.remove(idx);
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tech modifiers and effective ship stats
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub struct TechMods {
    pub attack_mul: f32,
    pub hp_mul: f32,
    pub speed_bonus: f32,
    pub cost_mul: f32,
    pub max_pop_bonus: i32,
    pub can_hostile: bool,
    pub colony_cost_mul: f32,
}

pub fn tech_modifiers(empire: &Empire) -> TechMods {
    let t = &empire.techs;
    let mut m = TechMods {
        attack_mul: 1.0,
        hp_mul: 1.0,
        speed_bonus: 0.0,
        cost_mul: 1.0,
        max_pop_bonus: 0,
        can_hostile: false,
        colony_cost_mul: 1.0,
    };
    let has = |k: &str| t.iter().any(|x| x == k);
    if has("weapons1") {
        m.attack_mul *= 1.15;
    }
    if has("weapons2") {
        m.attack_mul *= 1.15;
    }
    if has("weapons3") {
        m.attack_mul *= 1.25;
    }
    if has("shields1") {
        m.hp_mul *= 1.15;
    }
    if has("shields2") {
        m.hp_mul *= 1.15;
    }
    if has("shields3") {
        m.hp_mul *= 1.25;
    }
    if has("engines1") {
        m.speed_bonus += 40.0;
    }
    if has("engines2") {
        m.speed_bonus += 40.0;
    }
    if has("engines3") {
        m.speed_bonus += 80.0;
    }
    if has("construction1") {
        m.cost_mul *= 0.9;
    }
    if has("colonization1") {
        m.max_pop_bonus += 3;
    }
    if has("colonization2") {
        m.can_hostile = true;
    }
    if has("colonization3") {
        m.max_pop_bonus += 4;
        m.colony_cost_mul *= 0.75;
    }
    m
}

pub fn effective_ship_stats(st: &State, empire: &Empire, cls: &str) -> ShipStats {
    let base = ship_def(st, cls).unwrap_or_else(|| data::ship_classes()[2].stats());
    let m = tech_modifiers(empire);
    ShipStats {
        name: base.name,
        hp: (base.hp * m.hp_mul).round(),
        attack: (base.attack * m.attack_mul).round(),
        speed: base.speed + m.speed_bonus,
        credits: base.credits,
        minerals: base.minerals,
        build_turns: base.build_turns,
        tech: base.tech,
        colonize: base.colonize,
        desc: base.desc,
    }
}

pub fn ship_cost(st: &State, empire: &Empire, cls: &str) -> (i32, i32) {
    let base = ship_def(st, cls).unwrap_or_else(|| data::ship_classes()[2].stats());
    let m = tech_modifiers(empire);
    let mul = if cls == "colony" {
        m.colony_cost_mul
    } else {
        m.cost_mul
    };
    (
        (base.credits as f32 * mul).round() as i32,
        (base.minerals as f32 * mul).round() as i32,
    )
}

pub fn fleet_speed(st: &State, fleet: &Fleet) -> f32 {
    if fleet.ships.is_empty() {
        return 100.0;
    }
    let e = match st.empire(fleet.empire_id) {
        Some(e) => e,
        None => return 100.0,
    };
    fleet
        .ships
        .iter()
        .map(|s| effective_ship_stats(st, e, &s.cls).speed)
        .fold(f32::INFINITY, f32::min)
}

pub fn fleet_strength(st: &State, fleet: &Fleet) -> f32 {
    let e = match st.empire(fleet.empire_id) {
        Some(e) => e,
        None => return 0.0,
    };
    let mut total = 0.0;
    for s in &fleet.ships {
        let stt = effective_ship_stats(st, e, &s.cls);
        total += stt.attack * 2.0 + s.hp * 0.5;
    }
    total
}

pub fn empire_strength(st: &State, eid: usize) -> f32 {
    st.fleets
        .iter()
        .filter(|f| f.empire_id == eid)
        .map(|f| fleet_strength(st, f))
        .sum()
}

pub fn empire_ship_count(st: &State, eid: usize) -> usize {
    st.fleets
        .iter()
        .filter(|f| f.empire_id == eid)
        .map(|f| f.ships.len())
        .sum()
}

pub fn max_pop(system: &System, empire: Option<&Empire>) -> i32 {
    system.planet.max_pop + empire.map(|e| tech_modifiers(e).max_pop_bonus).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Colony economy
// ---------------------------------------------------------------------------
pub fn colony_production(system: &System, col: &Colony) -> Income {
    let mines = col.buildings.iter().filter(|b| **b == "mine").count() as f32;
    let markets = col.buildings.iter().filter(|b| **b == "market").count() as f32;
    let labs = col.buildings.iter().filter(|b| **b == "lab").count() as f32;
    Income {
        minerals: 2.0 + mines * 3.0 * system.planet.richness,
        credits: 2.0 + markets * 4.0 + col.population * 0.8,
        research: 1.0 + labs * 5.0 + col.population * 0.3,
    }
}

pub fn colony_growth_rate(col: &Colony) -> f32 {
    0.05 * if col.buildings.iter().any(|b| b == "habitat") {
        1.5
    } else {
        1.0
    }
}

pub fn can_colonize(empire: &Empire, system: &System) -> bool {
    let pt = match data::planet_type(&system.planet.kind) {
        Some(p) => p,
        None => return false,
    };
    if pt.habitability <= 0.0 {
        return false;
    }
    if pt.hostile && !tech_modifiers(empire).can_hostile {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// New game
// ---------------------------------------------------------------------------
#[derive(Clone)]
pub struct NewGameOpts {
    pub num_ai: usize,
    pub num_systems: usize,
    pub difficulty: String,
    pub player_name: String,
}

/// A blank state, used as a placeholder so a fresh `Sim` can start a game.
pub fn empty_state() -> State {
    State {
        turn: 1,
        systems: Vec::new(),
        empires: Vec::new(),
        fleets: Vec::new(),
        logs: Vec::new(),
        battles: Vec::new(),
        designs: HashMap::new(),
        selected: None,
        over: false,
        winner: None,
        difficulty: String::new(),
    }
}

pub fn new_game(ctx: &mut Ctx, opts: NewGameOpts) {
    let (systems, homes) = galaxy::generate(opts.num_systems, opts.num_ai + 1, ctx.rng);

    let mut empires: Vec<Empire> = Vec::new();
    for i in 0..(opts.num_ai + 1) {
        let defn = data::empire_defs()[i % data::empire_defs().len()];
        empires.push(Empire {
            id: i,
            name: if i == 0 {
                opts.player_name.clone()
            } else {
                defn.name.to_string()
            },
            color: defn.color.to_string(),
            desc: defn.desc.to_string(),
            is_player: i == 0,
            credits: 250.0,
            minerals: 200.0,
            tech_progress: 0.0,
            research_pool: 0.0,
            techs: Vec::new(),
            current_tech: None,
            home_system_id: None,
            alive: true,
            at_war_with: Vec::new(),
            difficulty: opts.difficulty.clone(),
            aggression: ctx.rng.gen_range(0.35..0.9f32),
            last_income: Income::default(),
            peace_offers: Vec::new(),
        });
    }

    let mut st = State {
        turn: 1,
        systems,
        empires,
        fleets: Vec::new(),
        logs: Vec::new(),
        battles: Vec::new(),
        designs: HashMap::new(),
        selected: None,
        over: false,
        winner: None,
        difficulty: opts.difficulty.clone(),
    };

    // Scatter anomalies on unclaimed worlds
    for s in st.systems.iter_mut() {
        if s.owner_empire_id.is_none() && ctx.rng.gen::<f32>() < 0.28 {
            let a = data::anomalies()[ctx.rng.gen_range(0..data::anomalies().len())];
            s.anomaly = Some(crate::state::Anomaly {
                name: a.name.to_string(),
                desc: a.desc.to_string(),
                reward: crate::state::AnomalyReward {
                    credits: a.reward.credits,
                    minerals: a.reward.minerals,
                    research: a.reward.research,
                },
            });
        }
    }

    let mut uid = 0usize;
    let emp_ids: Vec<usize> = st.empires.iter().map(|e| e.id).collect();
    for (i, eid) in emp_ids.into_iter().enumerate() {
        let home_id = homes[i];
        if let Some(e) = st.empire_mut(eid) {
            e.home_system_id = Some(home_id);
        }
        let (hx, hy) = {
            let home = st.system_mut(home_id).unwrap();
            home.planet = galaxy::make_home_planet();
            home.owner_empire_id = Some(eid);
            home.planet.colony = Some(Colony {
                empire_id: eid,
                population: 8.0,
                buildings: vec![
                    "mine".into(),
                    "market".into(),
                    "lab".into(),
                    "shipyard".into(),
                ],
                queue: Vec::new(),
                queue_progress: 0,
            });
            home.explored_by.push(eid);
            (home.x, home.y)
        };

        uid += 1;
        let mut fleet = Fleet {
            id: uid,
            empire_id: eid,
            system_id: home_id,
            x: hx,
            y: hy,
            ships: Vec::new(),
            moves: None,
        };
        for cls in ["colony", "frigate", "scout"] {
            let emp = st.empire(eid).cloned();
            let hp = emp
                .as_ref()
                .map(|e| effective_ship_stats(&st, e, cls).hp)
                .unwrap_or(1.0);
            fleet.ships.push(Ship {
                cls: cls.to_string(),
                hp,
            });
        }
        st.fleets.push(fleet);
    }

    update_exploration(&mut st, ctx.rng, ctx.toasts);
    log(
        &mut st,
        "info",
        "Your empire rises. Expand, research, and conquer.",
    );
    log(
        &mut st,
        "info",
        "Build ships at colonies with a Shipyard. Select a fleet and click a destination to move.",
    );
    *ctx.st = st;
}

// ---------------------------------------------------------------------------
// Exploration
// ---------------------------------------------------------------------------
pub fn update_exploration(st: &mut State, rng: &mut StdRng, toasts: &mut Vec<String>) {
    let mut discoveries: Vec<(usize, usize)> = Vec::new(); // (empire, system)
    for e in st.empires.iter() {
        if !e.alive {
            continue;
        }
        let fleet_sources: Vec<(f32, f32)> = st
            .fleets
            .iter()
            .filter(|f| f.empire_id == e.id)
            .map(|f| (f.x, f.y))
            .collect();
        let colony_sources: Vec<(f32, f32)> = st
            .systems
            .iter()
            .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(e.id))
            .map(|s| (s.x, s.y))
            .collect();
        for s in st.systems.iter() {
            if s.explored_by.contains(&e.id) {
                continue;
            }
            let near_fleet = fleet_sources
                .iter()
                .any(|(x, y)| dist(*x, *y, s.x, s.y) < SENSOR_RANGE);
            let near_colony = colony_sources
                .iter()
                .any(|(x, y)| dist(*x, *y, s.x, s.y) < SENSOR_RANGE);
            if !near_fleet && !near_colony {
                continue;
            }
            discoveries.push((e.id, s.id));
            let _ = rng;
        }
    }
    for (eid, sid) in discoveries {
        let has_anomaly = st.system(sid).and_then(|s| s.anomaly.as_ref()).is_some();
        if let Some(s) = st.system_mut(sid) {
            s.explored_by.push(eid);
        }
        if has_anomaly {
            grant_anomaly(st, eid, sid, toasts);
        }
    }
}

fn grant_anomaly(st: &mut State, eid: usize, sid: usize, toasts: &mut Vec<String>) {
    let (name, desc, reward) = match st.system(sid).and_then(|s| s.anomaly.clone()) {
        Some(a) => (a.name, a.desc, a.reward),
        None => return,
    };
    if let Some(s) = st.system_mut(sid) {
        s.anomaly = None;
    }
    let e = match st.empire_mut(eid) {
        Some(e) => e,
        None => return,
    };
    e.credits += reward.credits;
    e.minerals += reward.minerals;
    e.research_pool += reward.research;
    let mut bits: Vec<String> = Vec::new();
    if reward.credits > 0.0 {
        bits.push(format!("{} credits", reward.credits as i32));
    }
    if reward.minerals > 0.0 {
        bits.push(format!("{} minerals", reward.minerals as i32));
    }
    if reward.research > 0.0 {
        bits.push(format!("{} research", reward.research as i32));
    }
    let is_player = e.is_player;
    let ename = e.name.clone();
    log(
        st,
        if is_player { "good" } else { "info" },
        format!(
            "{} surveyed {}: {} - {} (+{})",
            ename,
            st.system(sid).map(|s| s.name.clone()).unwrap_or_default(),
            name,
            desc,
            bits.join(", ")
        ),
    );
    if is_player {
        toasts.push(format!(
            "{} discovered at {}",
            name,
            st.system(sid).map(|s| s.name.clone()).unwrap_or_default()
        ));
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------
pub fn select(st: &mut State, sel: Option<Selection>) {
    st.selected = sel;
}

pub fn issue_move(st: &mut State, fleet_id: usize, target_system_id: usize) -> bool {
    let (from_id, has_moves) = match st.fleet(fleet_id) {
        Some(f) => (f.system_id, f.moves.is_some()),
        None => return false,
    };
    if from_id == target_system_id && !has_moves {
        return false;
    }
    let from = match st.system(from_id) {
        Some(s) => (s.x, s.y),
        None => return false,
    };
    let to = match st.system(target_system_id) {
        Some(s) => (s.x, s.y),
        None => return false,
    };
    let speed = st
        .fleet(fleet_id)
        .map(|f| fleet_speed(st, f))
        .unwrap_or(100.0);
    let d = dist(from.0, from.1, to.0, to.1);
    let turns = ((d / speed).ceil() as i32).max(1);
    if let Some(f) = st.fleet_mut(fleet_id) {
        f.moves = Some(Move {
            to_system_id: target_system_id,
            turns_left: turns,
            total_turns: turns,
        });
    }
    true
}

pub fn stop_fleet(st: &mut State, fleet_id: usize) {
    let sys_pos = match st.fleet(fleet_id) {
        Some(f) => st.system(f.system_id).map(|s| (s.x, s.y)),
        None => None,
    };
    if let (Some(f), Some((x, y))) = (st.fleet_mut(fleet_id), sys_pos) {
        f.moves = None;
        f.x = x;
        f.y = y;
    }
}

pub fn split_fleet(st: &mut State, fleet_id: usize) -> bool {
    let (empire_id, system_id, x, y, moves, half) = match st.fleet(fleet_id) {
        Some(f) if f.ships.len() >= 2 => {
            let half = (f.ships.len() as f32 / 2.0).ceil() as usize;
            (f.empire_id, f.system_id, f.x, f.y, f.moves.clone(), half)
        }
        _ => return false,
    };
    let new_ships: Vec<Ship> = match st.fleet(fleet_id) {
        Some(f) => f.ships[..half].to_vec(),
        None => return false,
    };
    if let Some(f) = st.fleet_mut(fleet_id) {
        f.ships = f.ships[half..].to_vec();
    }
    let max_id = st.fleets.iter().map(|f| f.id).max().unwrap_or(0);
    st.fleets.push(Fleet {
        id: max_id + 1,
        empire_id,
        system_id,
        x,
        y,
        ships: new_ships,
        moves,
    });
    if let Some(s) = st.system(system_id) {
        log(st, "info", format!("Fleet split at {}.", s.name));
    }
    true
}

pub fn merge_fleets(st: &mut State, target_id: usize, source_id: usize) -> bool {
    if target_id == source_id {
        return false;
    }
    let (t_emp, t_sys, t_moves, s_emp, s_sys, s_moves) =
        match (st.fleet(target_id), st.fleet(source_id)) {
            (Some(t), Some(s)) => (
                t.empire_id,
                t.system_id,
                t.moves.is_some(),
                s.empire_id,
                s.system_id,
                s.moves.is_some(),
            ),
            _ => return false,
        };
    if t_emp != s_emp || t_sys != s_sys || t_moves || s_moves {
        return false;
    }
    let source_ships = match st.fleet(source_id) {
        Some(f) => f.ships.clone(),
        None => return false,
    };
    if let Some(t) = st.fleet_mut(target_id) {
        t.ships.extend(source_ships);
    }
    st.fleets.retain(|f| f.id != source_id);
    if t_emp == 0 {
        let name = st.system(t_sys).map(|s| s.name.clone()).unwrap_or_default();
        log(st, "info", format!("Fleets merged at {}.", name));
    }
    true
}

pub fn merge_all_fleets_at(st: &mut State, system_id: usize, empire_id: usize) -> bool {
    let ids: Vec<usize> = st
        .fleets
        .iter()
        .filter(|f| f.empire_id == empire_id && f.system_id == system_id && f.moves.is_none())
        .map(|f| f.id)
        .collect();
    if ids.len() < 2 {
        return false;
    }
    let target = ids[0];
    let mut extra: Vec<Ship> = Vec::new();
    for id in &ids[1..] {
        if let Some(f) = st.fleet(*id) {
            extra.extend(f.ships.clone());
        }
    }
    if let Some(t) = st.fleet_mut(target) {
        t.ships.extend(extra);
    }
    st.fleets.retain(|f| f.id == target || !ids.contains(&f.id));
    if empire_id == 0 {
        let name = st
            .system(system_id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        log(st, "info", format!("All fleets merged at {}.", name));
    }
    true
}

pub fn build_building(
    st: &mut State,
    system_id: usize,
    key: &str,
    toasts: &mut Vec<String>,
) -> bool {
    let (colony, empire_id) = match st.system(system_id) {
        Some(s) => match &s.planet.colony {
            Some(c) => (c.clone(), c.empire_id),
            None => return false,
        },
        None => return false,
    };
    let b = match data::building(key) {
        Some(b) => b,
        None => return false,
    };
    let empire = match st.empire(empire_id) {
        Some(e) => e.clone(),
        None => return false,
    };
    if colony.buildings.len() as i32 >= slots(st.system(system_id).unwrap()) {
        if empire.is_player {
            toasts.push("No free building slots".into());
        }
        return false;
    }
    if !b.repeat && colony.buildings.iter().any(|x| x == key) {
        return false;
    }
    if empire.minerals < b.cost as f32 {
        if empire.is_player {
            toasts.push("Not enough minerals".into());
        }
        return false;
    }
    let name = b.name.to_string();
    let sys_name = st
        .system(system_id)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    if let Some(e) = st.empire_mut(empire_id) {
        e.minerals -= b.cost as f32;
    }
    if let Some(s) = st.system_mut(system_id) {
        if let Some(c) = s.planet.colony.as_mut() {
            c.buildings.push(key.to_string());
        }
    }
    if empire.is_player {
        log(st, "build", format!("{} built at {}.", name, sys_name));
    }
    true
}

pub fn queue_ship(st: &mut State, system_id: usize, key: &str, toasts: &mut Vec<String>) -> bool {
    let (colony, empire_id) = match st.system(system_id) {
        Some(s) => match &s.planet.colony {
            Some(c) => (c.clone(), c.empire_id),
            None => return false,
        },
        None => return false,
    };
    let empire = match st.empire(empire_id) {
        Some(e) => e.clone(),
        None => return false,
    };
    let ship = match ship_def(st, key) {
        Some(s) => s,
        None => return false,
    };
    if !colony.buildings.iter().any(|b| b == "shipyard") {
        if empire.is_player {
            toasts.push("Requires a Shipyard".into());
        }
        return false;
    }
    if colony.queue.len() >= 6 {
        if empire.is_player {
            toasts.push("Queue is full".into());
        }
        return false;
    }
    if let Some(t) = &ship.tech {
        if !empire.techs.iter().any(|x| x == t) {
            if empire.is_player {
                toasts.push("Technology not researched".into());
            }
            return false;
        }
    }
    let (credits, minerals) = ship_cost(st, &empire, key);
    if empire.credits < credits as f32 || empire.minerals < minerals as f32 {
        if empire.is_player {
            toasts.push("Not enough resources".into());
        }
        return false;
    }
    let sname = ship.name.clone();
    let sys_name = st
        .system(system_id)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    if let Some(e) = st.empire_mut(empire_id) {
        e.credits -= credits as f32;
        e.minerals -= minerals as f32;
    }
    if let Some(s) = st.system_mut(system_id) {
        if let Some(c) = s.planet.colony.as_mut() {
            c.queue.push(key.to_string());
        }
    }
    if empire.is_player {
        log(st, "build", format!("{} queued at {}.", sname, sys_name));
    }
    true
}

pub fn set_research(st: &mut State, empire_id: usize, tech_key: &str) -> bool {
    let tech = match data::tech(tech_key) {
        Some(t) => t,
        None => return false,
    };
    let empire = match st.empire(empire_id) {
        Some(e) => e,
        None => return false,
    };
    if empire.techs.iter().any(|t| t == tech_key) {
        return false;
    }
    if !tech
        .requires
        .iter()
        .all(|r| empire.techs.iter().any(|t| t == r))
    {
        return false;
    }
    let name = tech.name.to_string();
    if let Some(e) = st.empire_mut(empire_id) {
        e.current_tech = Some(tech_key.to_string());
        e.tech_progress = 0.0;
    }
    log(st, "research", format!("Research started: {}.", name));
    true
}

pub fn declare_war(st: &mut State, a: usize, b: usize, toasts: &mut Vec<String>) {
    if a == b {
        return;
    }
    let (an, bn, a_player, b_player) = match (st.empire(a), st.empire(b)) {
        (Some(ea), Some(eb)) => (ea.name.clone(), eb.name.clone(), ea.is_player, eb.is_player),
        _ => return,
    };
    let already = st
        .empire(a)
        .map(|e| e.at_war_with.contains(&b))
        .unwrap_or(false);
    if let Some(ea) = st.empire_mut(a) {
        ea.at_war_with.push(b);
        ea.peace_offers.retain(|x| *x != b);
    }
    if let Some(eb) = st.empire_mut(b) {
        eb.at_war_with.push(a);
        eb.peace_offers.retain(|x| *x != a);
    }
    if !already {
        log(st, "war", format!("{} declares war on {}!", an, bn));
        if a_player || b_player {
            toasts.push(format!("War! {} <> {}", an, bn));
        }
    }
}

pub fn make_peace(st: &mut State, a: usize, b: usize) {
    let (an, bn) = match (st.empire(a), st.empire(b)) {
        (Some(ea), Some(eb)) => (ea.name.clone(), eb.name.clone()),
        _ => return,
    };
    if let Some(ea) = st.empire_mut(a) {
        ea.at_war_with.retain(|x| *x != b);
        ea.peace_offers.retain(|x| *x != b);
    }
    if let Some(eb) = st.empire_mut(b) {
        eb.at_war_with.retain(|x| *x != a);
        eb.peace_offers.retain(|x| *x != a);
    }
    log(
        st,
        "good",
        format!("Peace signed between {} and {}.", an, bn),
    );
}

pub fn offer_peace(st: &mut State, from_id: usize, to_id: usize) -> bool {
    let at_war = st
        .empire(from_id)
        .map(|e| e.at_war_with.contains(&to_id))
        .unwrap_or(false);
    if !at_war {
        return false;
    }
    let (from_name, to_name) = match (st.empire(from_id), st.empire(to_id)) {
        (Some(a), Some(b)) => (a.name.clone(), b.name.clone()),
        _ => return false,
    };
    if let Some(e) = st.empire_mut(to_id) {
        if !e.peace_offers.contains(&from_id) {
            e.peace_offers.push(from_id);
        }
    }
    log(
        st,
        "info",
        format!("{} offers peace to {}.", from_name, to_name),
    );
    true
}

pub fn player_sue_peace(
    st: &mut State,
    other_id: usize,
    rng: &mut StdRng,
    toasts: &mut Vec<String>,
) -> bool {
    let me = match st.player() {
        p => p.clone(),
    };
    if !me.at_war_with.contains(&other_id) {
        return false;
    }
    let my_str = empire_strength(st, me.id);
    let their_str = empire_strength(st, other_id);
    let other_name = st
        .empire(other_id)
        .map(|e| e.name.clone())
        .unwrap_or_default();
    if their_str < my_str * 0.7 || rng.gen::<f32>() < 0.5 {
        make_peace(st, me.id, other_id);
        toasts.push(format!("{} accepts peace", other_name));
        true
    } else {
        log(
            st,
            "bad",
            format!("{} rejects your peace offer.", other_name),
        );
        toasts.push(format!("{} rejects peace", other_name));
        false
    }
}

// ---------------------------------------------------------------------------
// Turn processing
// ---------------------------------------------------------------------------
pub fn end_turn(ctx: &mut Ctx) {
    if ctx.st.over {
        return;
    }
    let Ctx { st, rng, toasts } = ctx;

    // 1) AI empires take their turns
    let ai_ids: Vec<usize> = st
        .empires
        .iter()
        .filter(|e| !e.is_player && e.alive)
        .map(|e| e.id)
        .collect();
    for eid in ai_ids {
        crate::ai::take_turn(st, eid, rng, toasts);
    }

    // 2) Move every fleet one step
    let fleet_ids: Vec<usize> = st.fleets.iter().map(|f| f.id).collect();
    for fid in fleet_ids {
        let (from, to, mut moves) = match st.fleet(fid) {
            Some(f) => match &f.moves {
                Some(m) => (f.system_id, Some(m.to_system_id), Some(m.clone())),
                None => (f.system_id, None, None),
            },
            None => continue,
        };
        if let Some(m) = moves.as_mut() {
            m.turns_left -= 1;
            let t = 1.0 - m.turns_left as f32 / m.total_turns as f32;
            let (fx, fy) = match st.system(from) {
                Some(s) => (s.x, s.y),
                None => continue,
            };
            let (tx, ty) = match st.system(to.unwrap_or(from)) {
                Some(s) => (s.x, s.y),
                None => continue,
            };
            if let Some(f) = st.fleet_mut(fid) {
                f.x = fx + (tx - fx) * t;
                f.y = fy + (ty - fy) * t;
            }
            let arrived = m.turns_left <= 0;
            if arrived {
                let dest = to.unwrap_or(from);
                if let Some(f) = st.fleet_mut(fid) {
                    f.system_id = dest;
                    f.x = tx;
                    f.y = ty;
                    f.moves = None;
                }
                arrive_fleet(st, fid, toasts);
            } else if let Some(f) = st.fleet_mut(fid) {
                f.moves = Some(m.clone());
            }
        } else {
            if let Some(s) = st.system(from) {
                let (x, y) = (s.x, s.y);
                if let Some(f) = st.fleet_mut(fid) {
                    f.x = x;
                    f.y = y;
                }
            }
        }
    }

    // 3) Merge idle fleets of the same empire at the same system
    auto_merge_fleets(st);

    // 4) Combat
    resolve_combat(st, rng, toasts);

    // 5) Income, growth, research
    let emp_ids: Vec<usize> = st
        .empires
        .iter()
        .filter(|e| e.alive)
        .map(|e| e.id)
        .collect();
    for eid in emp_ids {
        apply_income(st, eid);
    }

    // 6) Construction
    progress_queues(st, toasts);

    // 7) Drop empty fleets
    st.fleets.retain(|f| !f.ships.is_empty());

    // 8) Exploration
    update_exploration(st, rng, toasts);

    // 9) Elimination
    update_alive(st, toasts);

    // 10) Advance and check victory
    st.turn += 1;
    check_victory(st, toasts);
}

fn auto_merge_fleets(st: &mut State) {
    let mut groups: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for f in st.fleets.iter() {
        if f.moves.is_some() {
            continue;
        }
        groups
            .entry((f.empire_id, f.system_id))
            .or_default()
            .push(f.id);
    }
    for ids in groups.values() {
        if ids.len() < 2 {
            continue;
        }
        let target = ids[0];
        let mut extra: Vec<Ship> = Vec::new();
        for id in &ids[1..] {
            if let Some(f) = st.fleet(*id) {
                extra.extend(f.ships.clone());
            }
        }
        if let Some(t) = st.fleet_mut(target) {
            t.ships.extend(extra);
        }
        st.fleets.retain(|f| f.id == target || !ids.contains(&f.id));
    }
}

fn arrive_fleet(st: &mut State, fleet_id: usize, toasts: &mut Vec<String>) {
    let (empire_id, sys_id) = match st.fleet(fleet_id) {
        Some(f) => (f.empire_id, f.system_id),
        None => return,
    };
    let colonize_idx = match st.fleet(fleet_id) {
        Some(f) => f
            .ships
            .iter()
            .position(|s| ship_def(st, &s.cls).map(|d| d.colonize).unwrap_or(false)),
        None => None,
    };
    let can_found = match (st.system(sys_id), st.empire(empire_id)) {
        (Some(s), Some(e)) => s.planet.colony.is_none() && can_colonize(e, s),
        _ => false,
    };
    if let (Some(idx), true) = (colonize_idx, can_found) {
        if let Some(f) = st.fleet_mut(fleet_id) {
            f.ships.remove(idx);
        }
        let sys_name = st
            .system(sys_id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        if let Some(s) = st.system_mut(sys_id) {
            s.planet.colony = Some(Colony {
                empire_id,
                population: 2.0,
                buildings: Vec::new(),
                queue: Vec::new(),
                queue_progress: 0,
            });
            s.owner_empire_id = Some(empire_id);
            s.explored_by.push(empire_id);
        }
        let ename = st
            .empire(empire_id)
            .map(|e| e.name.clone())
            .unwrap_or_default();
        let is_player = st.empire(empire_id).map(|e| e.is_player).unwrap_or(false);
        log(
            st,
            if is_player { "good" } else { "info" },
            format!("{} founded a colony at {}.", ename, sys_name),
        );
        if is_player {
            toasts.push(format!("Colony founded at {}!", sys_name));
        }
    }
}

fn apply_income(st: &mut State, eid: usize) {
    let difficulty = st
        .empire(eid)
        .map(|e| e.difficulty.clone())
        .unwrap_or_default();
    let is_player = st.empire(eid).map(|e| e.is_player).unwrap_or(false);
    let mut minerals = 0.0f32;
    let mut credits = 0.0f32;
    let mut research = 0.0f32;

    let colonies: Vec<(usize, Colony)> = st
        .systems
        .iter()
        .filter_map(|s| s.planet.colony.as_ref().map(|c| (s.id, c.clone())))
        .filter(|(_, c)| c.empire_id == eid)
        .collect();

    for (sid, col) in colonies {
        if let Some(s) = st.system(sid) {
            let p = colony_production(s, &col);
            minerals += p.minerals;
            credits += p.credits;
            research += p.research;
        }
        let cap = st
            .system(sid)
            .and_then(|s| s.planet.colony.as_ref())
            .map(|c| max_pop(st.system(sid).unwrap(), st.empire(c.empire_id)))
            .unwrap_or(0);
        if let Some(s) = st.system_mut(sid) {
            if let Some(c) = s.planet.colony.as_mut() {
                c.population = (cap as f32).min(c.population * (1.0 + colony_growth_rate(c)));
            }
        }
    }

    if !is_player {
        let mul = match difficulty.as_str() {
            "hard" => 1.25,
            "easy" => 0.8,
            _ => 1.0,
        };
        minerals *= mul;
        credits *= mul;
        research *= mul;
    }

    let current_tech = st.empire(eid).and_then(|e| e.current_tech.clone());
    let mut completed: Option<String> = None;
    if let Some(e) = st.empire_mut(eid) {
        e.minerals += minerals;
        e.credits += credits;
        e.last_income = Income {
            minerals,
            credits,
            research,
        };
        e.research_pool += research;
        if let Some(ref t) = current_tech {
            e.tech_progress += e.research_pool;
            e.research_pool = 0.0;
            let cost = data::tech(t).map(|x| x.cost as f32).unwrap_or(0.0);
            if e.tech_progress >= cost {
                e.techs.push(t.clone());
                completed = Some(t.clone());
                e.current_tech = None;
                e.tech_progress = 0.0;
            }
        }
    }
    if let Some(t) = completed {
        let tname = data::tech(&t)
            .map(|x| x.name.to_string())
            .unwrap_or_default();
        let is_p = st.empire(eid).map(|e| e.is_player).unwrap_or(false);
        log(
            st,
            if is_p { "research" } else { "info" },
            format!(
                "{} completed research: {}.",
                st.empire(eid).map(|e| e.name.clone()).unwrap_or_default(),
                tname
            ),
        );
        let _ = is_p;
    }
}

fn progress_queues(st: &mut State, toasts: &mut Vec<String>) {
    let ids: Vec<usize> = st.systems.iter().map(|s| s.id).collect();
    for sid in ids {
        let (empire_id, key, build_turns, done) = match st.system(sid) {
            Some(s) => match &s.planet.colony {
                Some(c) if !c.queue.is_empty() => {
                    let key = c.queue[0].clone();
                    let turns = ship_def(st, &key).map(|d| d.build_turns).unwrap_or(1);
                    let done = c.queue_progress + 1 >= turns;
                    (c.empire_id, key, turns, done)
                }
                _ => continue,
            },
            None => continue,
        };
        if let Some(s) = st.system_mut(sid) {
            if let Some(c) = s.planet.colony.as_mut() {
                c.queue_progress += 1;
            }
        }
        if done {
            if let Some(s) = st.system_mut(sid) {
                if let Some(c) = s.planet.colony.as_mut() {
                    c.queue.remove(0);
                    c.queue_progress = 0;
                }
            }
            deliver_ship(st, sid, empire_id, &key, toasts);
        }
    }
}

fn deliver_ship(
    st: &mut State,
    sys_id: usize,
    empire_id: usize,
    key: &str,
    _toasts: &mut Vec<String>,
) {
    let existing = st
        .fleets
        .iter()
        .find(|f| f.empire_id == empire_id && f.system_id == sys_id && f.moves.is_none())
        .map(|f| f.id);
    let fleet_id = match existing {
        Some(id) => id,
        None => {
            let max_id = st.fleets.iter().map(|f| f.id).max().unwrap_or(0);
            let (x, y) = match st.system(sys_id) {
                Some(s) => (s.x, s.y),
                None => (0.0, 0.0),
            };
            st.fleets.push(Fleet {
                id: max_id + 1,
                empire_id,
                system_id: sys_id,
                x,
                y,
                ships: Vec::new(),
                moves: None,
            });
            max_id + 1
        }
    };
    let hp = st
        .empire(empire_id)
        .map(|e| effective_ship_stats(st, e, key).hp)
        .unwrap_or(1.0);
    if let Some(f) = st.fleet_mut(fleet_id) {
        f.ships.push(Ship {
            cls: key.to_string(),
            hp,
        });
    }
    let sname = ship_def(st, key).map(|d| d.name).unwrap_or_default();
    let sys_name = st
        .system(sys_id)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let is_player = st.empire(empire_id).map(|e| e.is_player).unwrap_or(false);
    log(
        st,
        if is_player { "build" } else { "info" },
        format!("{} completed at {}.", sname, sys_name),
    );
}

// ---------------------------------------------------------------------------
// Combat
// ---------------------------------------------------------------------------
#[derive(Clone)]
struct SimShip {
    cls: String,
    hp: f32,
    attack: f32,
    empire_id: usize,
    ref_fleet: Option<usize>,
}

#[derive(Clone)]
struct Team {
    empire_id: usize,
    team: String,
    is_platforms: bool,
    platform_system_id: Option<usize>,
    ships: Vec<SimShip>,
}

#[derive(Clone, Default)]
struct BattleResult {
    winner_team: Option<String>,
    losses: HashMap<usize, i32>,
    survivors: HashMap<usize, i32>,
    rounds: i32,
}

/// Round-based combat simulation.
fn battle_correct(st: &State, teams: &[Team], rng: &mut StdRng) -> BattleResult {
    let mut sim: Vec<Team> = teams
        .iter()
        .map(|t| Team {
            empire_id: t.empire_id,
            team: t.team.clone(),
            is_platforms: t.is_platforms,
            platform_system_id: t.platform_system_id,
            ships: t.ships.clone(),
        })
        .collect();

    let mut start_count: HashMap<usize, i32> = HashMap::new();
    for t in &sim {
        *start_count.entry(t.empire_id).or_insert(0) += t.ships.len() as i32;
    }

    let mut rounds = 0;
    loop {
        let alive_count = sim.iter().filter(|t| !t.ships.is_empty()).count();
        if alive_count <= 1 || rounds > 300 {
            break;
        }
        rounds += 1;

        // Each ship picks an enemy team and one of its ships.
        let mut hits: Vec<(usize, usize, f32)> = Vec::new();
        for (ti, t) in sim.iter().enumerate() {
            let enemy_teams: Vec<usize> = sim
                .iter()
                .enumerate()
                .filter(|(oi, o)| *oi != ti && o.team != t.team && !o.ships.is_empty())
                .map(|(oi, _)| oi)
                .collect();
            if enemy_teams.is_empty() {
                continue;
            }
            for s in &t.ships {
                let target_team = enemy_teams[rng.gen_range(0..enemy_teams.len())];
                let target_idx = rng.gen_range(0..sim[target_team].ships.len());
                hits.push((
                    target_team,
                    target_idx,
                    s.attack * rng.gen_range(0.75..1.25f32),
                ));
            }
        }
        for (ti, si, dmg) in hits {
            if let Some(s) = sim[ti].ships.get_mut(si) {
                s.hp -= dmg;
            }
        }
        for t in sim.iter_mut() {
            t.ships.retain(|s| s.hp > 0.0);
        }
    }

    let alive: Vec<&Team> = sim.iter().filter(|t| !t.ships.is_empty()).collect();
    let winner_team = if alive.len() == 1 {
        Some(alive[0].team.clone())
    } else {
        None
    };

    let mut survivors: HashMap<usize, i32> = HashMap::new();
    for t in &sim {
        *survivors.entry(t.empire_id).or_insert(0) += t.ships.len() as i32;
    }
    let mut losses: HashMap<usize, i32> = HashMap::new();
    for (eid, n) in &start_count {
        losses.insert(*eid, *n - survivors.get(eid).copied().unwrap_or(0));
    }

    BattleResult {
        winner_team,
        losses,
        survivors,
        rounds,
    }
}

fn ship_ref(st: &State, empire_id: usize, fleet_id: usize, index: usize) -> SimShip {
    let cls = st
        .fleet(fleet_id)
        .and_then(|f| f.ships.get(index))
        .map(|s| s.cls.clone())
        .unwrap_or_else(|| "frigate".into());
    let hp = st
        .fleet(fleet_id)
        .and_then(|f| f.ships.get(index))
        .map(|s| s.hp)
        .unwrap_or(0.0);
    let attack = st
        .empire(empire_id)
        .map(|e| effective_ship_stats(st, e, &cls).attack)
        .unwrap_or(0.0);
    SimShip {
        cls,
        hp,
        attack,
        empire_id,
        ref_fleet: Some(fleet_id),
    }
}

fn resolve_combat(st: &mut State, rng: &mut StdRng, toasts: &mut Vec<String>) {
    let sys_ids: Vec<usize> = st.systems.iter().map(|s| s.id).collect();
    for sid in sys_ids {
        let fleets: Vec<Fleet> = st
            .fleets
            .iter()
            .filter(|f| f.system_id == sid)
            .cloned()
            .collect();
        if fleets.is_empty() {
            continue;
        }
        let owner = st.system(sid).and_then(|s| s.owner_empire_id);
        let mut by_empire: Vec<(usize, Vec<Fleet>)> = Vec::new();
        for f in fleets {
            match by_empire.iter_mut().find(|(eid, _)| *eid == f.empire_id) {
                Some((_, lst)) => lst.push(f),
                None => by_empire.push((f.empire_id, vec![f])),
            }
        }
        let present: Vec<usize> = by_empire.iter().map(|(e, _)| *e).collect();
        let has_colony = st
            .system(sid)
            .and_then(|s| s.planet.colony.as_ref())
            .is_some();

        // Case 1: owned system with attackers present
        if let Some(owner_id) = owner {
            let owner_alive = st.empire(owner_id).map(|e| e.alive).unwrap_or(false);
            if owner_alive {
                let attackers: Vec<usize> = present
                    .iter()
                    .copied()
                    .filter(|e| *e != owner_id && at_war(st, *e, owner_id))
                    .collect();
                if !attackers.is_empty() {
                    if combat_at_owned(st, sid, owner_id, &attackers, &by_empire, rng, toasts) {
                        continue;
                    }
                }
            }
        }

        // Case 2: free-for-all among hostile fleets
        let conflict: Vec<usize> = present
            .iter()
            .copied()
            .filter(|e| present.iter().any(|o| *o != *e && at_war(st, *e, *o)))
            .collect();
        if conflict.len() >= 2 {
            let mut teams: Vec<Team> = Vec::new();
            for e in &conflict {
                let mut ships = Vec::new();
                for (eid, lst) in &by_empire {
                    if eid == e {
                        for f in lst {
                            for i in 0..f.ships.len() {
                                ships.push(ship_ref(st, *e, f.id, i));
                            }
                        }
                    }
                }
                teams.push(Team {
                    empire_id: *e,
                    team: format!("f{}", e),
                    is_platforms: false,
                    platform_system_id: None,
                    ships,
                });
            }
            let res = battle_correct(st, &teams, rng);
            let winner_name = res
                .winner_team
                .as_ref()
                .and_then(|t| teams.iter().find(|x| x.team == *t))
                .and_then(|t| st.empire(t.empire_id).map(|e| e.name.clone()));
            let names: Vec<String> = conflict
                .iter()
                .filter_map(|c| st.empire(*c).map(|e| e.name.clone()))
                .collect();
            let total_losses: i32 = res.losses.values().sum();
            let is_player = conflict.contains(&0);
            log(
                st,
                if is_player { "war" } else { "info" },
                format!(
                    "Space battle at {}: {} - {}.",
                    st.system(sid).map(|s| s.name.clone()).unwrap_or_default(),
                    names.join(" vs "),
                    if let Some(w) = &winner_name {
                        format!("{} wins", w)
                    } else {
                        "no survivor".to_string()
                    }
                ),
            );
            record_battle(
                st,
                BattleRec {
                    turn: st.turn,
                    system: st.system(sid).map(|s| s.name.clone()).unwrap_or_default(),
                    kind: "space".into(),
                    participants: names,
                    winner: winner_name,
                    losses: total_losses,
                },
            );
        }
    }
}

/// Returns true if the system was handled (combat or capture).
fn combat_at_owned(
    st: &mut State,
    sid: usize,
    owner_id: usize,
    attackers: &[usize],
    by_empire: &[(usize, Vec<Fleet>)],
    rng: &mut StdRng,
    toasts: &mut Vec<String>,
) -> bool {
    let owner_fleets: Vec<Fleet> = by_empire
        .iter()
        .find(|(e, _)| *e == owner_id)
        .map(|(_, l)| l.clone())
        .unwrap_or_default();

    let mut defenders: Vec<SimShip> = Vec::new();
    for f in &owner_fleets {
        for i in 0..f.ships.len() {
            defenders.push(ship_ref(st, owner_id, f.id, i));
        }
    }
    let has_shield = st
        .system(sid)
        .and_then(|s| s.planet.colony.as_ref())
        .map(|c| c.buildings.iter().any(|b| b == "shield"))
        .unwrap_or(false);

    if defenders.is_empty() && !has_shield {
        let mut strongest = attackers[0];
        let mut best = -1.0f32;
        for a in attackers {
            let s = empire_strength(st, *a);
            if s > best {
                best = s;
                strongest = *a;
            }
        }
        capture_colony(st, sid, strongest, rng, toasts);
        return true;
    }

    let mut teams: Vec<Team> = Vec::new();
    if !defenders.is_empty() {
        teams.push(Team {
            empire_id: owner_id,
            team: "def".into(),
            is_platforms: false,
            platform_system_id: None,
            ships: defenders,
        });
    }
    if has_shield {
        teams.push(Team {
            empire_id: owner_id,
            team: "def".into(),
            is_platforms: true,
            platform_system_id: Some(sid),
            ships: (0..4)
                .map(|_| SimShip {
                    cls: "__platform".into(),
                    hp: 30.0,
                    attack: 10.0,
                    empire_id: owner_id,
                    ref_fleet: None,
                })
                .collect(),
        });
    }
    for a in attackers {
        let mut ships = Vec::new();
        for f in by_empire
            .iter()
            .filter(|(e, _)| e == a)
            .flat_map(|(_, l)| l)
        {
            for i in 0..f.ships.len() {
                ships.push(ship_ref(st, *a, f.id, i));
            }
        }
        teams.push(Team {
            empire_id: *a,
            team: format!("atk{}", a),
            is_platforms: false,
            platform_system_id: None,
            ships,
        });
    }

    let res = battle_correct(st, &teams, rng);
    let attacker_won = res
        .winner_team
        .as_ref()
        .map(|t| t.starts_with("atk"))
        .unwrap_or(false);
    let winner_name = res
        .winner_team
        .as_ref()
        .and_then(|t| teams.iter().find(|x| x.team == *t))
        .and_then(|t| st.empire(t.empire_id).map(|e| e.name.clone()));
    let total_losses: i32 = res.losses.values().sum();

    // Apply losses back to the real fleets.
    for t in &teams {
        if t.is_platforms {
            continue;
        }
        let mut by_fleet: HashMap<usize, i32> = HashMap::new();
        for s in &t.ships {
            if let Some(fid) = s.ref_fleet {
                *by_fleet.entry(fid).or_insert(0) += 1;
            }
        }
        for (fid, count) in by_fleet {
            if let Some(f) = st.fleet_mut(fid) {
                f.ships.truncate(count as usize);
            }
        }
    }
    // Destroy the shield building if its platforms died
    for t in &teams {
        if t.is_platforms && t.ships.is_empty() {
            if let Some(pid) = t.platform_system_id {
                if let Some(s) = st.system_mut(pid) {
                    if let Some(c) = s.planet.colony.as_mut() {
                        c.buildings.retain(|b| b != "shield");
                    }
                }
            }
        }
    }

    let owner_name = st
        .empire(owner_id)
        .map(|e| e.name.clone())
        .unwrap_or_default();
    let attacker_names: Vec<String> = attackers
        .iter()
        .filter_map(|a| st.empire(*a).map(|e| e.name.clone()))
        .collect();
    let is_player = owner_id == 0 || attackers.contains(&0);
    log(
        st,
        if is_player { "war" } else { "info" },
        format!(
            "Battle at {}: {} vs {} - {}. Losses: {} ships.",
            st.system(sid).map(|s| s.name.clone()).unwrap_or_default(),
            owner_name,
            attacker_names.join(", "),
            if let Some(w) = &winner_name {
                format!("{} victorious", w)
            } else {
                "stalemate".to_string()
            },
            total_losses
        ),
    );
    record_battle(
        st,
        BattleRec {
            turn: st.turn,
            system: st.system(sid).map(|s| s.name.clone()).unwrap_or_default(),
            kind: "colony".into(),
            participants: {
                let mut v = vec![owner_name.clone()];
                v.extend(attacker_names.clone());
                v
            },
            winner: winner_name.clone(),
            losses: total_losses,
        },
    );

    if attacker_won {
        if st
            .system(sid)
            .and_then(|s| s.planet.colony.as_ref())
            .is_some()
        {
            let mut survivors: Vec<usize> = Vec::new();
            for t in teams
                .iter()
                .filter(|t| t.team.starts_with("atk") && !t.ships.is_empty())
            {
                survivors.push(t.empire_id);
            }
            if !survivors.is_empty() {
                let mut capturer = survivors[0];
                let mut best = -1.0f32;
                for s in &survivors {
                    let str_ = empire_strength(st, *s);
                    if str_ > best {
                        best = str_;
                        capturer = *s;
                    }
                }
                capture_colony(st, sid, capturer, rng, toasts);
            }
        }
    }
    true
}

fn capture_colony(
    st: &mut State,
    sid: usize,
    empire_id: usize,
    rng: &mut StdRng,
    toasts: &mut Vec<String>,
) {
    let col = match st.system(sid).and_then(|s| s.planet.colony.clone()) {
        Some(c) => c,
        None => return,
    };
    let prev_id = col.empire_id;
    let prev_name = st
        .empire(prev_id)
        .map(|e| e.name.clone())
        .unwrap_or_default();
    let next_name = st
        .empire(empire_id)
        .map(|e| e.name.clone())
        .unwrap_or_default();
    let next_player = st.empire(empire_id).map(|e| e.is_player).unwrap_or(false);
    let prev_player = st.empire(prev_id).map(|e| e.is_player).unwrap_or(false);
    let sys_name = st.system(sid).map(|s| s.name.clone()).unwrap_or_default();

    let mut buildings = col.buildings.clone();
    if !buildings.is_empty() {
        let idx = rng.gen_range(0..buildings.len());
        buildings.remove(idx);
    }
    if let Some(s) = st.system_mut(sid) {
        s.planet.colony = Some(Colony {
            empire_id,
            population: (col.population * 0.5).round().max(1.0),
            buildings,
            queue: Vec::new(),
            queue_progress: 0,
        });
        s.owner_empire_id = Some(empire_id);
        s.explored_by.push(empire_id);
    }
    let kind = if next_player {
        "good"
    } else if prev_player {
        "bad"
    } else {
        "war"
    };
    log(
        st,
        kind,
        format!("{} captured {} from {}.", next_name, sys_name, prev_name),
    );
    if next_player {
        toasts.push(format!("Captured {}!", sys_name));
    }
    if prev_player {
        toasts.push(format!("Lost {}!", sys_name));
    }
}

// ---------------------------------------------------------------------------
// Elimination and victory
// ---------------------------------------------------------------------------
fn update_alive(st: &mut State, toasts: &mut Vec<String>) {
    let ids: Vec<usize> = st
        .empires
        .iter()
        .filter(|e| e.alive)
        .map(|e| e.id)
        .collect();
    for eid in ids {
        let colonies = st
            .systems
            .iter()
            .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(eid))
            .count();
        let has_colony_ship = st.fleets.iter().filter(|f| f.empire_id == eid).any(|f| {
            f.ships
                .iter()
                .any(|s| ship_def(st, &s.cls).map(|d| d.colonize).unwrap_or(false))
        });
        let queued_colony = st.systems.iter().any(|s| {
            s.planet.colony.as_ref().map(|c| c.empire_id) == Some(eid)
                && s.planet
                    .colony
                    .as_ref()
                    .map(|c| c.queue.iter().any(|q| q == "colony"))
                    .unwrap_or(false)
        });
        if colonies == 0 && !has_colony_ship && !queued_colony {
            let name = st.empire(eid).map(|e| e.name.clone()).unwrap_or_default();
            let is_player = st.empire(eid).map(|e| e.is_player).unwrap_or(false);
            if let Some(e) = st.empire_mut(eid) {
                e.alive = false;
            }
            st.fleets.retain(|f| f.empire_id != eid);
            log(
                st,
                if is_player { "bad" } else { "war" },
                format!("{} has been eliminated.", name),
            );
            if is_player {
                toasts.push("Your empire has fallen.".into());
            }
        }
    }
}

fn check_victory(st: &mut State, toasts: &mut Vec<String>) {
    if st.over {
        return;
    }
    let alive: Vec<usize> = st
        .empires
        .iter()
        .filter(|e| e.alive)
        .map(|e| e.id)
        .collect();
    let p = st.player().clone();
    if !p.alive {
        st.over = true;
        st.winner = alive.iter().copied().find(|id| *id != 0);
        log(st, "bad", "Defeat. Your empire is no more.");
        return;
    }
    if alive.len() == 1 && alive[0] == 0 {
        st.over = true;
        st.winner = Some(0);
        log(st, "good", "Victory! The galaxy is yours.");
        return;
    }
    let colonizable = st.systems.iter().filter(|s| can_colonize(&p, s)).count();
    let mine = st
        .systems
        .iter()
        .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(0))
        .count();
    if st.turn > 20 && colonizable > 0 && (mine as f32) / (colonizable as f32) >= 0.55 {
        st.over = true;
        st.winner = Some(0);
        log(st, "good", "Galactic domination achieved!");
        let _ = toasts;
    }
}

// ---------------------------------------------------------------------------
// Save / load
// ---------------------------------------------------------------------------
#[derive(Serialize, Deserialize)]
struct SaveFile {
    turn: i32,
    #[serde(default)]
    difficulty: String,
    #[serde(default)]
    over: bool,
    #[serde(default)]
    winner: Option<usize>,
    systems: Vec<System>,
    empires: Vec<Empire>,
    #[serde(default)]
    fleets: Vec<Fleet>,
    #[serde(default)]
    battles: Vec<BattleRec>,
    #[serde(default)]
    designs: HashMap<String, Vec<ShipDesign>>,
    #[serde(default)]
    logs: Vec<LogEntry>,
}

pub fn save_game(st: &State) -> bool {
    let save = SaveFile {
        turn: st.turn,
        difficulty: st.difficulty.clone(),
        over: st.over,
        winner: st.winner,
        systems: st.systems.clone(),
        empires: st.empires.clone(),
        fleets: st.fleets.clone(),
        battles: st.battles.clone(),
        designs: st.designs.clone(),
        logs: st.logs.iter().rev().take(100).rev().cloned().collect(),
    };
    match serde_json::to_string(&save) {
        Ok(json) => match std::fs::write(save_path(), json) {
            Ok(_) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

pub fn has_save() -> bool {
    save_path().exists()
}

pub fn load_game(ctx: &mut Ctx) -> bool {
    let json = match std::fs::read_to_string(save_path()) {
        Ok(j) => j,
        Err(_) => return false,
    };
    let save: SaveFile = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut st = State {
        turn: save.turn,
        systems: save.systems,
        empires: save.empires,
        fleets: save.fleets,
        logs: save.logs,
        battles: save.battles,
        designs: save.designs,
        selected: None,
        over: save.over,
        winner: save.winner,
        difficulty: save.difficulty,
    };
    // Older saves may lack the anomaly field on a system; serde defaults handle it.
    for s in st.systems.iter_mut() {
        if s.explored_by.is_empty() && s.owner_empire_id.is_some() {
            s.explored_by.push(s.owner_empire_id.unwrap());
        }
    }
    *ctx.st = st;
    true
}

/// Convenience used by the UI.
pub fn home_system_id(st: &State) -> Option<usize> {
    st.player().home_system_id
}

/// Width/height of the galaxy, for the camera clamp.
pub const GALAXY_W: f32 = WIDTH;
pub const GALAXY_H: f32 = HEIGHT;
