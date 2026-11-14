"""Render a few frames of the game offscreen (no window) for visual verification.

Usage:
    uv run python tools/render_test.py <outdir>
"""

import os
import sys

os.environ.setdefault("SDL_VIDEODRIVER", "dummy")
os.environ.setdefault("SDL_AUDIODRIVER", "dummy")

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(_HERE))

import pygame  # noqa: E402

from stellar_dominion import ai, engine  # noqa: E402
from stellar_dominion.ui import UI  # noqa: E402


def main():
    outdir = sys.argv[1] if len(sys.argv) > 1 else "."
    os.makedirs(outdir, exist_ok=True)

    ui = UI()
    engine.new_game({"numAI": 3, "numSystems": 46, "difficulty": "normal",
                     "playerName": "Terran Consortium"})
    ui.modal = None
    ui.focus_home()
    for _ in range(12):
        engine.end_turn(ai)

    # 1) main map view
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "01_map.png"))

    # 2) home system selected (bottom panel + build buttons)
    engine.select({"type": "system", "id": engine.player()["homeSystemId"]})
    engine.queue_ship(engine.get_system(engine.player()["homeSystemId"]), "frigate")
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "02_colony.png"))

    # 3) research modal
    engine.set_research(engine.player(), "weapons1")
    ui.modal = "research"
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "03_research.png"))
    ui.modal = None

    # 4) diplomacy modal
    ui.modal = "diplomacy"
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "04_diplomacy.png"))
    ui.modal = None

    # 5) battles modal (after a war)
    for _ in range(40):
        engine.end_turn(ai)
    ui.modal = "battles"
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "05_battles.png"))
    ui.modal = None

    # 6) help modal
    ui.modal = "help"
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "06_help.png"))
    ui.modal = None

    # 6b) ship designer modal
    ui.open_designer()
    ui.do_action("design_hull", "cruiser", None)
    ui.do_action("design_weapon", "w2", None)
    ui.do_action("design_armor", "a2", None)
    ui.do_action("design_engine", "e2", None)
    ui.d_name = "Aurelian Spear"
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "06b_designer.png"))
    ui.modal = None

    # 7) zoomed-out whole galaxy
    ui.cam = {"x": 1800, "y": 1200, "zoom": 0.33}
    ui.draw()
    pygame.image.save(ui.screen, os.path.join(outdir, "07_galaxy.png"))

    print("rendered 7 frames to", os.path.abspath(outdir))
    pygame.quit()


if __name__ == "__main__":
    main()
