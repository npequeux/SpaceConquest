//! Procedural galaxy generation.

use rand::Rng;

use crate::data;
use crate::state::{Planet, System};

pub const WIDTH: f32 = 3600.0;
pub const HEIGHT: f32 = 2400.0;

fn make_planet(rng: &mut impl Rng) -> Planet {
    let r: f32 = rng.gen();
    let kind = if r < 0.10 {
        "terran"
    } else if r < 0.22 {
        "ocean"
    } else if r < 0.36 {
        "desert"
    } else if r < 0.50 {
        "ice"
    } else if r < 0.64 {
        "barren"
    } else if r < 0.74 {
        "toxic"
    } else if r < 0.84 {
        "volcanic"
    } else {
        "gas"
    };

    let size_roll: f32 = rng.gen();
    let size = if size_roll < 0.33 {
        1
    } else if size_roll < 0.70 {
        2
    } else {
        3
    };

    let richness = rng.gen_range(0.5..2.0f32);
    let habitability = data::planet_type(kind)
        .map(|p| p.habitability)
        .unwrap_or(0.0);
    let max_pop = (size as f32 * habitability * 12.0).round() as i32;

    Planet {
        kind: kind.to_string(),
        size,
        richness,
        max_pop,
        colony: None,
    }
}

fn make_system(sid: usize, x: f32, y: f32, used: &mut Vec<String>, rng: &mut impl Rng) -> System {
    let star = data::star_classes()[rng.gen_range(0..data::star_classes().len())];
    System {
        id: sid,
        name: data::system_name(used, rng),
        x,
        y,
        star_class: star.key.to_string(),
        star_color: star.color.to_string(),
        star_radius: star.radius,
        planet: make_planet(rng),
        owner_empire_id: None,
        explored_by: Vec::new(),
        anomaly: None,
    }
}

fn pick_homes(systems: &[System], n: usize, rng: &mut impl Rng) -> Vec<usize> {
    let mut homes = vec![rng.gen_range(0..systems.len())];
    while homes.len() < n && homes.len() < systems.len() {
        let mut best = 0usize;
        let mut best_d = -1.0f32;
        for (i, s) in systems.iter().enumerate() {
            if homes.contains(&i) {
                continue;
            }
            let d = homes
                .iter()
                .map(|&h| {
                    let o = &systems[h];
                    ((s.x - o.x).powi(2) + (s.y - o.y).powi(2)).sqrt()
                })
                .fold(f32::INFINITY, f32::min);
            if d > best_d {
                best_d = d;
                best = i;
            }
        }
        homes.push(best);
    }
    homes
}

/// Generate a galaxy. Returns `(systems, home_system_ids)`.
pub fn generate(
    num_systems: usize,
    num_empires: usize,
    rng: &mut impl Rng,
) -> (Vec<System>, Vec<usize>) {
    let mut systems: Vec<System> = Vec::new();
    let mut used: Vec<String> = Vec::new();
    let min_dist = 340.0f32;
    let max_attempts = num_systems * 400;
    let mut attempts = 0usize;

    while systems.len() < num_systems && attempts < max_attempts {
        attempts += 1;
        let x = rng.gen_range(160.0..WIDTH - 160.0);
        let y = rng.gen_range(160.0..HEIGHT - 160.0);
        let ok = systems
            .iter()
            .all(|s| ((x - s.x).powi(2) + (y - s.y).powi(2)).sqrt() >= min_dist);
        if ok {
            let id = systems.len();
            systems.push(make_system(id, x, y, &mut used, rng));
        }
    }

    let homes = pick_homes(&systems, num_empires, rng);
    (systems, homes)
}

pub fn make_home_planet() -> Planet {
    Planet {
        kind: "terran".to_string(),
        size: 3,
        richness: 1.5,
        max_pop: 36,
        colony: None,
    }
}
