"""Stellar Dominion — pygame-ce frontend.

Owns the window, input, map rendering, HUD and modals. Reads engine state and
calls engine actions; never mutates game rules itself.
"""

import math
import os
import pygame

from . import ai
from . import data
from . import engine
from . import galaxy

# Set SD_DEBUG_CLICKS=1 to record every click (position, modal, hit action).
_CLICK_LOG = os.path.expanduser("~/.stellar_dominion_clicks.log")


def _log_click(msg):
    if os.environ.get("SD_DEBUG_CLICKS") != "1":
        return
    try:
        with open(_CLICK_LOG, "a", encoding="utf-8") as fh:
            fh.write(msg + "\n")
    except OSError:
        pass

# ---------------------------------------------------------------------------
# Theme
# ---------------------------------------------------------------------------
BG = (5, 7, 13)
BG2 = (11, 16, 32)
PANEL = (14, 21, 38)
PANEL2 = (19, 28, 51)
BORDER = (31, 44, 77)
TEXT = (214, 226, 245)
MUTED = (125, 141, 176)
ACCENT = (77, 208, 225)
ACCENT2 = (124, 77, 255)
GOOD = (102, 187, 106)
BAD = (239, 83, 80)
WAR = (255, 112, 67)
RESEARCH = (66, 165, 245)
GOLD = (255, 213, 79)
WHITE = (255, 255, 255)
BLACK = (4, 6, 12)

TOP_H = 52
SIDEBAR_W = 280
PANEL_H = 232
MINIMAP_W, MINIMAP_H = 256, 148
MM_SCALE = min((MINIMAP_W - 16) / galaxy.WIDTH, (MINIMAP_H - 12) / galaxy.HEIGHT)
MM_OX = (MINIMAP_W - galaxy.WIDTH * MM_SCALE) / 2
MM_OY = (MINIMAP_H - galaxy.HEIGHT * MM_SCALE) / 2


def rgb(hexcolor):
    hexcolor = hexcolor.lstrip("#")
    return (int(hexcolor[0:2], 16), int(hexcolor[2:4], 16), int(hexcolor[4:6], 16))


def rgba(hexcolor, alpha):
    r, g, b = rgb(hexcolor)
    return (r, g, b, alpha)


