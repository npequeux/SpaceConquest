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

// A browser has no OS entropy source, and macroquad's wasm backend expects the
// host to provide the `env` imports. Fill getrandom's buffer from
// `crypto.getRandomValues`, which rust/web/index.html installs on the glue's
// import object before the wasm is instantiated.
#[cfg(target_arch = "wasm32")]
mod wasm_entropy {
    #[link(wasm_import_module = "env")]
    extern "C" {
        fn sd_fill_random(ptr: *mut u8, len: u32);
    }

    fn fill(buf: &mut [u8]) -> Result<(), getrandom::Error> {
        // crypto.getRandomValues rejects buffers longer than 65536 bytes.
        for chunk in buf.chunks_mut(65536) {
            unsafe { sd_fill_random(chunk.as_mut_ptr(), chunk.len() as u32) };
        }
        Ok(())
    }

    getrandom::register_custom_getrandom!(fill);
}

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
