//! Stellar Dominion library: the simulation and rules, independent of the
//! windowing layer. `main.rs` is a thin macroquad shell over these modules.

pub mod ai;
pub mod data;
pub mod engine;
pub mod galaxy;
pub mod sim;
pub mod state;
#[cfg(feature = "ui")]
pub mod ui;
