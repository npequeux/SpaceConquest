"""Headless interaction test: drive the pygame UI without a window.

Usage:
    uv run python tools/interact_test.py
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
    class _Ev:
        def __init__(self, **kw):
            self.__dict__.update(kw)

    ui = UI()
    engine.new_game({"numAI": 3, "numSystems": 46, "difficulty": "normal",
                     "playerName": "Terran Consortium", "seed": 42})
    ui.modal = None
    ui.focus_home()

    ok = []

    # --- 1. click the home system on the map -> selects it
    ui.draw()
    home = engine.get_system(engine.player()["homeSystemId"])
    p = ui.w2screen(home["x"], home["y"])
    ui.mouse = (int(p[0]), int(p[1]))
    ui.handle_click(ui.mouse)
    ok.append(("select home system", engine.state["selected"] == {"type": "system", "id": home["id"]}))

    # --- 2. click a build button (Mine)
    ui.draw()
    before_min = engine.player()["minerals"]
    target = next((c for c in ui.clickables if c[1] == "build_building" and c[2] == "mine"), None)
    ok.append(("mine button present", target is not None))
    if target:
        ui.do_action("build_building", "mine", target[0].center)
        col = home["planet"]["colony"]
        ok.append(("mine built", "mine" in col["buildings"] and engine.player()["minerals"] == before_min - 60))

    # --- 3. queue a ship (frigate)
    ui.draw()
    target = next((c for c in ui.clickables if c[1] == "queue_ship" and c[2] == "frigate"), None)
    ok.append(("frigate button present", target is not None))
    if target:
        ui.do_action("queue_ship", "frigate", target[0].center)
        ok.append(("frigate queued", home["planet"]["colony"]["queue"] == ["frigate"]))

    # --- 4. select the fleet, then click a destination system -> move order
    ui.draw()
    fleet = engine.empire_fleets(0)[0]
    fp = ui.fleet_draw_pos(fleet)
    fs = ui.w2screen(fp[0], fp[1])
    ui.mouse = (int(fs[0]), int(fs[1]))
    ui.handle_click(ui.mouse)
    ok.append(("select fleet", engine.state["selected"] == {"type": "fleet", "id": fleet["id"]}))

    ui.draw()
    candidates = [s for s in engine.state["systems"]
                  if s["id"] != home["id"] and engine.can_colonize(engine.player(), s)]
    dest = min(candidates, key=lambda s: engine.dist(home["x"], home["y"], s["x"], s["y"]))
    ui.cam = {"x": (home["x"] + dest["x"]) / 2, "y": (home["y"] + dest["y"]) / 2, "zoom": 0.5}
    ui.draw()
    dp = ui.w2screen(dest["x"], dest["y"])
    ui.mouse = (int(dp[0]), int(dp[1]))
    ui.handle_click(ui.mouse)
    ok.append(("fleet ordered to move", bool(fleet["moves"]) and fleet["moves"]["toSystemId"] == dest["id"]))

    # --- 5. split and merge
    ui.do_action("split_fleet", None, None)
    n_fleets = len(engine.empire_fleets(0))
    ok.append(("split creates a second fleet", n_fleets == 2))
    for f in engine.empire_fleets(0):
        engine.stop_fleet(f)
    engine.select({"type": "fleet", "id": engine.empire_fleets(0)[0]["id"]})
    ui.do_action("merge_all", engine.empire_fleets(0)[0]["id"], None)
    ok.append(("merge-all back to one", len(engine.empire_fleets(0)) == 1))

    # --- 6. End Turn button
    turn_before = engine.state["turn"]
    ui.draw()
    end_btn = next((c for c in ui.clickables if c[1] == "end_turn"), None)
    ok.append(("end turn button present", end_btn is not None))
    if end_btn:
        ui.do_action("end_turn", None, end_btn[0].center)
        ok.append(("turn advanced", engine.state["turn"] == turn_before + 1))

    # --- 7. minimap jump
    cam_before = dict(ui.cam)
    ui.draw()
    mm = next((c for c in ui.clickables if c[1] == "minimap"), None)
    ok.append(("minimap clickable present", mm is not None))
    if mm:
        ui.do_action("minimap", None, (mm[0].x + 20, mm[0].y + 20))
        ok.append(("minimap jumps camera", ui.cam["x"] != cam_before["x"] or ui.cam["y"] != cam_before["y"]))

    # --- 8. modals open and have clickables
    for name in ["research", "diplomacy", "battles", "help", "new"]:
        ui.modal = name
        ui.draw()
        has = len(ui.clickables) > 0
        ok.append(("modal %s opens" % name, has))
    ui.modal = None

    # --- 9. research modal button sets a tech
    engine.set_research(engine.player(), "engines1")
    engine.player()["currentTech"] = None
    ui.modal = "research"
    ui.draw()
    btn = next((c for c in ui.clickables if c[1] == "set_tech" and c[2] == "engines1"), None)
    ok.append(("research button present", btn is not None))
    if btn:
        ui.do_action("set_tech", "engines1", btn[0].center)
        ok.append(("tech selected", engine.player()["currentTech"] == "engines1"))
    ui.modal = None

    # --- 10. diplomacy declare-war button
    ui.modal = "diplomacy"
    ui.draw()
    war_btn = next((c for c in ui.clickables if c[1] == "declare_war"), None)
    ok.append(("declare war button present", war_btn is not None))
    if war_btn:
        ui.do_action("declare_war", war_btn[2], war_btn[0].center)
        ok.append(("war declared", len(engine.player()["atWarWith"]) == 1))
    ui.modal = None

    # --- 11. save / load
    engine.save_game()
    snapshot = (engine.state["turn"], len(engine.state["fleets"]),
                round(engine.player()["credits"]), len(engine.state["battles"]))
    ok.append(("save works", engine.has_save()))
    ok.append(("load works", engine.load_game()))
    ok.append(("state round-trips",
               (engine.state["turn"], len(engine.state["fleets"]),
                round(engine.player()["credits"]), len(engine.state["battles"])) == snapshot))

    # --- 12. ship designer
    from stellar_dominion import data
    home = engine.get_system(engine.player()["homeSystemId"])
    p = engine.player()
    p["credits"] = 5000
    p["minerals"] = 5000
    for t in data.TECHS:
        p["techs"][t] = True
    engine.select({"type": "system", "id": home["id"]})
    ui.modal = None
    ui.draw()
    design_btn = next((c for c in ui.clickables if c[1] == "open_designer"), None)
    ok.append(("designer button present", design_btn is not None))
    if design_btn:
        ui.do_action("open_designer", None, design_btn[0].center)
        ok.append(("designer modal opens", ui.modal == "designer"))
        ui.do_action("design_hull", "cruiser", None)
        ui.do_action("design_weapon", "w2", None)
        ui.do_action("design_armor", "a2", None)
        ui.do_action("design_engine", "e2", None)
        ui.d_name = "Test Runner"
        ui.draw()
        save_btn = next((c for c in ui.clickables if c[1] == "save_design"), None)
        ok.append(("save design button present", save_btn is not None))
        if save_btn:
            ui.do_action("save_design", None, save_btn[0].center)
            designs = engine.list_designs(0)
            ok.append(("design saved", len(designs) == 1))
            if designs:
                d = engine.ship_def(designs[0])
                ok.append(("design stats computed",
                           d["hp"] == round(250 * 2.2) and d["attack"] == 38 and d["speed"] == 360))
                ui.draw()
                qbtn = next((c for c in ui.clickables
                             if c[1] == "queue_ship" and c[2] == designs[0]), None)
                ok.append(("custom ship buildable", qbtn is not None))
                if qbtn:
                    ui.do_action("queue_ship", designs[0], qbtn[0].center)
                    ok.append(("custom ship queued",
                               home["planet"]["colony"]["queue"][-1] == designs[0]))
                ui.do_action("open_designer", None, None)
                ui.draw()
                delbtn = next((c for c in ui.clickables if c[1] == "delete_design"), None)
                if delbtn:
                    ui.do_action("delete_design", delbtn[2], delbtn[0].center)
                    ok.append(("design deleted", len(engine.list_designs(0)) == 0))

    # --- 13. AI empires design their own ships
    import random as _random
    _random.seed(2024)   # combat/anomaly rolls use the global RNG; pin it for determinism
    engine.new_game({"numAI": 3, "numSystems": 46, "difficulty": "normal",
                     "playerName": "Terran Consortium", "seed": 7})
    ui.modal = None
    for _ in range(100):
        engine.end_turn(ai)
    ai_designs = sum(len(engine.list_designs(e["id"])) for e in engine.state["empires"] if not e["isPlayer"])
    ok.append(("AI empires created designs", ai_designs >= 1))
    ai_custom_ships = sum(
        1 for f in engine.state["fleets"] for s in f["ships"] if s["cls"].startswith("custom:"))
    print("  (info) AI custom ships fielded after 100 turns: %d" % ai_custom_ships)
    # funded, the AI must pick and be able to build its own design
    ai_emp = next((e for e in engine.state["empires"]
                   if not e["isPlayer"] and engine.list_designs(e["id"])), None)
    if ai_emp:
        ai_emp["credits"] = 9999
        ai_emp["minerals"] = 9999
        picked = ai.choose_ship(ai_emp)
        ok.append(("AI prefers its design when affordable",
                   isinstance(picked, str) and picked.startswith("custom:")))
        col = next(iter(engine.empire_colonies(ai_emp["id"])), None)
        if col and picked:
            ok.append(("AI queues its design", engine.queue_ship(col, picked)))

    # --- 14. long simulation through the UI's end_turn
    errs = []
    for _ in range(40):
        try:
            ui.end_turn()
        except Exception as exc:  # noqa: BLE001
            errs.append(repr(exc))
            break
    ok.append(("40 UI turns, no exceptions", not errs))

    # --- 15. modal buttons must work through the real mouse-click path
    engine.new_game({"numAI": 3, "numSystems": 46, "difficulty": "normal",
                     "playerName": "Terran Consortium", "seed": 3})
    ui.modal = "new"
    ui.name_input = "Click Test"
    ui.opt_size = 34
    ui.draw()
    start_btn = next((c for c in ui.clickables if c[1] == "start_new_game"), None)
    ok.append(("start button is clickable", start_btn is not None))
    if start_btn:
        old_state = engine.state
        centre = start_btn[0].center
        ui.drag = {"start": centre, "last": centre, "moved": False}
        ui.onMouseUp(_Ev(button=1, pos=centre))
        ok.append(("clicking Start Game starts a game",
                   engine.state is not old_state and ui.modal is None
                   and len(engine.state["systems"]) == 34))

    # a click that hits nothing while a modal is open must not reach the map
    ui.modal = "new"
    ui.draw()
    ui.handle_click((5, 5))
    ok.append(("click outside modal is swallowed", ui.modal == "new"))

    # with no modal, clicking empty space deselects
    ui.modal = None
    ui.draw()
    engine.select({"type": "system", "id": engine.state["systems"][0]["id"]})
    ui.handle_click((5, 5))
    ok.append(("click on empty space deselects", engine.state["selected"] is None))

    print()
    all_ok = True
    for label, passed in ok:
        print("  %-38s %s" % (label, "PASS" if passed else "FAIL"))
        all_ok = all_ok and passed
    if errs:
        print("  errors:", errs)
    print()
    print("ALL PASS" if all_ok else "SOME FAILURES")
    pygame.quit()
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())
