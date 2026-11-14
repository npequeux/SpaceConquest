# Stellar Dominion

A turn-based 4X space strategy game that runs entirely in the browser. No build step, no
dependencies, no server-side code — plain HTML, CSS and JavaScript on a `<canvas>`.

**Explore** a procedurally generated galaxy, **expand** by founding colonies on alien worlds,
**exploit** minerals / credits / research, **exterminate** three rival empires.

---

## Run it

Either of these works:

```bash
# 1. just open the file
open index.html            # double-clicking it in Explorer works too

# 2. or serve it (recommended, avoids any browser file:// quirks)
python -m http.server 8000
# then visit http://localhost:8000
```

No install, no build. Everything is client-side.

---

## How to play

**Goal:** win by eliminating every rival empire, or by controlling **55% of colonizable
worlds after turn 20**. You lose if your empire is wiped out.

### Turn structure
Each turn your colonies produce **minerals**, **credits** and **research**. Research fills
the current technology — points earned while no project is selected are **banked** and flow
into the next one automatically. Minerals and credits are stockpiled and spent on buildings
and ships. Press **Enter** or click **End Turn** to resolve the turn (AI moves, fleets travel,
battles resolve, production arrives).

### Controls
| Action | How |
| --- | --- |
| Select a system / fleet | **Left-click** it |
| Move a fleet | Select it, then **click a destination system** |
| Deselect | **Right-click** or **Esc** |
| Pan the map | **Drag** |
| Zoom | **Mouse wheel** |
| Jump to a region | **Click the minimap** |
| End turn | **Enter** or the **End Turn** button |

### Fleet management
Fleets of the same empire at the same system merge automatically at the end of a turn. You
can also **Split Fleet** (halves the ships into a new fleet) and, when other friendly fleets
share a system, **Merge All Here** or merge a specific fleet into the selected one.

### Expansion
Build a **Colony Ship** at a colony with a **Shipyard**, then send it to an uncolonized
world. Some worlds (Toxic / Volcanic) need the *Xeno Colonization* tech; Gas Giants can
never be colonized. Population grows over time and boosts every resource.

### Buildings (limited slots, cost minerals)
`Mine` (+minerals) · `Market` (+credits) · `Research Lab` (+research) · `Habitat` (+growth)
· `Shipyard` (required for ships) · `Planetary Shield` (defense platforms in combat)

### Ships
`Scout` (fast, fragile) · `Colony Ship` (founds colonies) · `Frigate` · `Destroyer`
(needs Advanced Construction) · `Cruiser` (Capital Shipyard) · `Battleship` (Titan Foundries).
Fleet speed is set by its **slowest** ship.

### Research
15 technologies across 5 branches: Weapons, Defense, Propulsion, Industry, Expansion.
Each completed tech applies to every ship you own, including ones already built.

### Combat
Battles resolve automatically at the end of a turn when hostile fleets share a system.
Destroy the defenders and the colony is **captured** (its population and a random building
are lost). Planetary Shields fight as extra defense platforms. Winning fleets keep the system.

### Diplomacy
Empires start at peace. Rivals declare war when they think they are stronger; you can
**Declare War** or **Sue for Peace** from the Diplomacy panel. Losing empires may offer
you peace — accept or refuse.

### Exploration
The galaxy is hidden at first. Move fleets within sensor range of a system to reveal its
planet, owner and any fleets present. Scouts are fast and cheap — use them.

### Anomalies
Unexplored systems may hide an anomaly (derelict freighter, pirate cache, ancient data core...).
The first empire to **survey** a system with an actual fleet — colony sensors are not enough —
claims a one-time reward of credits, minerals or research.

### Battle reports
The **Battles** button opens a log of recent engagements: system, type (space battle or colony
assault), participants, winner and ship losses. The event log carries the same news inline.

---

## Project layout

```
index.html          shell + UI containers
css/style.css       theme
js/util.js          helpers, RNG, math
js/data.js          star classes, planet types, ships, buildings, tech tree, empires
js/galaxy.js        procedural galaxy generation
js/game.js          state, economy, movement, combat, diplomacy, save/load
js/ai.js            rival empire behaviour (research, build, expand, fight, diplomacy)
js/ui.js            canvas rendering, input, panels, modals
js/main.js          bootstrap
```

Saves are written to `localStorage` (key `stellar_dominion_save_v1`).

---

## Design notes & roadmap

The simulation is fully deterministic per action and runs headless-clean (verified for
80+ turns with no exceptions). Ideas for later versions: ship designing, ground invasions,
alliances and trade, piracy, more planet traits, a minimap, and sound.
