//! AI empire behaviour. The AI is just another caller of the player's action
//! functions in `engine`.

use rand::Rng;

use crate::data;
use crate::engine as g;
use crate::state::{Empire, Fleet, State};

const RESEARCH_PRIORITY: &[&str] = &[
    "construction1",
    "weapons1",
    "engines1",
    "colonization1",
    "shields1",
    "construction2",
    "weapons2",
    "engines2",
    "colonization2",
    "shields2",
    "construction3",
    "weapons3",
    "engines3",
    "colonization3",
    "shields3",
];

pub fn take_turn(st: &mut State, eid: usize, rng: &mut impl Rng, toasts: &mut Vec<String>) {
    maybe_design(st, eid);
    choose_research(st, eid);
    manage_colonies(st, eid, rng, toasts);
    manage_fleets(st, eid, rng);
    diplomacy(st, eid, rng, toasts);
}

// ---------------------------------------------------------------------------
// Research
// ---------------------------------------------------------------------------
fn choose_research(st: &mut State, eid: usize) {
    if st
        .empire(eid)
        .and_then(|e| e.current_tech.clone())
        .is_some()
    {
        return;
    }
    for key in RESEARCH_PRIORITY {
        let tech = match data::tech(key) {
            Some(t) => t,
            None => continue,
        };
        let empire = match st.empire(eid) {
            Some(e) => e,
            None => return,
        };
        if empire.techs.iter().any(|t| t == key) {
            continue;
        }
        if !tech
            .requires
            .iter()
            .all(|r| empire.techs.iter().any(|t| t == r))
        {
            continue;
        }
        if let Some(e) = st.empire_mut(eid) {
            e.current_tech = Some(key.to_string());
            e.tech_progress = 0.0;
        }
        return;
    }
}

// ---------------------------------------------------------------------------
// Colonies
// ---------------------------------------------------------------------------
fn manage_colonies(st: &mut State, eid: usize, rng: &mut impl Rng, toasts: &mut Vec<String>) {
    let colonies: Vec<usize> = st
        .systems
        .iter()
        .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(eid))
        .map(|s| s.id)
        .collect();

    // 1) Ship production first — capital ships take several turns, so start early
    for sid in &colonies {
        let (has_shipyard, queue_len) = match st.system(*sid) {
            Some(s) => match &s.planet.colony {
                Some(c) => (c.buildings.iter().any(|b| b == "shipyard"), c.queue.len()),
                None => (false, 0),
            },
            None => (false, 0),
        };
        if has_shipyard && queue_len < 3 {
            if let Some(key) = choose_ship(st, eid, rng) {
                g::queue_ship(st, *sid, &key, toasts);
            }
        }
    }

    // 2) Buildings — a couple per turn at most, so minerals keep accumulating
    let mut builds = 0;
    for sid in &colonies {
        if builds >= 2 {
            break;
        }
        let (buildings, empire_minerals, slots) = match st.system(*sid) {
            Some(s) => match &s.planet.colony {
                Some(c) => (
                    c.buildings.clone(),
                    st.empire(eid).map(|e| e.minerals).unwrap_or(0.0),
                    g::slots(s),
                ),
                None => (Vec::new(), 0.0, 0),
            },
            None => (Vec::new(), 0.0, 0),
        };

        if !buildings.iter().any(|b| b == "shipyard") && (buildings.len() as i32) < slots {
            if g::build_building(st, *sid, "shipyard", toasts) {
                builds += 1;
                continue;
            }
        }

        for key in [
            "mine", "lab", "market", "mine", "lab", "market", "habitat", "shield",
        ] {
            if buildings.len() as i32 >= slots {
                break;
            }
            let b = match data::building(key) {
                Some(b) => b,
                None => continue,
            };
            if !b.repeat && buildings.iter().any(|x| x == key) {
                continue;
            }
            if empire_minerals >= b.cost as f32 + 40.0 {
                if g::build_building(st, *sid, key, toasts) {
                    builds += 1;
                }
                break;
            }
        }
    }
}

