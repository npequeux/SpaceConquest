//! Headless smoke tests for the simulation: no window, no input, just rules.

use stellar_dominion::engine::NewGameOpts;
use stellar_dominion::sim::Sim;
use stellar_dominion::state::State;

fn started() -> Sim {
    let mut sim = Sim::new();
    assert!(!sim.has_state(), "a fresh Sim has no game");
    sim.new_game(NewGameOpts {
        num_ai: 3,
        num_systems: 40,
        difficulty: "normal".into(),
        player_name: "Test Player".into(),
    });
    sim
}

#[test]
fn new_game_builds_a_playable_galaxy() {
    let sim = started();
    let st = sim.state().expect("new_game must create a state");
    assert_eq!(st.turn, 1);
    assert_eq!(st.empires.len(), 4, "player + 3 rivals");
    assert!(st.systems.len() >= 40);
    assert_eq!(st.fleets.len(), 4, "one starting fleet per empire");
    assert!(st.empires.iter().filter(|e| e.alive).count() == 4);
    assert!(st.player().home_system_id.is_some());
    assert!(!st.logs.is_empty(), "the opening turn logs a briefing");
}

#[test]
fn forty_turns_run_without_panicking() {
    let mut sim = started();
    for _ in 0..40 {
        sim.end_turn();
    }
    let st = sim.state().unwrap();
    assert_eq!(st.turn, 41);
    assert!(
        st.empires.iter().any(|e| !e.techs.is_empty()),
        "the AI should have researched something by turn 41"
    );
    let player = st.player();
    assert!(player.credits.is_finite() && player.minerals.is_finite());
    assert!(player.last_income.minerals.is_finite());
}

#[test]
fn state_survives_a_save_load_roundtrip() {
    let sim = started();
    let st = sim.state().unwrap();
    let json = serde_json::to_string(st).expect("state serializes");
    let back: State = serde_json::from_str(&json).expect("state deserializes");
    assert_eq!(back.turn, st.turn);
    assert_eq!(back.systems.len(), st.systems.len());
    assert_eq!(back.empires.len(), st.empires.len());
    assert_eq!(back.fleets.len(), st.fleets.len());
    assert_eq!(back.player().name, st.player().name);
}
