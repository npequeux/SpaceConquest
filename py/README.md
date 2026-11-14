# Stellar Dominion (Python edition)

A turn-based 4X space strategy game written in **Python** with **pygame-ce**.
No JavaScript, no Java — pure Python.

> This is the Python rewrite of the original browser version. It is a desktop
> application because a browser can only execute JavaScript or WebAssembly, and
> JavaScript is off-limits for this project.

---

## Requirements

- Python 3.12 or newer
- [pygame-ce](https://pyga.me/) (the community edition of pygame)

## Run it

The easiest way is [uv](https://docs.astral.sh/uv/), which is already set up in
this folder:

```bash
cd py
uv run python main.py
```

Or with an ordinary Python environment:

```bash
cd py
python -m pip install pygame-ce
python main.py
```

The project virtual environment (`.venv`) is committed in the folder, so
`uv run` works out of the box.

---

## How to play

**Goal:** win by eliminating every rival empire, or by controlling **55% of
colonizable worlds after turn 20**. You lose if your empire is wiped out.

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
| Save | **Ctrl+S** or the **Save** button |

### The game
- **Explore** — systems start hidden; move fleets within sensor range to reveal them.
- **Expand** — build Colony Ships at colonies with a Shipyard and settle new worlds.
- **Exploit** — Mines, Markets, Labs, Habitats, Shipyards and Planetary Shields;
  population grows and boosts every resource.
- **Research** — 15 technologies across Weapons, Defense, Propulsion, Industry and
  Expansion. Unspent research is banked for your next project.
- **Exterminate** — 6 ship classes, auto-resolved battles, colony capture, and
  three AI rivals that expand, defend, declare war and sue for peace.
- **Design** — combine a hull with weapon, armour and engine modules to create your own
  ship classes. Rival empires design and field theirs too.
- **Anomalies** — derelict freighters, pirate caches and ancient data cores are
  claimed only by an actual fleet survey, so scouts matter.
- **Diplomacy** — declare war, sue for peace, or accept a rival's offer.

### Ship designer
Select one of your colonies with a Shipyard and click **+ Design a Ship**. Pick a hull
(Frigate / Destroyer / Cruiser / Battleship) and a weapon, armour and engine module; the
preview shows hit points, attack, speed, cost, build time and any research the design
needs. Saved designs appear in the colony's build list beside the stock classes.

---

## Project layout

```
py/
  stellar_dominion/
    data.py      static tables: stars, planets, ships, buildings, techs, designer, empires
    galaxy.py    procedural galaxy generation
    engine.py    state, rules, economy, movement, combat, diplomacy, save/load
    ai.py        rival empire behaviour (reuses the player's action functions)
    ui.py        pygame frontend: map, panels, minimap, modals, input
  main.py        entry point
  tools/
    render_test.py     headless frame rendering (no window needed)
    interact_test.py   headless UI interaction test
```

The simulation in `engine.py` is pure: it never imports pygame and never touches
the filesystem except through `save_game()` / `load_game()`. Saves live in
`~/.stellar_dominion_save.json`.

## Development

```bash
uv run python tools/render_test.py out/     # renders 7 frames offscreen
uv run python tools/interact_test.py        # 38 UI interaction checks
```

Both run with SDL's dummy video driver, so they need no display.