# ---------------------------------------------------------------------------
# UI helpers
# ---------------------------------------------------------------------------
class UI:
    def __init__(self):
        pygame.init()
        pygame.display.set_caption("Stellar Dominion")
        self.screen = pygame.display.set_mode((1360, 800), pygame.RESIZABLE)
        self.clock = pygame.time.Clock()
        self.running = True

        self.cam = {"x": 1800, "y": 1200, "zoom": 0.45}
        self.starfield = []
        self.nebulae = []
        self._make_background()

        self.clickables = []      # [(rect, action, arg, enabled)] rebuilt every frame
        self._modal_click_start = 0
        self.mouse = (0, 0)
        self.hover = None
        self.modal = None         # 'new' | 'research' | 'diplomacy' | 'battles' | 'help' | 'gameover' | 'text'
        self.toast_msg = None
        self.toast_until = 0
        self.log_scroll = 0
        self.name_input = "Terran Consortium"
        self.name_focus = False
        self.d_hull = data.HULLS[0]["key"]
        self.d_weapon = data.WEAPON_MODULES[0]["key"]
        self.d_armor = data.ARMOR_MODULES[0]["key"]
        self.d_engine = data.ENGINE_MODULES[0]["key"]
        self.d_name = ""
        self.d_name_focus = False
        self.opt_size = 46
        self.opt_ai = 3
        self.opt_diff = "normal"
        self.drag = None
        self._build_fonts()

        engine.toast_handler = self.toast

    def _build_fonts(self):
        families = ["segoeui", "tahoma", "calibri", "arial"]
        def make(size, bold=False):
            for fam in families:
                try:
                    return pygame.font.SysFont(fam, size, bold=bold)
                except Exception:
                    continue
            return pygame.font.Font(None, size)

        self.f_title = make(15, True)
        self.f_h = make(11, True)
        self.f = make(12)
        self.f_small = make(11)
        self.f_tiny = make(10)
        self.f_map = make(11)
        self.f_btn = make(12)
        self.f_btn_bold = make(12, True)

    def _make_background(self):
        import random
        rng = random.Random(12345)
        self.starfield = [{
            "x": rng.uniform(0, galaxy.WIDTH), "y": rng.uniform(0, galaxy.HEIGHT),
            "r": rng.uniform(0.4, 1.9), "a": rng.uniform(0.25, 0.85),
        } for _ in range(1400)]
        colors = ["#1b2a5e", "#3a1b5e", "#0e3a4a", "#4a1b2a", "#1b4a3a", "#2a2a6e"]
        self.nebulae = [{
            "x": rng.uniform(0, galaxy.WIDTH), "y": rng.uniform(0, galaxy.HEIGHT),
            "r": rng.uniform(400, 950), "color": rng.choice(colors),
        } for _ in range(9)]
        self._scale_cache = {}
        # soft glow sprites per star colour
        self.glows = {s["color"]: self._radial(rgb(s["color"]), 96, 150, 2.2) for s in data.STAR_CLASSES}
        # soft nebula blobs
        self.nebula_surfs = [(self._radial(rgb(n["color"]), 192, 130, 1.9), n) for n in self.nebulae]
        # galactic band (two overlapping ribbons)
        self.band_surf = self._make_band()

    @staticmethod
    def _radial(color, radius, peak_alpha, power):
        surf = pygame.Surface((radius * 2, radius * 2), pygame.SRCALPHA)
        steps = 40
        for i in range(steps, 0, -1):
            f = i / steps
            a = int(peak_alpha * (1 - f) ** power)
            pygame.draw.circle(surf, (color[0], color[1], color[2], a),
                               (radius, radius), max(1, int(radius * f)))
        return surf

    def _make_band(self):
        bw, bh = 720, 260
        band = pygame.Surface((bw, bh), pygame.SRCALPHA)
        for base, peak, off in (((120, 140, 220), 120, 0),
                                ((90, 110, 190), 90, 18)):
            for i in range(40, 0, -1):
                f = i / 40
                a = int(peak * (1 - f) ** 1.9)
                pygame.draw.ellipse(band, (base[0], base[1], base[2], a),
                                    (bw / 2 - (bw / 2) * f,
                                     bh / 2 - (bh / 2) * f + off * f,
                                     bw * f, bh * f))
        return band

    def _scaled(self, surf, size):
        key = (id(surf), size)
        got = self._scale_cache.get(key)
        if got is None:
            got = pygame.transform.smoothscale(surf, (size, size))
            if len(self._scale_cache) > 600:
                self._scale_cache.clear()
            self._scale_cache[key] = got
        return got

    # ------------------------------------------------------------------
    # Layout
    # ------------------------------------------------------------------
    def layout(self):
        w, h = self.screen.get_size()
        self.map_rect = pygame.Rect(0, TOP_H, max(50, w - SIDEBAR_W), max(50, h - TOP_H - PANEL_H))
        self.side_rect = pygame.Rect(w - SIDEBAR_W, TOP_H, SIDEBAR_W, max(50, h - TOP_H - PANEL_H))
        self.panel_rect = pygame.Rect(0, h - PANEL_H, w, PANEL_H)
        self.top_rect = pygame.Rect(0, 0, w, TOP_H)

    # ------------------------------------------------------------------
    # Coordinate helpers
    # ------------------------------------------------------------------
    def w2s(self, wx, wy):
        return ((wx - self.cam["x"]) * self.cam["zoom"] + self.map_rect.w / 2,
                (wy - self.cam["y"]) * self.cam["zoom"] + self.map_rect.h / 2)

    def s2w(self, sx, sy):
        return ((sx - self.map_rect.w / 2) / self.cam["zoom"] + self.cam["x"],
                (sy - self.map_rect.h / 2) / self.cam["zoom"] + self.cam["y"])

    def w2screen(self, wx, wy):
        """World coordinates -> absolute screen coordinates (for hit testing)."""
        p = self.w2s(wx, wy)
        return (p[0] + self.map_rect.x, p[1] + self.map_rect.y)

    # ------------------------------------------------------------------
    # Drawing helpers
    # ------------------------------------------------------------------
    def text(self, surf, s, pos, font=None, color=TEXT, bold=False):
        font = font or (self.f_btn_bold if bold else self.f)
        surf.blit(font.render(str(s), True, color), pos)

    def text_width(self, s, font=None):
        font = font or self.f
        return font.size(str(s))[0]

    def wrap(self, s, font, max_w):
        words = str(s).split(" ")
        lines, cur = [], ""
        for w in words:
            trial = (cur + " " + w).strip()
            if font.size(trial)[0] <= max_w or not cur:
                cur = trial
            else:
                lines.append(cur)
                cur = w
        if cur:
            lines.append(cur)
        return lines

    def rect(self, surf, r, fill=None, border=None, width=1):
        if fill:
            pygame.draw.rect(surf, fill, r)
        if border:
            pygame.draw.rect(surf, border, r, width)

    def button(self, surf, r, label, action=None, arg=None, enabled=True,
               style="normal", font=None, sub=None):
        """Draw a button, register it as clickable, return hover bool."""
        font = font or self.f_btn
        hovered = r.collidepoint(self.mouse) and enabled
        if style == "primary":
            fill = (20, 77, 92) if enabled else (30, 40, 60)
            border = ACCENT if enabled else BORDER
        elif style == "danger":
            fill = PANEL2 if enabled else (30, 40, 60)
            border = BAD if enabled else BORDER
        elif style == "small":
            fill = (40, 54, 92) if hovered else PANEL2
            border = ACCENT if hovered else BORDER
        else:
            fill = (40, 54, 92) if hovered else PANEL2
            border = ACCENT if hovered else BORDER
        if not enabled:
            fill = (24, 32, 52)
        pygame.draw.rect(surf, fill, r)
        pygame.draw.rect(surf, border, r, 1)

        label_color = TEXT if enabled else (90, 100, 125)
        if sub:
            self.text(surf, label, (r.x + 8, r.y + 5), font, label_color, True)
            self.text(surf, sub, (r.x + 8, r.y + 20), self.f_tiny, MUTED if enabled else (80, 90, 115))
        else:
            tw = font.size(label)[0]
            self.text(surf, label, (r.x + (r.w - tw) / 2, r.y + (r.h - font.size(label)[1]) / 2),
                      font, label_color, style == "primary")

        if action and enabled:
            self.clickables.append((r, action, arg, True))
        return hovered

    def toast(self, msg):
        self.toast_msg = msg
        self.toast_until = pygame.time.get_ticks() + 2200

    def hint(self, surf, s, pos, color=MUTED):
        self.text(surf, s, pos, self.f_tiny, color)

    # ------------------------------------------------------------------
    # Event handling
    # ------------------------------------------------------------------
    def handle_events(self):
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                self.running = False
            elif event.type == pygame.VIDEORESIZE:
                self.screen = pygame.display.set_mode((event.w, event.h), pygame.RESIZABLE)
                self.layout()
            elif event.type == pygame.MOUSEBUTTONDOWN:
                self.onMouseDown(event)
            elif event.type == pygame.MOUSEBUTTONUP:
                self.onMouseUp(event)
            elif event.type == pygame.MOUSEMOTION:
                self.mouse = event.pos
                self.onMotion(event)
            elif event.type == pygame.MOUSEWHEEL:
                self.onWheel(event)
            elif event.type == pygame.KEYDOWN:
                self.onKey(event)

    def onMouseDown(self, event):
        _log_click("DOWN button=%s pos=%s" % (getattr(event, "button", None), getattr(event, "pos", None)))
        if event.button == 1:
            self.drag = {"start": event.pos, "last": event.pos, "moved": False}
        elif event.button == 3:
            if self.modal:
                self.modal = None
            elif engine.state:
                engine.select(None)

    def onMouseUp(self, event):
        _log_click("UP button=%s pos=%s drag=%s" % (getattr(event, "button", None),
                                                    getattr(event, "pos", None), self.drag))
        if event.button == 1 and self.drag:
            # Modal buttons live in `clickables` too, so process every click —
            # handle_click() gives modal entries priority and swallows the rest.
            if not self.drag["moved"]:
                self.handle_click(event.pos)
            self.drag = None

    def onMotion(self, event):
        if self.drag:
            dx = event.pos[0] - self.drag["last"][0]
            dy = event.pos[1] - self.drag["last"][1]
            if abs(event.pos[0] - self.drag["start"][0]) + abs(event.pos[1] - self.drag["start"][1]) > 4:
                self.drag["moved"] = True
            if self.drag["moved"] and self.map_rect.collidepoint(event.pos):
                self.cam["x"] -= dx / self.cam["zoom"]
                self.cam["y"] -= dy / self.cam["zoom"]
                self.clamp_cam()
            self.drag["last"] = event.pos

    def onWheel(self, event):
        if self.modal:
            return
        self.mouse = event.pos
        if self.side_rect.collidepoint(self.mouse):
            self.log_scroll = max(0, self.log_scroll - event.y * 3)
            return
        if self.map_rect.collidepoint(self.mouse):
            mx, my = self.mouse[0] - self.map_rect.x, self.mouse[1] - self.map_rect.y
            before = self.s2w(mx, my)
            self.cam["zoom"] = max(0.12, min(2.6, self.cam["zoom"] * (1.12 if event.y > 0 else 0.89)))
            after = self.s2w(mx, my)
            self.cam["x"] += before[0] - after[0]
            self.cam["y"] += before[1] - after[1]
            self.clamp_cam()

    def onKey(self, event):
        if self.modal == "designer" and self.d_name_focus:
            if event.key == pygame.K_BACKSPACE:
                self.d_name = self.d_name[:-1]
            elif event.key == pygame.K_RETURN:
                self.d_name_focus = False
            elif event.unicode and event.unicode.isprintable() and len(self.d_name) < 28:
                self.d_name += event.unicode
            return
        if self.modal == "new" and self.name_focus:
            if event.key == pygame.K_BACKSPACE:
                self.name_input = self.name_input[:-1]
            elif event.key == pygame.K_RETURN:
                self.name_focus = False
            elif event.unicode and event.unicode.isprintable() and len(self.name_input) < 30:
                self.name_input += event.unicode
            return
        if event.key == pygame.K_ESCAPE:
            if self.modal:
                self.modal = None
            elif engine.state:
                engine.select(None)
        elif event.key == pygame.K_RETURN and not self.modal and engine.state and not engine.state["over"]:
            self.end_turn()
        elif event.key == pygame.K_s and (event.mod & pygame.KMOD_CTRL) and engine.state:
            engine.save_game()
            self.toast("Game saved")

    def clamp_cam(self):
        self.cam["x"] = max(-300, min(galaxy.WIDTH + 300, self.cam["x"]))
        self.cam["y"] = max(-300, min(galaxy.HEIGHT + 300, self.cam["y"]))

    def handle_click(self, pos):
        # Modals take priority — their clickables were registered last, and only
        # those are live while the overlay is up.
        start = self._modal_click_start if self.modal else 0
        for rect, action, arg, enabled in reversed(self.clickables[start:]):
            if enabled and rect.collidepoint(pos):
                _log_click("pos=%s modal=%s -> action=%s arg=%s" % (pos, self.modal, action, arg))
                self.do_action(action, arg, pos)
                return
        _log_click("pos=%s modal=%s -> NO HIT" % (pos, self.modal))
        # While a modal is open a click that hits nothing must not fall through
        # to the map behind the overlay.
        if not self.modal:
            engine.select(None)

    def do_action(self, action, arg, pos):
        g = engine
        if action == "select_system":
            sel = g.state["selected"]
            if sel and sel["type"] == "fleet":
                fleet = g.get_fleet(sel["id"])
                sys = g.get_system(arg)
                if fleet and sys and fleet["empireId"] == 0 and fleet["systemId"] != sys["id"]:
                    if g.issue_move(fleet, sys):
                        self.toast("Fleet moving to %s" % sys["name"])
                    return
            g.select({"type": "system", "id": arg})
        elif action == "select_fleet":
            g.select({"type": "fleet", "id": arg})
        elif action == "minimap":
            self.minimap_jump(pos)
        elif action == "cycle_size":
            self.opt_size = {"34": 46, "46": 60, "60": 34}[str(self.opt_size)]
        elif action == "cycle_ai":
            self.opt_ai = {1: 2, 2: 3, 3: 1}[self.opt_ai]
        elif action == "cycle_diff":
            self.opt_diff = {"easy": "normal", "normal": "hard", "hard": "easy"}[self.opt_diff]
        elif action == "open_new":
            self.modal = "new"
        elif action == "build_building":
            sel = g.state["selected"]
            if sel and sel["type"] == "system":
                g.build_building(g.get_system(sel["id"]), arg)
        elif action == "queue_ship":
            sel = g.state["selected"]
            if sel and sel["type"] == "system":
                g.queue_ship(g.get_system(sel["id"]), arg)
        elif action == "split_fleet":
            sel = g.state["selected"]
            if sel and sel["type"] == "fleet":
                g.split_fleet(g.get_fleet(sel["id"]))
        elif action == "stop_fleet":
            sel = g.state["selected"]
            if sel and sel["type"] == "fleet":
                g.stop_fleet(g.get_fleet(sel["id"]))
        elif action == "merge_into":
            target, source = arg.split(":")
            g.merge_fleets(int(target), int(source))
        elif action == "merge_all":
            f = g.get_fleet(arg)
            if f:
                g.merge_all_fleets_at(f["systemId"], f["empireId"])
        elif action == "set_tech":
            g.set_research(g.player(), arg)
        elif action == "declare_war":
            g.declare_war(g.player()["id"], arg)
        elif action == "sue_peace":
            g.player_sue_peace(g.get_empire(arg))
        elif action == "accept_peace":
            g.make_peace(g.player()["id"], arg)
        elif action == "save":
            g.save_game()
        elif action == "end_turn":
            self.end_turn()
        elif action == "focus_name":
            self.name_focus = True
        elif action == "start_new_game":
            self.start_new_game()
        elif action == "load_save":
            if g.load_game():
                self.modal = None
                self.focus_home()
                self.toast("Saved game loaded")
        elif action == "close_modal":
            self.modal = None
        elif action == "open_designer":
            self.open_designer()
        elif action == "design_name_focus":
            self.d_name_focus = True
        elif action == "design_hull":
            self.d_hull = arg
        elif action == "design_weapon":
            self.d_weapon = arg
        elif action == "design_armor":
            self.d_armor = arg
        elif action == "design_engine":
            self.d_engine = arg
        elif action == "save_design":
            self.save_design()
        elif action == "delete_design":
            g.remove_design(arg)
        elif action == "research":
            self.modal = "research"
        elif action == "diplomacy":
            self.modal = "diplomacy"
        elif action == "battles":
            self.modal = "battles"
        elif action == "help":
            self.modal = "help"
        elif action == "home":
            self.focus_home()
        elif action == "new":
            self.modal = "new"

    def minimap_jump(self, pos):
        rect = pygame.Rect(self.side_rect.x + 12, self.side_rect.y + 24, MINIMAP_W, MINIMAP_H)
        if not rect.collidepoint(pos):
            return
        mx = (pos[0] - rect.x) * (MINIMAP_W / rect.w)
        my = (pos[1] - rect.y) * (MINIMAP_H / rect.h)
        wx = (mx - MM_OX) / MM_SCALE
        wy = (my - MM_OY) / MM_SCALE
        self.cam["x"] = wx
        self.cam["y"] = wy
        self.clamp_cam()

    def end_turn(self):
        engine.end_turn(ai)

    # ------------------------------------------------------------------
    # Frame
    # ------------------------------------------------------------------
    def draw(self):
        self.layout()
        self.clickables = []
        self.screen.fill(BG)

        if engine.state:
            self.draw_top_bar(self.screen)
            self.draw_map(self.screen)
            self.draw_sidebar(self.screen)
            self.draw_panel(self.screen)
        else:
            self.screen.fill(BG)

        if self.modal:
            # Clicks while a modal is open only reach the modal's own buttons
            self._modal_click_start = len(self.clickables)
            self.draw_modal()

        self.draw_toast()
        self.draw_map_hint()

    def draw_map_hint(self):
        if self.modal or not engine.state:
            return
        w, h = self.screen.get_size()
        s = ("Click a system or fleet to select · with a fleet selected, click a system to move · "
             "drag to pan · scroll to zoom · right-click to deselect")
        tw = self.text_width(s, self.f_tiny)
        box = pygame.Rect(w / 2 - tw / 2 - 8, self.map_rect.bottom - 26, tw + 16, 18)
        pygame.draw.rect(self.screen, (8, 12, 24, 200), box)
        self.text(self.screen, s, (box.x + 8, box.y + 3), self.f_tiny, MUTED)

    def draw_toast(self):
        if not self.toast_msg:
            return
        if pygame.time.get_ticks() > self.toast_until:
            self.toast_msg = None
            return
        s = self.toast_msg
        tw = self.text_width(s, self.f)
        box = pygame.Rect(self.screen.get_width() / 2 - tw / 2 - 12, 58, tw + 24, 26)
        pygame.draw.rect(self.screen, (14, 21, 38, 235), box)
        pygame.draw.rect(self.screen, ACCENT, box, 1)
        self.text(self.screen, s, (box.x + 12, box.y + 6), self.f, TEXT)

    # ------------------------------------------------------------------
    # Top bar
    # ------------------------------------------------------------------
    def draw_top_bar(self, surf):
        pygame.draw.rect(surf, PANEL, self.top_rect)
        pygame.draw.line(surf, BORDER, (0, TOP_H - 1), (surf.get_width(), TOP_H - 1))

        x = 14
        self.text(surf, "STELLAR DOMINION", (x, 17), self.f_title, ACCENT, True)
        x += self.text_width("STELLAR DOMINION", self.f_title) + 20

        p = engine.player()
        inc = p["lastIncome"]
        pool = int(p.get("researchPool", 0))
        research = ("%d/%d" % (int(p["techProgress"]), data.TECHS[p["currentTech"]]["cost"])
                    if p["currentTech"] else "—")
        items = [
            "Turn %d" % engine.state["turn"],
            "Credits %d  +%d" % (p["credits"], round(inc["credits"])),
            "Minerals %d  +%d" % (p["minerals"], round(inc["minerals"])),
            "Research %s  +%d%s" % (research, round(inc["research"]),
                                    ("  (%d banked)" % pool) if pool else ""),
        ]
        for s in items:
            self.text(surf, s, (x, 18), self.f, TEXT)
            x += self.text_width(s, self.f) + 16

        # Buttons laid out from the right edge
        buttons = [
            ("Research", "research", None, "normal"),
            ("Diplomacy", "diplomacy", None, "normal"),
            ("Battles", "battles", None, "normal"),
            ("Home", "home", None, "normal"),
            ("Save", "save", None, "normal"),
            ("New Game", "new", None, "normal"),
            ("Help", "help", None, "normal"),
            ("End Turn", "end_turn", None, "primary"),
        ]
        widths = [self.text_width(label, self.f_btn) + 22 for label, _, _, _ in buttons]
        bx = surf.get_width() - 14 - sum(widths) - 8 * (len(buttons) - 1)
        bx = max(x + 10, bx)
        for (label, action, arg, style), wdt in zip(buttons, widths):
            self.button(surf, pygame.Rect(bx, 10, wdt, 32), label, action, arg, style=style)
            bx += wdt + 8

    # ------------------------------------------------------------------
    # Map
    # ------------------------------------------------------------------
    def draw_map(self, surf):
        try:
            ms = surf.subsurface(self.map_rect)
        except ValueError:
            return

        ms.fill((4, 6, 12))
        zoom = self.cam["zoom"]
        zoom_f = math.sqrt(zoom)

        # galactic band
        band_c = self.w2s(galaxy.WIDTH / 2, galaxy.HEIGHT / 2)
        band_w = 2400 * zoom
        band_h = band_w * 0.32
        band_img = pygame.transform.smoothscale(self.band_surf, (max(4, int(band_w)), max(4, int(band_h))))
        ms.blit(band_img, (int(band_c[0] - band_w / 2), int(band_c[1] - band_h / 2)))

        # nebulae
        for blob, n in self.nebula_surfs:
            p = self.w2s(n["x"], n["y"])
            target_r = n["r"] * zoom * 0.55
            if target_r < 2:
                continue
            if (p[0] + target_r < 0 or p[0] - target_r > ms.get_width()
                    or p[1] + target_r < 0 or p[1] - target_r > ms.get_height()):
                continue
            size = max(4, int(target_r * 2 / 8) * 8)
            ms.blit(self._scaled(blob, size), (int(p[0] - target_r), int(p[1] - target_r)))

        # starfield
        for st in self.starfield:
            p = self.w2s(st["x"], st["y"])
            if p[0] < 0 or p[0] > ms.get_width() or p[1] < 0 or p[1] > ms.get_height():
                continue
            b = int(200 * st["a"])
            pygame.draw.circle(ms, (b, b, min(255, b + 30)), (int(p[0]), int(p[1])), max(1, int(st["r"])))

        # move lines
        for f in engine.state["fleets"]:
            if not f["moves"] or not self.fleet_visible(f):
                continue
            a = self.w2s(f["x"], f["y"])
            to_sys = engine.get_system(f["moves"]["toSystemId"])
            b = self.w2s(to_sys["x"], to_sys["y"])
            e = engine.get_empire(f["empireId"])
            self._dashed_line(ms, (int(a[0]), int(a[1])), (int(b[0]), int(b[1])), rgb(e["color"]))

        # systems
        for s in engine.state["systems"]:
            self.draw_system(ms, s, zoom_f)

        # fleets
        for f in engine.state["fleets"]:
            if self.fleet_visible(f):
                self.draw_fleet(ms, f, zoom_f)

        self.draw_selection(ms)

    def _dashed_line(self, surf, a, b, color):
        length = math.hypot(b[0] - a[0], b[1] - a[1])
        if length < 1:
            return
        steps = int(length // 8)
        for i in range(0, steps, 2):
            t0 = i / steps
            t1 = min(1.0, (i + 1) / steps)
            pygame.draw.line(surf, color,
                             (a[0] + (b[0] - a[0]) * t0, a[1] + (b[1] - a[1]) * t0),
                             (a[0] + (b[0] - a[0]) * t1, a[1] + (b[1] - a[1]) * t1), 1)

    def fleet_visible(self, f):
        sys = engine.get_system(f["systemId"])
        return bool(sys and 0 in sys["exploredBy"])

    def draw_system(self, surf, s, zoom_f):
        p = self.w2s(s["x"], s["y"])
        explored = 0 in s["exploredBy"]
        r = max(4.5, 7 * s["starRadius"] * zoom_f)
        if p[0] < -60 or p[0] > surf.get_width() + 60 or p[1] < -60 or p[1] > surf.get_height() + 60:
            return

        base = rgb(s["starColor"])
        glow = self.glows.get(s["starColor"])
        if glow:
            gr = r * 2.4
            size = max(4, int(gr * 2 / 4) * 4)
            surf.blit(self._scaled(glow, size), (int(p[0] - gr), int(p[1] - gr)))

        pygame.draw.circle(surf, base if explored else (90, 100, 120), (int(p[0]), int(p[1])), int(r))

        if explored and s["ownerEmpireId"] is not None:
            e = engine.get_empire(s["ownerEmpireId"])
            pygame.draw.circle(surf, rgb(e["color"]), (int(p[0]), int(p[1])), int(r + 4), 2)
        elif not explored:
            pygame.draw.circle(surf, (125, 141, 176), (int(p[0]), int(p[1])), int(r + 3), 1)

        label = s["name"] if explored else "Unknown"
        col = (214, 226, 245) if explored else (125, 141, 176)
        lw = self.f_map.size(label)[0]
        surf.blit(self.f_map.render(label, True, col), (int(p[0] - lw / 2), int(p[1] + r + 15 * zoom_f + 2)))

        # click target (converted to absolute screen coordinates)
        hit = pygame.Rect(p[0] + self.map_rect.x - (r + 8), p[1] + self.map_rect.y - (r + 8),
                          (r + 8) * 2, (r + 8) * 2)
        self.clickables.append((hit, "select_system", s["id"], True))

    def fleet_draw_pos(self, f):
        if f["moves"]:
            return (f["x"], f["y"])
        sys = engine.get_system(f["systemId"])
        idle = [x for x in engine.fleets_at(sys["id"]) if not x["moves"]]
        try:
            idx = max(0, idle.index(f))
        except ValueError:
            idx = 0
        angle = -math.pi / 2 + (idx % 8) * (math.pi * 2 / 8)
        return (sys["x"] + math.cos(angle) * 46, sys["y"] + math.sin(angle) * 46)

    def draw_fleet(self, surf, f, zoom_f):
        pos = self.fleet_draw_pos(f)
        p = self.w2s(pos[0], pos[1])
        e = engine.get_empire(f["empireId"])
        color = rgb(e["color"])
        fs = max(7, 9 * zoom_f)
        dest = engine.get_system(f["moves"]["toSystemId"]) if f["moves"] else None
        angle = math.atan2(dest["y"] - f["y"], dest["x"] - f["x"]) if dest else -math.pi / 2

        pts = [(fs, 0), (-fs * 0.7, fs * 0.62), (-fs * 0.4, 0), (-fs * 0.7, -fs * 0.62)]
        ca, sa = math.cos(angle + math.pi / 2), math.sin(angle + math.pi / 2)
        rotated = [(p[0] + x * ca - y * sa, p[1] + x * sa + y * ca) for x, y in pts]
        pygame.draw.polygon(surf, color, rotated)
        pygame.draw.polygon(surf, BLACK, rotated, 1)

        cnt = str(len(f["ships"]))
        surf.blit(self.f_small.render(cnt, True, (235, 242, 255)),
                  (int(p[0] + fs + 2), int(p[1] - fs)))

        hit = pygame.Rect(p[0] + self.map_rect.x - 12, p[1] + self.map_rect.y - 12, 24, 24)
        self.clickables.append((hit, "select_fleet", f["id"], True))

    def draw_selection(self, surf):
        sel = engine.state["selected"]
        if not sel:
            return
        if sel["type"] == "system":
            s = engine.get_system(sel["id"])
            if not s:
                return
            p = self.w2s(s["x"], s["y"])
            r = max(6, 9 * s["starRadius"] * math.sqrt(self.cam["zoom"])) + 8
            self._dashed_circle(surf, (int(p[0]), int(p[1])), int(r), WHITE)
        else:
            f = engine.get_fleet(sel["id"])
            if not f:
                return
            pos = self.fleet_draw_pos(f)
            p = self.w2s(pos[0], pos[1])
            pygame.draw.circle(surf, WHITE, (int(p[0]), int(p[1])), 12, 1)

    def _dashed_circle(self, surf, center, radius, color):
        steps = 40
        for i in range(0, steps, 2):
            a0 = (i / steps) * math.pi * 2
            a1 = ((i + 1) / steps) * math.pi * 2
            pygame.draw.arc(surf, color,
                            (center[0] - radius, center[1] - radius, radius * 2, radius * 2), a0, a1, 1)

    # ------------------------------------------------------------------
    # Sidebar
    # ------------------------------------------------------------------
    def draw_sidebar(self, surf):
        pygame.draw.rect(surf, PANEL, self.side_rect)
        pygame.draw.line(surf, BORDER, (self.side_rect.x, self.side_rect.y),
                         (self.side_rect.x, self.side_rect.bottom))

        y = self.side_rect.y + 6
        self.text(surf, "GALAXY", (self.side_rect.x + 12, y), self.f_tiny, MUTED, True)
        y += 18

        mm_rect = pygame.Rect(self.side_rect.x + 12, y, MINIMAP_W, MINIMAP_H)
        try:
            mms = surf.subsurface(mm_rect)
            self.draw_minimap(mms)
        except ValueError:
            pass
        # minimap click
        hit = mm_rect.inflate(2, 2)
        self.clickables.append((hit, "minimap", None, True))
        y += MINIMAP_H + 10

        self.text(surf, "EVENT LOG", (self.side_rect.x + 12, y), self.f_tiny, MUTED, True)
        y += 18

        log_rect = pygame.Rect(self.side_rect.x + 6, y, SIDEBAR_W - 12,
                               self.side_rect.bottom - y - 6)
        try:
            ls = surf.subsurface(log_rect)
        except ValueError:
            return
        pygame.draw.rect(ls, BG2, (0, 0, log_rect.w, log_rect.h))
        line_h = 14
        avail_w = log_rect.w - 46
        logs = engine.state["logs"][-120:]
        display = []
        for entry in logs:
            col = {
                "good": GOOD, "bad": BAD, "war": WAR, "research": RESEARCH,
                "build": GOLD, "info": TEXT,
            }.get(entry["type"], TEXT)
            tag = "[%d]" % entry["turn"]
            lines = self.wrap(entry["msg"], self.f_tiny, avail_w)
            for j, line in enumerate(lines):
                display.append((tag if j == 0 else "", line, col))
        max_scroll = max(0, len(display) * line_h - log_rect.h)
        self.log_scroll = max(0, min(self.log_scroll, max_scroll))
        for i, (tag, line, col) in enumerate(display):
            yy = i * line_h - self.log_scroll
            if yy < -line_h or yy > log_rect.h:
                continue
            if tag:
                ls.blit(self.f_tiny.render(tag, True, (84, 98, 138)), (2, yy))
            ls.blit(self.f_tiny.render(line, True, col), (40, yy))

    def draw_minimap(self, surf):
        surf.fill((7, 11, 22))
        for s in engine.state["systems"]:
            x = MM_OX + s["x"] * MM_SCALE
            y = MM_OY + s["y"] * MM_SCALE
            explored = 0 in s["exploredBy"]
            if explored and s["ownerEmpireId"] is not None:
                e = engine.get_empire(s["ownerEmpireId"])
                col = rgb(e["color"])
            elif explored:
                col = (141, 156, 196)
            else:
                col = (58, 69, 96)
            pygame.draw.circle(surf, col, (int(x), int(y)), 2)

        for f in engine.state["fleets"]:
            if not self.fleet_visible(f):
                continue
            pos = self.fleet_draw_pos(f)
            x = MM_OX + pos[0] * MM_SCALE
            y = MM_OY + pos[1] * MM_SCALE
            e = engine.get_empire(f["empireId"])
            pygame.draw.rect(surf, rgb(e["color"]), (int(x) - 1, int(y) - 1, 3, 3))

        # viewport
        tl = self.s2w(0, 0)
        br = self.s2w(self.map_rect.w, self.map_rect.h)
        vx = MM_OX + tl[0] * MM_SCALE
        vy = MM_OY + tl[1] * MM_SCALE
        vw = (br[0] - tl[0]) * MM_SCALE
        vh = (br[1] - tl[1]) * MM_SCALE
        pygame.draw.rect(surf, ACCENT, (vx, vy, vw, vh), 1)

    # ------------------------------------------------------------------
    # Bottom panel
    # ------------------------------------------------------------------
    def draw_panel(self, surf):
        pygame.draw.rect(surf, PANEL, self.panel_rect)
        pygame.draw.line(surf, BORDER, (0, self.panel_rect.y), (surf.get_width(), self.panel_rect.y))

        x0 = self.panel_rect.x + 14
        y0 = self.panel_rect.y + 10
        w = self.panel_rect.w - 28

        sel = engine.state["selected"]
        if not sel:
            self.panel_overview(surf, x0, y0, w)
        elif sel["type"] == "system":
            s = engine.get_system(sel["id"])
            if s:
                self.panel_system(surf, s, x0, y0, w)
        elif sel["type"] == "fleet":
            f = engine.get_fleet(sel["id"])
            if f:
                self.panel_fleet(surf, f, x0, y0, w)

    def panel_overview(self, surf, x, y, w):
        p = engine.player()
        self.text(surf, p["name"], (x, y), self.f_title, ACCENT, True)
        self.text(surf, p["desc"], (x, y + 22), self.f_small, MUTED)
        stats = [
            ("Colonies", len(engine.empire_colonies(p["id"]))),
            ("Systems controlled", len(engine.empire_systems(p["id"]))),
            ("Fleets", len(engine.empire_fleets(p["id"]))),
            ("Ships", engine.empire_ship_count(p["id"])),
            ("Fleet strength", round(engine.empire_strength(p["id"]))),
            ("Research", data.TECHS[p["currentTech"]]["name"] if p["currentTech"] else "none"),
        ]
        sx = x
        sy = y + 44
        for label, val in stats:
            self.text(surf, label, (sx, sy), self.f_tiny, MUTED)
            self.text(surf, str(val), (sx, sy + 13), self.f, TEXT, True)
            sx += 150
        self.hint(surf, "Select a system to inspect it, or a fleet to command it.  Press Enter to end the turn.",
                  (x, y + 82))

    def panel_system(self, surf, s, x, y, w):
        explored = 0 in s["exploredBy"]
        star = data.STAR_BY_KEY.get(s["starClass"], {})
        self.text(surf, s["name"], (x, y), self.f_title, ACCENT, True)

        if not explored:
            self.text(surf, "Unexplored system", (x, y + 22), self.f_small, MUTED)
            self.hint(surf, "Send a fleet within sensor range to reveal this system's planets and owner.",
                      (x, y + 44))
            return

        pt = data.PLANET_TYPES[s["planet"]["type"]]
        owner = engine.get_empire(s["ownerEmpireId"]) if s["ownerEmpireId"] is not None else None
        self.text(surf, "%s · %s world" % (star.get("name", ""), pt["name"]),
                  (x, y + 22), self.f_small, MUTED)

        size_name = ["Small", "Medium", "Large"][s["planet"]["size"] - 1]
        stats = [
            ("Planet", pt["name"], rgb(pt["color"])),
            ("Size", size_name, TEXT),
            ("Richness", "%.2fx" % s["planet"]["richness"], TEXT),
            ("Max population", str(s["planet"]["maxPop"]), TEXT),
            ("Owner", owner["name"] if owner else "Unclaimed",
             rgb(owner["color"]) if owner else MUTED),
        ]
        sx = x
        sy = y + 42
        for label, val, col in stats:
            self.text(surf, label, (sx, sy), self.f_tiny, MUTED)
            self.text(surf, str(val), (sx, sy + 13), self.f, col, True)
            sx += 140

        col = s["planet"]["colony"]
        ly = y + 82

        if s["anomaly"]:
            self.text(surf, "Anomaly — %s: %s" % (s["anomaly"]["name"], s["anomaly"]["desc"]),
                      (x, ly), self.f_small, GOLD)
            ly += 18

        if col:
            c = engine.get_empire(col["empireId"])
            prod = engine.colony_production(s, col, c)
            self.text(surf, "COLONY — %s" % c["name"], (x, ly), self.f_h, MUTED, True)
            self.text(surf, "Population %d / %d" % (col["population"], engine.max_pop(s, c)),
                      (x + 230, ly), self.f_small, TEXT)
            self.text(surf, "Income +%.1f min · +%.1f cr · +%.1f rp" % (
                prod["minerals"], prod["credits"], prod["research"]),
                (x + 400, ly), self.f_small, TEXT)
            ly += 20
            bx = x
            for b in col["buildings"]:
                label = data.BUILDINGS[b]["name"]
                bw = self.text_width(label, self.f_small) + 14
                r = pygame.Rect(bx, ly, bw, 20)
                pygame.draw.rect(surf, PANEL2, r)
                pygame.draw.rect(surf, BORDER, r, 1)
                self.text(surf, label, (bx + 7, ly + 4), self.f_small, TEXT)
                bx += bw + 5
            ly += 26
            if col["queue"]:
                self.text(surf, "Construction", (x, ly), self.f_h, MUTED, True)
                ly += 15
                for i, key in enumerate(col["queue"]):
                    ship = engine.ship_def(key)
                    prog = " (%d/%d)" % (col["queueProgress"], ship["buildTurns"]) if i == 0 else ""
                    self.text(surf, ship["name"] + prog, (x, ly), self.f_small,
                              GOLD if i == 0 else MUTED)
                    ly += 14
        else:
            p = engine.player()
            if engine.can_colonize(p, s):
                self.hint(surf, "Uncolonized. Send a Colony Ship to found a colony here.", (x, ly))
            elif data.PLANET_TYPES[s["planet"]["type"]]["habitability"] <= 0:
                self.hint(surf, "This world cannot support a colony.", (x, ly))
            else:
                self.hint(surf, "This hostile world requires Xeno Colonization research.", (x, ly))

        # Right column — build options for the player's own colonies
        if col:
            c = engine.get_empire(col["empireId"])
            if c["isPlayer"]:
                bx0 = x + 760
                by = y + 42
                self.text(surf, "BUILDINGS", (bx0, by), self.f_h, MUTED, True)
                by += 17
                bx = bx0
                for key, b in data.BUILDINGS.items():
                    if not b["repeat"] and key in col["buildings"]:
                        continue
                    if len(col["buildings"]) >= engine.slots(s):
                        continue
                    label = "%s %dmin" % (b["name"], b["cost"])
                    bw = self.text_width(label, self.f_small) + 14
                    self.button(surf, pygame.Rect(bx, by, bw, 22), label, "build_building", key,
                                enabled=engine.player()["minerals"] >= b["cost"], style="small",
                                font=self.f_small)
                    bx += bw + 5
                by += 28
                if "shipyard" in col["buildings"]:
                    self.text(surf, "SHIPS", (bx0, by), self.f_h, MUTED, True)
                    self.button(surf, pygame.Rect(bx0 + 78, by - 3, 118, 18), "+ Design a Ship",
                                "open_designer", None, style="small", font=self.f_tiny)
                    by += 17
                    bx = bx0
                    ship_items = [(k, data.SHIPS[k]) for k in data.SHIPS]
                    ship_items += [(k, engine.ship_def(k))
                                   for k in engine.list_designs(engine.player()["id"])]
                    for key, ship in ship_items:
                        if ship["tech"] and not engine.player()["techs"].get(ship["tech"]):
                            continue
                        cost = engine.ship_cost(engine.player(), key)
                        tag = "" if key in data.SHIPS else "* "
                        label = "%s%s %dcr %dmin %dt" % (tag, ship["name"], cost["credits"],
                                                         cost["minerals"], ship["buildTurns"])
                        bw = self.text_width(label, self.f_small) + 14
                        self.button(surf, pygame.Rect(bx, by, bw, 22), label, "queue_ship", key,
                                    enabled=(engine.player()["credits"] >= cost["credits"]
                                             and engine.player()["minerals"] >= cost["minerals"]),
                                    style="small", font=self.f_small)
                        bx += bw + 5

    def panel_fleet(self, surf, f, x, y, w):
        e = engine.get_empire(f["empireId"])
        sys = engine.get_system(f["systemId"])
        self.text(surf, "Fleet — %s" % e["name"], (x, y), self.f_title, ACCENT, True)
        dest = engine.get_system(f["moves"]["toSystemId"]) if f["moves"] else None
        sub = "At %s" % sys["name"]
        if dest:
            sub += "  →  %s (%d turn%s)" % (dest["name"], f["moves"]["turnsLeft"],
                                            "" if f["moves"]["turnsLeft"] == 1 else "s")
        sub += "   ·   Speed %d   ·   Strength %d" % (engine.fleet_speed(f),
                                                     round(engine.fleet_strength(f)))
        self.text(surf, sub, (x, y + 22), self.f_small, MUTED)

        self.text(surf, "Ships (%d)" % len(f["ships"]), (x, y + 42), self.f_h, MUTED, True)
        bx = x
        by = y + 58
        for s in f["ships"]:
            st = engine.effective_ship_stats(e, s["cls"])
            label = engine.ship_def(s["cls"])["name"]
            bw = self.text_width(label, self.f_small) + 14 + 60
            r = pygame.Rect(bx, by, bw, 20)
            pygame.draw.rect(surf, PANEL2, r)
            pygame.draw.rect(surf, BORDER, r, 1)
            self.text(surf, label, (bx + 7, by + 4), self.f_small, TEXT)
            pct = max(0, min(100, int(100 * s["hp"] / st["hp"])))
            pygame.draw.rect(surf, (34, 42, 61), (bx + bw - 58, by + 6, 50, 8))
            pygame.draw.rect(surf, GOOD, (bx + bw - 58, by + 6, int(50 * pct / 100), 8))
            bx += bw + 5
            if bx > x + w - bw:
                bx = x
                by += 24

        by += 30
        if e["isPlayer"]:
            others = [o for o in engine.fleets_at(f["systemId"])
                      if o["id"] != f["id"] and o["empireId"] == e["id"] and not o["moves"]]
            self.button(surf, pygame.Rect(x, by, 96, 24), "Split Fleet", "split_fleet", None,
                        enabled=len(f["ships"]) >= 2, style="small")
            self.button(surf, pygame.Rect(x + 102, by, 60, 24), "Stop", "stop_fleet", None,
                        enabled=bool(f["moves"]), style="small")
            if others:
                self.button(surf, pygame.Rect(x + 168, by, 150, 24),
                            "Merge All Here (%d)" % (len(others) + 1), "merge_all", f["id"],
                            style="small")
            self.hint(surf, "Click a system on the map to move this fleet there.", (x + 340, by + 5))
            by += 32
            if others:
                self.text(surf, "Other fleets here", (x, by), self.f_h, MUTED, True)
                by += 16
                for o in others:
                    names = ", ".join(engine.ship_def(s["cls"])["name"] for s in o["ships"])
                    self.text(surf, names, (x, by), self.f_small, TEXT)
                    self.button(surf, pygame.Rect(x + 260, by - 2, 60, 20), "Merge",
                                "merge_into", "%s:%s" % (f["id"], o["id"]), style="small",
                                font=self.f_small)
                    by += 20
        else:
            war = engine.at_war(e["id"], 0)
            self.hint(surf, "Hostile fleet. " + (
                "Engage it by moving a fleet into its system." if war
                else "You are not at war with this empire."), (x, by))

    # ------------------------------------------------------------------
    # Modals
    # ------------------------------------------------------------------
    def draw_modal(self):
        w, h = self.screen.get_size()
        overlay = pygame.Surface((w, h), pygame.SRCALPHA)
        overlay.fill((3, 5, 12, 200))
        self.screen.blit(overlay, (0, 0))

        if self.modal == "gameover":
            self.modal_gameover(self.screen, w, h)
        elif self.modal == "new":
            self.modal_new(self.screen, w, h)
        elif self.modal == "research":
            self.modal_research(self.screen, w, h)
        elif self.modal == "diplomacy":
            self.modal_diplomacy(self.screen, w, h)
        elif self.modal == "battles":
            self.modal_battles(self.screen, w, h)
        elif self.modal == "help":
            self.modal_help(self.screen, w, h)
        elif self.modal == "designer":
            self.modal_designer(self.screen, w, h)

    def _modal_box(self, w, h, box_w, box_h):
        r = pygame.Rect((w - box_w) / 2, (h - box_h) / 2, box_w, box_h)
        pygame.draw.rect(self.screen, PANEL, r)
        pygame.draw.rect(self.screen, BORDER, r, 1)
        return r

    def _close_button(self, r):
        self.button(self.screen, pygame.Rect(r.right - 78, r.bottom - 42, 66, 26),
                    "Close", "close_modal", style="small")

    def modal_new(self, surf, w, h):
        box = self._modal_box(w, h, 460, 420)
        self.text(surf, "Stellar Dominion", (box.x + 20, box.y + 16), self.f_title, ACCENT, True)
        body = ("A turn-based 4X space strategy game. Explore, expand, exploit and exterminate "
                "across a procedurally generated galaxy.")
        yy = box.y + 44
        for line in self.wrap(body, self.f, box.w - 40):
            self.text(surf, line, (box.x + 20, yy), self.f, TEXT)
            yy += 16
        yy += 10

        self.text(surf, "Empire name", (box.x + 20, yy), self.f_small, MUTED)
        yy += 18
        r = pygame.Rect(box.x + 20, yy, box.w - 40, 26)
        pygame.draw.rect(surf, PANEL2, r)
        pygame.draw.rect(surf, ACCENT if self.name_focus else BORDER, r, 1)
        self.text(surf, self.name_input, (r.x + 8, r.y + 6), self.f, TEXT)
        self.clickables.append((r, "focus_name", None, True))
        yy += 36

        opts = [
            ("Galaxy size", ["34", "46", "60"], self.opt_size, "size"),
            ("Rivals", ["1", "2", "3"], self.opt_ai, "ai"),
            ("Difficulty", ["easy", "normal", "hard"], self.opt_diff, "diff"),
        ]
        for label, choices, current, key in opts:
            self.text(surf, label, (box.x + 20, yy), self.f_small, MUTED)
            r = pygame.Rect(box.x + 160, yy - 3, 120, 24)
            self.button(surf, r, str(current), "cycle_" + key, None, style="small")
            yy += 30

        yy += 8
        if engine.has_save():
            self.button(surf, pygame.Rect(box.x + 20, box.bottom - 48, 130, 28),
                        "Load Saved Game", "load_save", style="small")
        self.button(surf, pygame.Rect(box.right - 150, box.bottom - 50, 130, 30),
                    "Start Game", "start_new_game", style="primary")

    def start_new_game(self):
        engine.new_game({
            "numAI": self.opt_ai,
            "numSystems": self.opt_size,
            "difficulty": self.opt_diff,
            "playerName": self.name_input.strip() or "Terran Consortium",
        })
        self.modal = None
        self.focus_home()
        self.toast("Welcome to Stellar Dominion")

    def open_designer(self):
        self.d_hull = data.HULLS[0]["key"]
        self.d_weapon = data.WEAPON_MODULES[0]["key"]
        self.d_armor = data.ARMOR_MODULES[0]["key"]
        self.d_engine = data.ENGINE_MODULES[0]["key"]
        self.d_name = ""
        self.d_name_focus = False
        self.modal = "designer"

    def save_design(self):
        name = self.d_name.strip() or "Custom Design"
        key = engine.add_design(engine.player()["id"], name, self.d_hull,
                                self.d_weapon, self.d_armor, self.d_engine)
        self.toast("Design saved: %s" % engine.ship_def(key)["name"])
        self.modal = None

    def _module_row(self, surf, x, y, label, modules, current, action, empire):
        self.text(surf, label, (x, y + 3), self.f_small, MUTED)
        bx = x + 66
        for m in modules:
            locked = bool(m["tech"]) and not empire["techs"].get(m["tech"])
            selected = m["key"] == current
            text = m["name"] + (" *" if locked else "")
            bw = self.text_width(text, self.f_small) + 14
            r = pygame.Rect(bx, y, bw, 22)
            fill = (28, 68, 88) if selected else PANEL2
            border = ACCENT if selected else BORDER
            pygame.draw.rect(surf, fill, r)
            pygame.draw.rect(surf, border, r, 1)
            self.text(surf, text, (bx + 7, y + 5), self.f_small,
                      TEXT if not locked else (90, 100, 125))
            if not locked:
                self.clickables.append((r, action, m["key"], True))
            bx += bw + 5
        sel = next((m for m in modules if m["key"] == current), modules[0])
        self.text(surf, sel["desc"], (x + 66, y + 25), self.f_tiny, MUTED)
        return y + 42

    def modal_designer(self, surf, w, h):
        box = self._modal_box(w, h, 580, 560)
        self.text(surf, "Ship Designer", (box.x + 20, box.y + 16), self.f_title, ACCENT, True)
        self.text(surf, "Combine a hull with weapon, armour and engine modules.",
                  (box.x + 20, box.y + 40), self.f_small, MUTED)

        empire = engine.player()
        y = box.y + 62
        self.text(surf, "Design name", (box.x + 20, y + 4), self.f_small, MUTED)
        r = pygame.Rect(box.x + 130, y - 3, box.w - 150, 24)
        pygame.draw.rect(surf, PANEL2, r)
        pygame.draw.rect(surf, ACCENT if self.d_name_focus else BORDER, r, 1)
        self.text(surf, self.d_name if self.d_name else "Custom Design",
                  (r.x + 8, r.y + 5), self.f, TEXT if self.d_name else MUTED)
        self.clickables.append((r, "design_name_focus", None, True))
        y += 32

        y = self._module_row(surf, box.x + 20, y, "Hull", data.HULLS, self.d_hull,
                             "design_hull", empire)
        y = self._module_row(surf, box.x + 20, y, "Weapon", data.WEAPON_MODULES,
                             self.d_weapon, "design_weapon", empire)
        y = self._module_row(surf, box.x + 20, y, "Armour", data.ARMOR_MODULES,
                             self.d_armor, "design_armor", empire)
        y = self._module_row(surf, box.x + 20, y, "Engine", data.ENGINE_MODULES,
                             self.d_engine, "design_engine", empire)

        d = data.compute_design(self.d_name.strip() or "Custom Design",
                                self.d_hull, self.d_weapon, self.d_armor, self.d_engine)
        py = y + 6
        pygame.draw.rect(surf, PANEL2, pygame.Rect(box.x + 20, py, box.w - 40, 44))
        self.text(surf, "HP %d   ·   Attack %d   ·   Speed %d   ·   %dcr %dmin   ·   %d turns" % (
            d["hp"], d["attack"], d["speed"], d["credits"], d["minerals"], d["buildTurns"]),
            (box.x + 30, py + 6), self.f, TEXT)
        req = []
        for m in (data.HULL_BY_KEY[self.d_hull], data.WEAPON_BY_KEY[self.d_weapon],
                  data.ARMOR_BY_KEY[self.d_armor], data.ENGINE_BY_KEY[self.d_engine]):
            if m["tech"] and not empire["techs"].get(m["tech"]):
                req.append(data.TECHS[m["tech"]]["name"])
        self.text(surf, ("Requires: " + ", ".join(req)) if req else "No special research required",
                  (box.x + 30, py + 24), self.f_tiny, WAR if req else GOOD)

        dy = py + 56
        self.text(surf, "YOUR DESIGNS", (box.x + 20, dy), self.f_h, MUTED, True)
        dy += 16
        designs = engine.list_designs(empire["id"])
        if not designs:
            self.hint(surf, "No custom designs yet — configure one above and save it.",
                      (box.x + 20, dy))
            dy += 16
        for key in designs:
            if dy > box.bottom - 58:
                break
            dd = engine.ship_def(key)
            self.text(surf, dd["name"], (box.x + 20, dy), self.f_small, TEXT)
            self.text(surf, dd["desc"], (box.x + 200, dy + 1), self.f_tiny, MUTED)
            self.button(surf, pygame.Rect(box.right - 96, dy - 2, 76, 20), "Delete",
                        "delete_design", key, style="danger", font=self.f_tiny)
            dy += 22

        self.button(surf, pygame.Rect(box.right - 290, box.bottom - 42, 130, 28),
                    "Save Design", "save_design", style="primary")
        self._close_button(box)

    def focus_home(self):
        p = engine.player()
        home = engine.get_system(p["homeSystemId"])
        if home:
            self.cam = {"x": home["x"], "y": home["y"], "zoom": 0.7}

    def modal_research(self, surf, w, h):
        box = self._modal_box(w, h, 680, 560)
        self.text(surf, "Research", (box.x + 20, box.y + 16), self.f_title, ACCENT, True)
        p = engine.player()
        intro = "Research points accumulate each turn from your colonies."
        if p.get("researchPool", 0) > 0:
            intro += "  %d RP banked." % int(p["researchPool"])
        self.text(surf, intro, (box.x + 20, box.y + 40), self.f_small, MUTED)

        y = box.y + 62
        for cat in data.TECH_CATS:
            self.text(surf, cat.upper(), (box.x + 20, y), self.f_h, MUTED, True)
            y += 18
            for key, t in data.TECHS.items():
                if t["cat"] != cat:
                    continue
                done = key in p["techs"]
                current = p["currentTech"] == key
                locked = not all(r in p["techs"] for r in t["requires"])
                col = GOOD if done else (RESEARCH if current else (MUTED if locked else TEXT))
                self.text(surf, t["name"], (box.x + 28, y), self.f, col, current)
                desc = t["desc"]
                if t["requires"] and not locked:
                    desc += "  (requires %s)" % ", ".join(data.TECHS[r]["name"] for r in t["requires"])
                self.text(surf, desc, (box.x + 230, y + 1), self.f_tiny, MUTED)
                status = "Researched" if done else ("In progress" if current else ("Locked" if locked else "%d RP" % t["cost"]))
                self.text(surf, status, (box.x + 470, y), self.f_small, col)
                if not done and not current and not locked:
                    self.button(surf, pygame.Rect(box.right - 120, y - 3, 96, 22),
                                "Research", "set_tech", key, style="small", font=self.f_small)
                if current:
                    bw = 200
                    pct = max(0, min(100, 100 * p["techProgress"] / t["cost"]))
                    pygame.draw.rect(surf, (34, 42, 61), (box.x + 470, y + 15, bw, 5))
                    pygame.draw.rect(surf, RESEARCH, (box.x + 470, y + 15, int(bw * pct / 100), 5))
                    self.text(surf, "%d / %d" % (int(p["techProgress"]), t["cost"]),
                              (box.x + 678, y + 12), self.f_tiny, MUTED)
                y += 24
            y += 6
        self._close_button(box)

    def modal_diplomacy(self, surf, w, h):
        box = self._modal_box(w, h, 560, 380)
        self.text(surf, "Diplomacy", (box.x + 20, box.y + 16), self.f_title, ACCENT, True)
        self.text(surf, "Relations with the other empires of the galaxy.",
                  (box.x + 20, box.y + 40), self.f_small, MUTED)

        p = engine.player()
        my_str = round(engine.empire_strength(p["id"]))
        y = box.y + 66
        for e in engine.state["empires"]:
            if e["id"] == p["id"] or not e["alive"]:
                continue
            at_war = e["id"] in p["atWarWith"]
            offered = e["id"] in p["peaceOffers"]
            their_str = round(engine.empire_strength(e["id"]))
            self.text(surf, e["name"], (box.x + 20, y), self.f, rgb(e["color"]), True)
            status = "AT WAR" if at_war else "PEACE"
            self.text(surf, status, (box.x + 200, y), self.f_small, WAR if at_war else GOOD, True)
            self.text(surf, "Strength %d (you: %d)  ·  Colonies %d  ·  Ships %d" % (
                their_str, my_str, len(engine.empire_colonies(e["id"])), engine.empire_ship_count(e["id"])),
                (box.x + 20, y + 17), self.f_tiny, MUTED)
            if offered:
                self.text(surf, "Peace offer received.", (box.x + 20, y + 31), self.f_tiny, GOOD)
            by = y + (48 if offered else 32)
            if offered:
                self.button(surf, pygame.Rect(box.right - 170, y, 150, 22),
                            "Accept Peace", "accept_peace", e["id"], style="small", font=self.f_small)
            elif at_war:
                self.button(surf, pygame.Rect(box.right - 170, y, 150, 22),
                            "Sue for Peace", "sue_peace", e["id"], style="small", font=self.f_small)
            else:
                self.button(surf, pygame.Rect(box.right - 170, y, 150, 22),
                            "Declare War", "declare_war", e["id"], style="danger", font=self.f_small)
            y = by + 8
        self._close_button(box)

    def modal_battles(self, surf, w, h):
        box = self._modal_box(w, h, 640, 520)
        self.text(surf, "Battle Reports", (box.x + 20, box.y + 16), self.f_title, ACCENT, True)
        self.text(surf, "Recent engagements across the galaxy, newest first.",
                  (box.x + 20, box.y + 40), self.f_small, MUTED)
        y = box.y + 64
        battles = list(reversed(engine.state["battles"]))
        if not battles:
            self.hint(surf, "No battles have been fought yet.", (box.x + 20, y))
        for b in battles:
            if y > box.bottom - 60:
                self.hint(surf, "…", (box.x + 20, y), MUTED)
                break
            self.text(surf, b["system"], (box.x + 20, y), self.f, TEXT, True)
            tag = "COLONY ASSAULT" if b["kind"] == "colony" else "SPACE BATTLE"
            self.text(surf, tag, (box.x + 180, y + 1), self.f_tiny, WAR if b["kind"] == "colony" else MUTED)
            self.text(surf, "Turn %d" % b["turn"], (box.right - 90, y), self.f_tiny, RESEARCH)
            self.text(surf, " vs ".join(b["participants"]), (box.x + 20, y + 15), self.f_small, MUTED)
            self.text(surf, "Winner: %s   ·   Losses: %d ship%s" % (
                b["winner"] or "none", b["losses"], "" if b["losses"] == 1 else "s"),
                (box.x + 20, y + 30), self.f_tiny, TEXT)
            y += 52
        self._close_button(box)

    def modal_help(self, surf, w, h):
        box = self._modal_box(w, h, 620, 560)
        self.text(surf, "How to Play", (box.x + 20, box.y + 16), self.f_title, ACCENT, True)
        y = box.y + 44
        lines = [
            ("Goal", True),
            ("Win by eliminating all rivals, or by controlling 55% of colonizable worlds after turn 20.", False),
            ("", False),
            ("Controls", True),
            ("Click a system or fleet to select it. With a fleet selected, click a system to move there.", False),
            ("Drag to pan, scroll to zoom, click the minimap to jump, right-click or Esc to deselect.", False),
            ("Enter ends the turn. Ctrl+S saves.", False),
            ("", False),
            ("Expansion", True),
            ("Build a Colony Ship at a colony with a Shipyard, then send it to an uncolonized world.", False),
            ("Colonies grow population over time; bigger populations produce more of everything.", False),
            ("", False),
            ("Construction", True),
            ("Colonies have limited building slots. Mines boost minerals, Markets credits, Labs research,", False),
            ("Habitats speed growth, Shipyards enable ships, Planetary Shields add defense platforms.", False),
            ("", False),
            ("Combat", True),
            ("Hostile fleets in the same system fight at the end of the turn. Destroy the defenders and", False),
            ("the attacker captures the colony. Fleet speed comes from the slowest ship.", False),
            ("", False),
            ("Exploration", True),
            ("Systems start unexplored. Move fleets within sensor range to reveal them — and to claim", False),
            ("anomalies, which only a fleet survey (not colony sensors) can uncover.", False),
        ]
        for text, header in lines:
            if header:
                self.text(surf, text, (box.x + 20, y), self.f_h, MUTED, True)
                y += 18
            elif text:
                self.text(surf, text, (box.x + 20, y), self.f_small, TEXT)
                y += 16
            else:
                y += 6
        self._close_button(box)

    def modal_gameover(self, surf, w, h):
        box = self._modal_box(w, h, 460, 200)
        won = engine.state["winner"] is engine.player()
        winner = engine.state["winner"]
        title = "Victory!" if won else "Defeat"
        col = GOOD if won else BAD
        self.text(surf, title, (box.x + 20, box.y + 22), self.f_title, col, True)
        if won:
            msg = "Your empire dominates the galaxy. The stars themselves bow to your banner."
        elif winner:
            msg = "%s has conquered your empire." % winner["name"]
        else:
            msg = "Your empire has been wiped out."
        yy = box.y + 52
        for line in self.wrap(msg, self.f, box.w - 40):
            self.text(surf, line, (box.x + 20, yy), self.f, TEXT)
            yy += 16
        self.text(surf, "Turns played: %d" % engine.state["turn"], (box.x + 20, yy + 8),
                  self.f_small, MUTED)
        self.button(surf, pygame.Rect(box.right - 130, box.bottom - 46, 110, 28),
                    "New Game", "open_new", style="primary")

    # ------------------------------------------------------------------
    # Main loop
    # ------------------------------------------------------------------
    def _foreground(self):
        """Bring the game window to the front (Windows only)."""
        if os.name != "nt":
            return
        try:
            import ctypes

            hwnd = pygame.display.get_wm_info().get("window")
            if hwnd:
                ctypes.windll.user32.SetForegroundWindow(int(hwnd))
        except Exception:
            pass

    def run(self):
        self.layout()
        self.modal = "new"
        self._foreground()
        while self.running:
            self.handle_events()
            # keep the window in front until a game exists, so the first click on
            # Start Game cannot be swallowed by another window
            if not engine.state:
                self._foreground()
            self.draw()
            pygame.display.flip()
            self.clock.tick(60)
        pygame.quit()


def main():
    ui = UI()
    ui.run()


if __name__ == "__main__":
    main()
