//! Stellar Dominion — a turn-based 4X space strategy game.
//!
//! Modules:
//!   data    — static tables (stars, planets, ships, tech, empires)
//!   galaxy  — procedural galaxy generation
//!   state   — serializable game state
//!   engine  — rules, economy, movement, combat, diplomacy, save/load
//!   ai      — rival empire behaviour
//!   sim     — owns state + RNG + toast queue
//!   ui      — macroquad frontend (map, HUD, panels, modals)

use macroquad::prelude::*;
use stellar_dominion::{sim, ui};

fn window_conf() -> Conf {
    Conf {
        window_title: "Stellar Dominion".to_owned(),
        window_width: 1600,
        window_height: 900,
        // render at the monitor's native resolution: without this Windows
        // upscales the framebuffer and the whole interface comes out blurry
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut sim = sim::Sim::new();
    let mut ui = ui::Ui::new();

    loop {
        ui.frame(&mut sim);
        next_frame().await;
    }
}
