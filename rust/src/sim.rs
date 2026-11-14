//! Owns the simulation state, the RNG and the pending toast queue.

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::engine::{self, Ctx, NewGameOpts};
use crate::state::State;

pub struct Sim {
    pub st: Option<State>,
    pub rng: StdRng,
    pub toasts: Vec<String>,
}

impl Sim {
    pub fn new() -> Self {
        Sim {
            st: None,
            rng: StdRng::from_entropy(),
            toasts: Vec::new(),
        }
    }

    pub fn has_state(&self) -> bool {
        self.st.is_some()
    }

    pub fn state(&self) -> Option<&State> {
        self.st.as_ref()
    }

    /// Build a mutable context over the current state, if a game is running.
    pub fn ctx(&mut self) -> Option<Ctx<'_>> {
        let Sim { st, rng, toasts } = self;
        let st = st.as_mut()?;
        Some(Ctx { st, rng, toasts })
    }

    pub fn new_game(&mut self, opts: NewGameOpts) {
        if self.st.is_none() {
            self.st = Some(engine::empty_state());
        }
        if let Some(mut ctx) = self.ctx() {
            engine::new_game(&mut ctx, opts);
        }
    }

    pub fn end_turn(&mut self) {
        if let Some(mut ctx) = self.ctx() {
            engine::end_turn(&mut ctx);
        }
    }

    pub fn save(&mut self) {
        if let Some(st) = &self.st {
            if engine::save_game(st) {
                self.toasts.push("Game saved".into());
            } else {
                self.toasts.push("Save failed".into());
            }
        }
    }

    pub fn load(&mut self) -> bool {
        match self.ctx() {
            Some(mut ctx) => engine::load_game(&mut ctx),
            None => false,
        }
    }

    pub fn drain_toasts(&mut self) -> Vec<String> {
        std::mem::take(&mut self.toasts)
    }
}

impl Default for Sim {
    fn default() -> Self {
        Self::new()
    }
}