fn choose_ship(st: &mut State, eid: usize, rng: &mut impl Rng) -> Option<String> {
    let mut colony_ships = 0usize;
    let mut military = 0usize;
    for f in st.fleets.iter().filter(|f| f.empire_id == eid) {
        for s in &f.ships {
            if s.cls == "colony" {
                colony_ships += 1;
            }
            if g::ship_def(st, &s.cls)
                .map(|d| d.attack > 0.0)
                .unwrap_or(false)
            {
                military += 1;
            }
        }
    }
    for s in st.systems.iter() {
        if let Some(c) = &s.planet.colony {
            if c.empire_id == eid {
                colony_ships += c.queue.iter().filter(|q| **q == "colony").count();
            }
        }
    }

    let colonies = st
        .systems
        .iter()
        .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(eid))
        .count();
    let empire = match st.empire(eid) {
        Some(e) => e.clone(),
        None => return None,
    };
    let targets: Vec<usize> = st
        .systems
        .iter()
        .filter(|s| s.planet.colony.is_none() && g::can_colonize(&empire, s))
        .map(|s| s.id)
        .collect();

    // Expand only once the empire fields a credible fleet
    if !targets.is_empty() && colony_ships < 2 && military >= colonies && rng.gen::<f32>() < 0.7 {
        return Some("colony".to_string());
    }

    let mut scored: Vec<(f32, String, i32, i32)> = Vec::new();
    let mut candidates: Vec<String> = data::ship_classes()
        .iter()
        .map(|c| c.key.to_string())
        .collect();
    candidates.extend(g::list_designs(st, eid));
    for key in candidates {
        let ship = match g::ship_def(st, &key) {
            Some(s) => s,
            None => continue,
        };
        if let Some(t) = &ship.tech {
            if !empire.techs.iter().any(|x| x == t) {
                continue;
            }
        }
        let (credits, minerals) = g::ship_cost(st, &empire, &key);
        let score = ship.attack * 2.0 + ship.hp * 0.5 + ship.speed * 0.05;
        scored.push((score, key, credits, minerals));
    }
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let (_, top_key, top_cr, top_min) = &scored[0];
    if empire.credits >= *top_cr as f32 && empire.minerals >= *top_min as f32 {
        return Some(top_key.clone());
    }
    if military < (colonies * 2).min(8) {
        for (_, key, cr, min) in &scored {
            if empire.credits >= *cr as f32 && empire.minerals >= *min as f32 {
                return Some(key.clone());
            }
        }
    }
    None
}

fn best_module<'a>(modules: &'a [data::Module], empire: &Empire) -> &'a data::Module {
    let mut best = &modules[0];
    for m in modules {
        if let Some(t) = m.tech {
            if !empire.techs.iter().any(|x| x == t) {
                continue;
            }
        }
        best = m;
    }
    best
}

fn maybe_design(st: &mut State, eid: usize) {
    let designs = g::list_designs(st, eid);
    if st.turn < 8 {
        return;
    }
    let empire = match st.empire(eid) {
        Some(e) => e.clone(),
        None => return,
    };
    let mut hull = data::hulls()[0];
    for h in data::hulls() {
        if let Some(t) = h.tech {
            if !empire.techs.iter().any(|x| x == t) {
                continue;
            }
        }
        hull = *h;
    }
    let weapon = best_module(data::weapon_modules(), &empire);
    let armor = best_module(data::armor_modules(), &empire);
    let engine = best_module(data::engine_modules(), &empire);
    if armor.key == "a0" && engine.key == "e0" {
        return;
    }

    let candidate = data::compute_design("tmp", hull.key, weapon.key, armor.key, engine.key);
    let cand_score = candidate.attack * 2.0 + candidate.hp * 0.5 + candidate.speed * 0.05;

    if !designs.is_empty() {
        let mut weakest: Option<String> = None;
        let mut weakest_score = f32::INFINITY;
        for k in &designs {
            if let Some(d) = g::ship_def(st, k) {
                let s = d.attack * 2.0 + d.hp * 0.5 + d.speed * 0.05;
                if s < weakest_score {
                    weakest_score = s;
                    weakest = Some(k.clone());
                }
            }
        }
        if let Some(w) = weakest {
            if cand_score <= weakest_score * 1.1 {
                return;
            }
            if designs.len() >= 4 {
                g::remove_design(st, &w);
            }
        }
    }

    let first_word = empire.name.split_whitespace().next().unwrap_or("Ship");
    let hull_name = hull.name.replace(" Hull", "");
    let name = format!("{} {} Mk{}", first_word, hull_name, designs.len() + 1);
    g::add_design(st, eid, &name, hull.key, weapon.key, armor.key, engine.key);
}

