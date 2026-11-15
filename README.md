# Stellar Dominion

A turn-based 4X space strategy game written in **Rust** with **macroquad**.

No JavaScript, no Java — a native desktop application.

---

## Rust edition

### Requirements

- Rust stable (2021 edition). On Windows the GNU toolchain
  (`stable-x86_64-pc-windows-gnu`) plus a MinGW-w64 `x86_64-w64-mingw32-gcc`
  linker is used, configured in `rust/.cargo/config.toml`. On Linux and macOS the
  default host toolchain works unchanged.

### Run it

```bash
cd rust
cargo run
```

The macroquad frontend is behind the default `ui` feature. The simulation itself
has no windowing dependency, so it builds and tests headlessly:

```bash
cargo test --no-default-features
```

The window opens at 1600x900 and is resizable. The interface scales itself to the
window (reference design 1024x640) and renders at the monitor's native resolution
(`high_dpi`), so text stays sharp on high-DPI displays instead of being upscaled
by the window system.

Everything is drawn in *logical* pixels: macroquad's default projection maps
`screen_size / dpi`, and `macroquad::window::screen_width()` returns the same
logical width. Font sizes passed to `draw_text` are logical too, so a design
size multiplied by the UI scale is what you get on screen. Do not divide by
`screen_dpi_scale()` in that path — it shrinks the text by the DPI factor.

The interface uses the bundled Inter face (`rust/assets/Inter.ttf`, SIL Open
Font License 1.1) instead of macroquad's ProggyClean pixel font, which turns to
mush above ~14px. It is rasterised with nearest-neighbour filtering and every
label gets a 1px shadow so it stays readable over the starfield.

### Project layout

```
rust/
  src/
    data.rs      static tables: stars, planets, ships, buildings, techs, designer, empires
    galaxy.rs    procedural galaxy generation
    state.rs     serializable game state
    engine.rs    rules, economy, movement, combat, diplomacy, save/load
    ai.rs        rival empire behaviour
    sim.rs       owns the state, the RNG and the toast queue
    ui.rs        macroquad frontend: map, panels, minimap, modals, input
    lib.rs       the simulation library (ui is feature-gated)
    main.rs      thin macroquad shell
  tests/
    sim_smoke.rs headless simulation tests (new game, 40 turns, save/load)
```

The simulation never touches the windowing layer; rendering and input live only
in `ui.rs`. Saves live in `~/.stellar_dominion_save.json`.

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

## Development

Rust:

```bash
cd rust
cargo test --no-default-features              # headless simulation tests
cargo clippy --no-default-features --all-targets -- -D warnings
cargo clippy --all-targets -- -D warnings     # needs the macroquad toolchain
cargo build --release                         # Windows executable
```

CI (`.github/workflows/ci.yml`) runs on every push and pull request:

| Job | Runner | What it does |
| --- | --- | --- |
| `rust` | ubuntu | clippy (headless), build, test |
| `ui` | windows | clippy (macroquad frontend), build |
| `packages` | windows + ubuntu + macOS x2 | `cargo build --release --target …`, then `install/package.sh` |
| `publish` | ubuntu | attaches the packages to GitHub releases |
| `pages` | ubuntu | `cargo build --release --target wasm32-unknown-unknown`, deploy to Pages |

Every push to `main` also moves the prerelease GitHub release
[`latest`](https://github.com/npequeux/SpaceConquest/releases/tag/latest) to the
new build, and redeploys the browser version to
<https://npequeux.github.io/SpaceConquest/>. Pushing a tag
(`git tag v3.0.0 && git push origin v3.0.0`) creates a separate, non-moving
release for that version.

### Install packages

`install/package.sh` builds one package set per platform; CI runs it on all of
them and publishes the result.

| Platform | Artefacts |
| --- | --- |
| Windows x64 | `.msi` installer (WiX), portable `.zip`, raw `.exe` |
| Linux x64 | `.tar.gz`, Debian/Ubuntu `.deb` |
| macOS arm64 / x64 | `.zip` with a `StellarDominion.app` bundle, `.dmg` |

```bash
cd rust
cargo build --release --target x86_64-unknown-linux-gnu
./install/package.sh linux-x64 x86_64-unknown-linux-gnu linux
```

Linux needs `libx11-dev libxcursor-dev libxrandr-dev libxinerama-dev libxi-dev
libgl1-mesa-dev libasound2-dev` at build time and X11 + ALSA at runtime. The
Windows MSI is only produced when a WiX v3 toolchain is on `PATH`
(`WIX_DIR` overrides the lookup).

