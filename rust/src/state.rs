//! Serializable game state. These types are written to disk by `save_game`.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Income {
    #[serde(default)]
    pub minerals: f32,
    #[serde(default)]
    pub credits: f32,
    #[serde(default)]
    pub research: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Colony {
    #[serde(rename = "empireId")]
    pub empire_id: usize,
    pub population: f32,
    #[serde(default)]
    pub buildings: Vec<String>,
    #[serde(default)]
    pub queue: Vec<String>,
    #[serde(rename = "queueProgress", default)]
    pub queue_progress: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Planet {
    #[serde(rename = "type")]
    pub kind: String,
    pub size: i32,
    pub richness: f32,
    #[serde(rename = "maxPop")]
    pub max_pop: i32,
    #[serde(default)]
    pub colony: Option<Colony>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub name: String,
    pub desc: String,
    #[serde(default)]
    pub reward: AnomalyReward,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AnomalyReward {
    #[serde(default)]
    pub credits: f32,
    #[serde(default)]
    pub minerals: f32,
    #[serde(default)]
    pub research: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct System {
    pub id: usize,
    pub name: String,
    pub x: f32,
    pub y: f32,
    #[serde(rename = "starClass")]
    pub star_class: String,
    #[serde(rename = "starColor")]
    pub star_color: String,
    #[serde(rename = "starRadius")]
    pub star_radius: f32,
    pub planet: Planet,
    #[serde(rename = "ownerEmpireId", default)]
    pub owner_empire_id: Option<usize>,
    #[serde(rename = "exploredBy", default)]
    pub explored_by: Vec<usize>,
    #[serde(default)]
    pub anomaly: Option<Anomaly>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Empire {
    pub id: usize,
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub desc: String,
    #[serde(rename = "isPlayer", default)]
    pub is_player: bool,
    #[serde(default)]
    pub credits: f32,
    #[serde(default)]
    pub minerals: f32,
    #[serde(rename = "techProgress", default)]
    pub tech_progress: f32,
    #[serde(rename = "researchPool", default)]
    pub research_pool: f32,
    #[serde(default)]
    pub techs: Vec<String>,
    #[serde(rename = "currentTech", default)]
    pub current_tech: Option<String>,
    #[serde(rename = "homeSystemId", default)]
    pub home_system_id: Option<usize>,
    #[serde(default = "default_true")]
    pub alive: bool,
    #[serde(rename = "atWarWith", default)]
    pub at_war_with: Vec<usize>,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub aggression: f32,
    #[serde(rename = "lastIncome", default)]
    pub last_income: Income,
    #[serde(rename = "peaceOffers", default)]
    pub peace_offers: Vec<usize>,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ship {
    pub cls: String,
    #[serde(default)]
    pub hp: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Move {
    #[serde(rename = "toSystemId")]
    pub to_system_id: usize,
    #[serde(rename = "turnsLeft")]
    pub turns_left: i32,
    #[serde(rename = "totalTurns")]
    pub total_turns: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Fleet {
    pub id: usize,
    #[serde(rename = "empireId")]
    pub empire_id: usize,
    #[serde(rename = "systemId")]
    pub system_id: usize,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub ships: Vec<Ship>,
    #[serde(default)]
    pub moves: Option<Move>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LogEntry {
    #[serde(default)]
    pub turn: i32,
    #[serde(default)]
    pub msg: String,
    #[serde(rename = "type", default)]
    pub kind: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BattleRec {
    #[serde(default)]
    pub turn: i32,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default)]
    pub winner: Option<String>,
    #[serde(default)]
    pub losses: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Selection {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub id: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub turn: i32,
    pub systems: Vec<System>,
    pub empires: Vec<Empire>,
    #[serde(default)]
    pub fleets: Vec<Fleet>,
    #[serde(default)]
    pub logs: Vec<LogEntry>,
    #[serde(default)]
    pub battles: Vec<BattleRec>,
    #[serde(default)]
    pub designs: HashMap<String, Vec<crate::data::ShipDesign>>,
    #[serde(default)]
    pub selected: Option<Selection>,
    #[serde(default)]
    pub over: bool,
    /// empire id of the winner, if the game is over
    #[serde(default)]
    pub winner: Option<usize>,
    #[serde(default)]
    pub difficulty: String,
}

impl State {
    pub fn system(&self, id: usize) -> Option<&System> {
        self.systems.get(id)
    }

    pub fn system_mut(&mut self, id: usize) -> Option<&mut System> {
        self.systems.get_mut(id)
    }

    pub fn empire(&self, id: usize) -> Option<&Empire> {
        self.empires.get(id)
    }

    pub fn empire_mut(&mut self, id: usize) -> Option<&mut Empire> {
        self.empires.get_mut(id)
    }

    pub fn fleet(&self, id: usize) -> Option<&Fleet> {
        self.fleets.iter().find(|f| f.id == id)
    }

    pub fn fleet_mut(&mut self, id: usize) -> Option<&mut Fleet> {
        self.fleets.iter_mut().find(|f| f.id == id)
    }

    pub fn player(&self) -> &Empire {
        &self.empires[0]
    }

    pub fn player_mut(&mut self) -> &mut Empire {
        &mut self.empires[0]
    }

    /// systems owned by an empire
    pub fn empire_systems(&self, eid: usize) -> Vec<&System> {
        self.systems.iter().filter(|s| s.owner_empire_id == Some(eid)).collect()
    }

    pub fn empire_colonies(&self, eid: usize) -> Vec<&System> {
        self.systems
            .iter()
            .filter(|s| s.planet.colony.as_ref().map(|c| c.empire_id) == Some(eid))
            .collect()
    }

    pub fn empire_fleets(&self, eid: usize) -> Vec<&Fleet> {
        self.fleets.iter().filter(|f| f.empire_id == eid).collect()
    }

    pub fn fleets_at(&self, system_id: usize) -> Vec<&Fleet> {
        self.fleets.iter().filter(|f| f.system_id == system_id).collect()
    }

    pub fn is_explored(&self, system: &System, eid: usize) -> bool {
        system.explored_by.contains(&eid)
    }
}