// ---------------------------------------------------------------------------
// Fleets
// ---------------------------------------------------------------------------
fn manage_fleets(st: &mut State, eid: usize, rng: &mut impl Rng) {
    let home_id = st.empire(eid).and_then(|e| e.home_system_id);
    let idle: Vec<usize> = st
        .fleets
        .iter()
        .filter(|f| f.empire_id == eid && f.moves.is_none())
        .map(|f| f.id)
        .collect();

    // 1) Colony ships expand
    for fid in &idle {
        let has_colony = st
            .fleet(*fid)
            .map(|f| {
                f.ships
                    .iter()
                    .any(|s| g::ship_def(st, &s.cls).map(|d| d.colonize).unwrap_or(false))
            })
            .unwrap_or(false);
        if has_colony {
            if let Some(target) = best_colonization_target(st, *fid, eid) {
                g::issue_move(st, *fid, target);
            }
        }
    }

    let military: Vec<usize> = idle
        .iter()
        .copied()
        .filter(|fid| {
            st.fleet(*fid)
                .map(|f| {
                    f.ships.iter().any(|s| {
                        g::ship_def(st, &s.cls)
                            .map(|d| d.attack > 0.0)
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
        .collect();

    let enemies: Vec<usize> = st
        .empires
        .iter()
        .filter(|e| e.alive && e.id != eid && g::at_war(st, eid, e.id))
        .map(|e| e.id)
        .collect();

    // 2) Defense — repel fleets intruding in our systems
    let my_colonies: Vec<usize> = st
        .systems
        .iter()
        .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(eid))
        .map(|s| s.id)
        .collect();
    for sid in my_colonies {
        let intruders: Vec<usize> = st
            .fleets
            .iter()
            .filter(|f| f.system_id == sid && f.empire_id != eid && g::at_war(st, eid, f.empire_id))
            .map(|f| f.id)
            .collect();
        if intruders.is_empty() {
            continue;
        }
        let def_str: f32 = intruders
            .iter()
            .map(|f| g::fleet_strength(st, st.fleet(*f).unwrap()))
            .sum();
        let mut candidates: Vec<usize> = military.clone();
        candidates.sort_by(|a, b| {
            let da = {
                let sa = st.system(st.fleet(*a).unwrap().system_id).unwrap();
                let sy = st.system(sid).unwrap();
                g::dist(sa.x, sa.y, sy.x, sy.y)
            };
            let db = {
                let sb = st.system(st.fleet(*b).unwrap().system_id).unwrap();
                let sy = st.system(sid).unwrap();
                g::dist(sb.x, sb.y, sy.x, sy.y)
            };
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut acc = 0.0f32;
        for f in candidates {
            if acc >= def_str * 1.2 {
                break;
            }
            g::issue_move(st, f, sid);
            acc += g::fleet_strength(st, st.fleet(f).unwrap());
        }
    }

    // 3) Keep a garrison at home while at war
    if !enemies.is_empty() && military.len() >= 2 {
        if let Some(home_id) = home_id {
            let mut weakest: Option<usize> = None;
            let mut weakest_str = f32::INFINITY;
            for f in &military {
                let strn = g::fleet_strength(st, st.fleet(*f).unwrap());
                if strn < weakest_str {
                    weakest_str = strn;
                    weakest = Some(*f);
                }
            }
            if let Some(w) = weakest {
                if st.fleet(w).map(|f| f.system_id) != Some(home_id) {
                    g::issue_move(st, w, home_id);
                }
            }
        }
    }

    // 4) Offense — only attack with a credible force
    let mut enemy_systems: Vec<usize> = Vec::new();
    for e in &enemies {
        enemy_systems.extend(
            st.systems
                .iter()
                .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(*e))
                .map(|s| s.id),
        );
    }

    if !enemy_systems.is_empty() {
        for f in &military {
            if st.fleet(*f).and_then(|x| x.moves.as_ref()).is_some() {
                continue;
            }
            let strength = g::fleet_strength(st, st.fleet(*f).unwrap());
            let cur = st.fleet(*f).unwrap().system_id;
            if strength < 70.0 {
                if let Some(home_id) = home_id {
                    if cur != home_id {
                        g::issue_move(st, *f, home_id);
                    }
                }
                continue;
            }
            let mut best: Option<usize> = None;
            let mut best_d = f32::INFINITY;
            for sid in &enemy_systems {
                let a = st.system(cur).unwrap();
                let b = st.system(*sid).unwrap();
                let d = g::dist(a.x, a.y, b.x, b.y);
                if d < best_d {
                    best_d = d;
                    best = Some(*sid);
                }
            }
            let best = match best {
                Some(b) => b,
                None => continue,
            };
            let def_str: f32 = st
                .fleets
                .iter()
                .filter(|x| x.system_id == best && g::at_war(st, eid, x.empire_id))
                .map(|x| g::fleet_strength(st, x))
                .sum();
            if def_str > 0.0 && strength < def_str * 1.1 {
                if let Some(home_id) = home_id {
                    if cur != home_id && rng.gen::<f32>() < 0.6 {
                        g::issue_move(st, *f, home_id);
                    }
                }
                continue;
            }
            g::issue_move(st, *f, best);
        }
    } else if let Some(home_id) = home_id {
        for f in &military {
            if st.fleet(*f).and_then(|x| x.moves.as_ref()).is_none()
                && st.fleet(*f).map(|x| x.system_id) != Some(home_id)
                && rng.gen::<f32>() < 0.25
            {
                g::issue_move(st, *f, home_id);
            }
        }
    }
}

fn best_colonization_target(st: &State, fleet_id: usize, eid: usize) -> Option<usize> {
    let from = st.fleet(fleet_id)?.system_id;
    let (fx, fy) = {
        let s = st.system(from)?;
        (s.x, s.y)
    };
    let empire = st.empire(eid)?;
    let mut best: Option<usize> = None;
    let mut best_score = f32::NEG_INFINITY;
    for s in &st.systems {
        if s.planet.colony.is_some() || !g::can_colonize(empire, s) {
            continue;
        }
        let d = g::dist(fx, fy, s.x, s.y);
        let hab = data::planet_type(&s.planet.kind)
            .map(|p| p.habitability)
            .unwrap_or(0.0);
        let score = hab * 100.0 - d * 0.05 + s.planet.richness * 5.0;
        if score > best_score {
            best_score = score;
            best = Some(s.id);
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Diplomacy
// ---------------------------------------------------------------------------
fn diplomacy(st: &mut State, eid: usize, rng: &mut impl Rng, toasts: &mut Vec<String>) {
    let others: Vec<usize> = st
        .empires
        .iter()
        .filter(|o| o.id != eid && o.alive)
        .map(|o| o.id)
        .collect();
    for other in others {
        let at_war = st
            .empire(eid)
            .map(|e| e.at_war_with.contains(&other))
            .unwrap_or(false);
        let my_str = g::empire_strength(st, eid);
        let their_str = g::empire_strength(st, other);
        let other_is_player = st.empire(other).map(|e| e.is_player).unwrap_or(false);

        if !at_war {
            // Avoid dogpiling: only one rival may be at war with the player at a time.
            if other_is_player {
                let already = st
                    .empires
                    .iter()
                    .any(|e| e.alive && !e.is_player && e.id != eid && g::at_war(st, e.id, other));
                if already {
                    continue;
                }
            }
            let aggression = st.empire(eid).map(|e| e.aggression).unwrap_or(0.5);
            if st.turn > 14 && my_str > their_str * 1.3 && rng.gen::<f32>() < aggression * 0.1 {
                g::declare_war(st, eid, other, toasts);
            }
        } else if my_str < their_str * 0.35 && rng.gen::<f32>() < 0.18 {
            if other_is_player {
                g::offer_peace(st, eid, other);
            } else {
                g::make_peace(st, eid, other);
            }
        }
    }
}
