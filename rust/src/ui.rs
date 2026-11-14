//! macroquad frontend: window, input, map rendering, HUD and panels.
//!
//! Reads simulation state and calls engine actions; never mutates game rules.

use ::rand::rngs::StdRng;
use ::rand::SeedableRng;
use macroquad::prelude::*;

use crate::data;
use crate::engine::{self, NewGameOpts};
use crate::galaxy;
use crate::sim::Sim;
use crate::state::State;

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------
fn c(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

fn with_alpha(col: Color, a: u8) -> Color {
    Color {
        r: col.r,
        g: col.g,
        b: col.b,
        a: a as f32 / 255.0,
    }
}

const BG: Color = Color::from_rgba(6, 9, 16, 255);
const BG2: Color = Color::from_rgba(12, 18, 34, 255);
const PANEL: Color = Color::from_rgba(16, 24, 44, 255);
const PANEL2: Color = Color::from_rgba(25, 36, 63, 255);
const BORDER: Color = Color::from_rgba(48, 66, 108, 255);
const TEXT: Color = Color::from_rgba(230, 238, 252, 255);
const MUTED: Color = Color::from_rgba(160, 175, 208, 255);
const ACCENT: Color = Color::from_rgba(94, 220, 235, 255);
const ACCENT2: Color = Color::from_rgba(124, 77, 255, 255);
const GOOD: Color = Color::from_rgba(102, 187, 106, 255);
const BAD: Color = Color::from_rgba(239, 83, 80, 255);
const WAR: Color = Color::from_rgba(255, 112, 67, 255);
const RESEARCH: Color = Color::from_rgba(66, 165, 245, 255);
const GOLD: Color = Color::from_rgba(255, 213, 79, 255);
const WHITE: Color = Color::from_rgba(255, 255, 255, 255);
const BLACK: Color = Color::from_rgba(4, 6, 12, 255);

const TOP_H: f32 = 52.0;
const SIDEBAR_W: f32 = 280.0;
const PANEL_H: f32 = 232.0;
const MINIMAP_W: f32 = 256.0;
const MINIMAP_H: f32 = 148.0;

pub fn rgb(hexcolor: &str) -> Color {
    let h = hexcolor.trim_start_matches('#');
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(255);
    Color::from_rgba(r, g, b, 255)
}

// ---------------------------------------------------------------------------
// Actions and clickables
// ---------------------------------------------------------------------------
#[derive(Clone)]
pub enum Action {
    SelectSystem(usize),
    SelectFleet(usize),
    Minimap,
    CycleSize,
    CycleAi,
    CycleDiff,
    OpenNew,
    BuildBuilding(String),
    QueueShip(String),
    SplitFleet,
    StopFleet,
    MergeInto(usize, usize),
    MergeAll(usize),
    SetTech(String),
    DeclareWar(usize),
    SuePeace(usize),
    AcceptPeace(usize),
    Save,
    EndTurn,
    FocusName,
    StartNewGame,
    LoadSave,
    CloseModal,
    OpenDesigner,
    DesignNameFocus,
    DesignHull(String),
    DesignWeapon(String),
    DesignArmor(String),
    DesignEngine(String),
    SaveDesign,
    DeleteDesign(String),
    Research,
    Diplomacy,
    Battles,
    Help,
    Home,
    New,
}

#[derive(Clone)]
struct Clickable {
    r: Rect,
    action: Action,
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Normal,
    Primary,
    Danger,
    Small,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Modal {
    New,
    Research,
    Diplomacy,
    Battles,
    Help,
    Designer,
    GameOver,
}

#[derive(Clone, Copy, PartialEq)]
enum Diff {
    Easy,
    Normal,
    Hard,
}

impl Diff {
    fn label(self) -> &'static str {
        match self {
            Diff::Easy => "easy",
            Diff::Normal => "normal",
            Diff::Hard => "hard",
        }
    }
    fn next(self) -> Diff {
        match self {
            Diff::Easy => Diff::Normal,
            Diff::Normal => Diff::Hard,
            Diff::Hard => Diff::Easy,
        }
    }
}

#[derive(Clone)]
struct Drag {
    start: Vec2,
    last: Vec2,
    moved: bool,
}

struct Star {
    x: f32,
    y: f32,
    r: f32,
    a: f32,
}

struct Nebula {
    x: f32,
    y: f32,
    r: f32,
    color: Color,
}

pub struct Ui {
    cam_x: f32,
    cam_y: f32,
    zoom: f32,
    /// Global UI scale: the interface is laid out at a 1024x640 reference and
    /// multiplied by this, so text and panels stay legible on large or hi-DPI
    /// displays instead of shrinking with the window.
    pub scale: f32,
    starfield: Vec<Star>,
    nebulae: Vec<Nebula>,
    clickables: Vec<Clickable>,
    modal_click_start: usize,
    drag: Option<Drag>,
    pub modal: Option<Modal>,
    toast: Option<String>,
    toast_until: f64,
    log_scroll: f32,
    name_input: String,
    name_focus: bool,
    opt_size: i32,
    opt_ai: i32,
    opt_diff: Diff,
    d_hull: String,
    d_weapon: String,
    d_armor: String,
    d_engine: String,
    d_name: String,
    d_name_focus: bool,
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

fn right(r: Rect) -> f32 {
    r.x + r.w
}
fn bottom(r: Rect) -> f32 {
    r.y + r.h
}

impl Ui {
    pub fn new() -> Self {
        let mut rng = StdRng::seed_from_u64(12345);
        use ::rand::Rng;
        let starfield: Vec<Star> = (0..1400)
            .map(|_| Star {
                x: rng.gen_range(0.0..galaxy::WIDTH),
                y: rng.gen_range(0.0..galaxy::HEIGHT),
                r: rng.gen_range(0.4..1.9f32),
                a: rng.gen_range(0.25..0.85f32),
            })
            .collect();
        let colors = [
            "#1b2a5e", "#3a1b5e", "#0e3a4a", "#4a1b2a", "#1b4a3a", "#2a2a6e",
        ];
        let nebulae: Vec<Nebula> = (0..9)
            .map(|_| Nebula {
                x: rng.gen_range(0.0..galaxy::WIDTH),
                y: rng.gen_range(0.0..galaxy::HEIGHT),
                r: rng.gen_range(400.0..950.0f32),
                color: rgb(colors[rng.gen_range(0..colors.len())]),
            })
            .collect();

        Ui {
            cam_x: 1800.0,
            cam_y: 1200.0,
            zoom: 0.45,
            scale: Self::compute_scale(),
            starfield,
            nebulae,
            clickables: Vec::new(),
            modal_click_start: 0,
            drag: None,
            modal: Some(Modal::New),
            toast: None,
            toast_until: 0.0,
            log_scroll: 0.0,
            name_input: "Terran Consortium".to_string(),
            name_focus: false,
            opt_size: 46,
            opt_ai: 3,
            opt_diff: Diff::Normal,
            d_hull: data::hulls()[0].key.to_string(),
            d_weapon: data::weapon_modules()[0].key.to_string(),
            d_armor: data::armor_modules()[0].key.to_string(),
            d_engine: data::engine_modules()[0].key.to_string(),
            d_name: String::new(),
            d_name_focus: false,
        }
    }

    // ------------------------------------------------------------------
    // Layout and coordinates
    // ------------------------------------------------------------------
    /// Interface scale for the current window. The design reference is
    /// 1024x640; larger windows scale the whole interface up.
    fn compute_scale() -> f32 {
        let w = screen_width();
        let h = screen_height();
        if w <= 1.0 || h <= 1.0 {
            return 1.0;
        }
        (w / 1024.0).min(h / 640.0).clamp(0.9, 2.5)
    }

    /// Scale a design-pixel length (padding, radius, line width).
    fn u(&self, v: f32) -> f32 {
        v * self.scale
    }

    fn layout(&self) -> (Rect, Rect, Rect, Rect) {
        let w = screen_width();
        let h = screen_height();
        let s = self.scale;
        let top_h = TOP_H * s;
        let side_w = SIDEBAR_W * s;
        let panel_h = PANEL_H * s;
        let map = rect(
            0.0,
            top_h,
            (w - side_w).max(50.0),
            (h - top_h - panel_h).max(50.0),
        );
        let side = rect(w - side_w, top_h, side_w, (h - top_h - panel_h).max(50.0));
        let panel = rect(0.0, h - panel_h, w, panel_h);
        let top = rect(0.0, 0.0, w, top_h);
        (map, side, panel, top)
    }

    /// Minimap geometry: (width, height, world->map scale, offset x, offset y).
    fn minimap_geom(&self) -> (f32, f32, f32, f32, f32) {
        let mw = MINIMAP_W * self.scale;
        let mh = MINIMAP_H * self.scale;
        let sc = ((mw - self.u(16.0)) / galaxy::WIDTH).min((mh - self.u(12.0)) / galaxy::HEIGHT);
        let ox = (mw - galaxy::WIDTH * sc) / 2.0;
        let oy = (mh - galaxy::HEIGHT * sc) / 2.0;
        (mw, mh, sc, ox, oy)
    }

    fn w2s(&self, map: Rect, wx: f32, wy: f32) -> Vec2 {
        vec2(
            (wx - self.cam_x) * self.zoom + map.w / 2.0,
            (wy - self.cam_y) * self.zoom + map.h / 2.0,
        )
    }

    fn s2w(&self, map: Rect, sx: f32, sy: f32) -> Vec2 {
        vec2(
            (sx - map.w / 2.0) / self.zoom + self.cam_x,
            (sy - map.h / 2.0) / self.zoom + self.cam_y,
        )
    }

    fn clamp_cam(&mut self) {
        self.cam_x = self.cam_x.clamp(-300.0, galaxy::WIDTH + self.u(300.0));
        self.cam_y = self.cam_y.clamp(-300.0, galaxy::HEIGHT + self.u(300.0));
    }

    // ------------------------------------------------------------------
    // Text helpers
    // ------------------------------------------------------------------
    fn text(&self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        draw_text(s, x, y, size * self.scale, color);
    }

    fn text_width(&self, s: &str, size: f32) -> f32 {
        measure_text(s, None, (size * self.scale).max(1.0) as u16, 1.0).width
    }

    fn wrap(&self, s: &str, size: f32, max_w: f32) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut cur = String::new();
        for word in s.split(' ') {
            let trial = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", cur, word)
            };
            if self.text_width(&trial, size) <= max_w || cur.is_empty() {
                cur = trial;
            } else {
                lines.push(cur);
                cur = word.to_string();
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        lines
    }

    fn button(
        &mut self,
        r: Rect,
        label: &str,
        action: Action,
        enabled: bool,
        style: Style,
        size: f32,
    ) -> bool {
        let s = self.scale;
        let size = size * s;
        let label_color = if enabled { TEXT } else { c(90, 100, 125) };
        let tw = self.text_width(label, size);
        // grow fixed-width buttons so the label is never clipped
        let r = rect(r.x, r.y, r.w.max(tw + self.u(16.0)), r.h * s);
        let hovered = enabled && r.contains(mouse_position().into());
        let fill = match style {
            Style::Primary => {
                if enabled {
                    c(20, 77, 92)
                } else {
                    c(30, 40, 60)
                }
            }
            Style::Danger => {
                if enabled {
                    PANEL2
                } else {
                    c(30, 40, 60)
                }
            }
            _ => {
                if hovered {
                    c(40, 54, 92)
                } else {
                    PANEL2
                }
            }
        };
        let border = match style {
            Style::Primary => {
                if enabled {
                    ACCENT
                } else {
                    BORDER
                }
            }
            Style::Danger => {
                if enabled {
                    BAD
                } else {
                    BORDER
                }
            }
            _ => {
                if hovered {
                    ACCENT
                } else {
                    BORDER
                }
            }
        };
        let fill = if !enabled { c(24, 32, 52) } else { fill };

        draw_rectangle(r.x, r.y, r.w, r.h, fill);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, s.max(1.0), border);

        let th = size;
        self.text(
            label,
            r.x + (r.w - tw) / 2.0,
            r.y + (r.h - th) / 2.0 + th * 0.75,
            size,
            label_color,
        );

        if enabled {
            self.clickables.push(Clickable { r, action });
        }
        hovered
    }

    fn hint(&self, s: &str, x: f32, y: f32, color: Color) {
        self.text(s, x, y, 13.0, color);
    }

    // ------------------------------------------------------------------
    // Input
    // ------------------------------------------------------------------
    fn handle_input(&mut self, sim: &mut Sim) {
        let mouse: Vec2 = mouse_position().into();
        let (map, side, _panel, _top) = self.layout();

        if is_mouse_button_pressed(MouseButton::Left) {
            self.drag = Some(Drag {
                start: mouse,
                last: mouse,
                moved: false,
            });
        } else if is_mouse_button_down(MouseButton::Left) {
            let mut update: Option<Vec2> = None;
            if let Some(d) = self.drag.as_mut() {
                let delta = mouse - d.last;
                if (mouse - d.start).length() > 4.0 {
                    d.moved = true;
                }
                if d.moved && map.contains(mouse) {
                    update = Some(delta);
                }
                d.last = mouse;
            }
            if let Some(delta) = update {
                self.cam_x -= delta.x / self.zoom;
                self.cam_y -= delta.y / self.zoom;
                self.clamp_cam();
            }
        }
        if is_mouse_button_released(MouseButton::Left) {
            if let Some(d) = self.drag.take() {
                if !d.moved {
                    self.handle_click(sim, mouse);
                }
            }
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            if self.modal.is_some() {
                self.modal = None;
            } else {
                if let Some(ctx) = sim.ctx() {
                    engine::select(ctx.st, None);
                }
            }
        }

        let wheel = mouse_wheel();
        if wheel.1 != 0.0 && self.modal.is_none() {
            if side.contains(mouse) {
                self.log_scroll = (self.log_scroll - wheel.1 * 3.0 * 8.0).max(0.0);
            } else if map.contains(mouse) {
                let mx = mouse.x - map.x;
                let my = mouse.y - map.y;
                let before = self.s2w(map, mx, my);
                self.zoom = (self.zoom * if wheel.1 > 0.0 { 1.12 } else { 0.89 }).clamp(0.12, 2.6);
                let after = self.s2w(map, mx, my);
                self.cam_x += before.x - after.x;
                self.cam_y += before.y - after.y;
                self.clamp_cam();
            }
        }

        let typed = get_char_pressed();
        let mut end_turn = false;
        if is_key_pressed(KeyCode::Escape) {
            if self.modal.is_some() {
                self.modal = None;
            } else if sim.has_state() {
                if let Some(ctx) = sim.ctx() {
                    engine::select(ctx.st, None);
                }
            }
        }
        if is_key_pressed(KeyCode::Enter) && self.modal.is_none() && sim.has_state() {
            end_turn = true;
        }
        if is_key_down(KeyCode::LeftControl) && is_key_pressed(KeyCode::S) && sim.has_state() {
            sim.save();
            self.toast("Game saved".into());
        }
        if end_turn {
            if let Some(st) = sim.state() {
                if !st.over {
                    sim.end_turn();
                }
            }
        }

        // text entry
        if self.modal == Some(Modal::Designer) && self.d_name_focus {
            if let Some(ch) = typed {
                if ch == '\u{8}' {
                    self.d_name.pop();
                } else if ch == '\r' || ch == '\n' {
                    self.d_name_focus = false;
                } else if ch.is_ascii_graphic() && self.d_name.len() < 28 {
                    self.d_name.push(ch);
                }
            }
            return;
        }
        if self.modal == Some(Modal::New) && self.name_focus {
            if let Some(ch) = typed {
                if ch == '\u{8}' {
                    self.name_input.pop();
                } else if ch == '\r' || ch == '\n' {
                    self.name_focus = false;
                } else if ch.is_ascii_graphic() && self.name_input.len() < 30 {
                    self.name_input.push(ch);
                }
            }
        }
    }

    fn handle_click(&mut self, sim: &mut Sim, pos: Vec2) {
        let start = if self.modal.is_some() {
            self.modal_click_start
        } else {
            0
        };
        let mut hit: Option<Action> = None;
        for c in self.clickables[start..].iter().rev() {
            if c.r.contains(pos) {
                hit = Some(c.action.clone());
                break;
            }
        }
        let action = match hit {
            Some(a) => a,
            None => {
                if self.modal.is_none() {
                    if let Some(ctx) = sim.ctx() {
                        engine::select(ctx.st, None);
                    }
                }
                return;
            }
        };
        self.do_action(sim, action, pos);
    }

    fn do_action(&mut self, sim: &mut Sim, action: Action, pos: Vec2) {
        match action {
            Action::SelectSystem(id) => {
                let mut move_issued = false;
                if let Some(st) = sim.state() {
                    if let Some(sel) = st.selected.clone() {
                        if sel.kind == "fleet" {
                            let (empire_id, from) = match st.fleet(sel.id) {
                                Some(f) => (f.empire_id, f.system_id),
                                None => (0, 0),
                            };
                            let target = st.system(id).cloned();
                            if empire_id == 0 {
                                if let Some(t) = target {
                                    if from != t.id {
                                        let ctx = sim.ctx().unwrap();
                                        if engine::issue_move(ctx.st, sel.id, t.id) {
                                            ctx.toasts.push(format!("Fleet moving to {}", t.name));
                                            move_issued = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !move_issued {
                    if let Some(ctx) = sim.ctx() {
                        engine::select(
                            ctx.st,
                            Some(crate::state::Selection {
                                kind: "system".into(),
                                id,
                            }),
                        );
                    }
                }
            }
            Action::SelectFleet(id) => {
                if let Some(ctx) = sim.ctx() {
                    engine::select(
                        ctx.st,
                        Some(crate::state::Selection {
                            kind: "fleet".into(),
                            id,
                        }),
                    );
                }
            }
            Action::Minimap => self.minimap_jump(pos),
            Action::CycleSize => {
                self.opt_size = match self.opt_size {
                    34 => 46,
                    46 => 60,
                    _ => 34,
                };
            }
            Action::CycleAi => {
                self.opt_ai = match self.opt_ai {
                    1 => 2,
                    2 => 3,
                    _ => 1,
                };
            }
            Action::CycleDiff => self.opt_diff = self.opt_diff.next(),
            Action::OpenNew => self.modal = Some(Modal::New),
            Action::BuildBuilding(key) => {
                if let Some(st) = sim.state() {
                    if let Some(sel) = st.selected.clone() {
                        if sel.kind == "system" {
                            if let Some(ctx) = sim.ctx() {
                                engine::build_building(ctx.st, sel.id, &key, ctx.toasts);
                            }
                        }
                    }
                }
            }
            Action::QueueShip(key) => {
                if let Some(st) = sim.state() {
                    if let Some(sel) = st.selected.clone() {
                        if sel.kind == "system" {
                            if let Some(ctx) = sim.ctx() {
                                engine::queue_ship(ctx.st, sel.id, &key, ctx.toasts);
                            }
                        }
                    }
                }
            }
            Action::SplitFleet => {
                if let Some(st) = sim.state() {
                    if let Some(sel) = st.selected.clone() {
                        if sel.kind == "fleet" {
                            if let Some(ctx) = sim.ctx() {
                                engine::split_fleet(ctx.st, sel.id);
                            }
                        }
                    }
                }
            }
            Action::StopFleet => {
                if let Some(st) = sim.state() {
                    if let Some(sel) = st.selected.clone() {
                        if sel.kind == "fleet" {
                            if let Some(ctx) = sim.ctx() {
                                engine::stop_fleet(ctx.st, sel.id);
                            }
                        }
                    }
                }
            }
            Action::MergeInto(a, b) => {
                if let Some(ctx) = sim.ctx() {
                    engine::merge_fleets(ctx.st, a, b);
                }
            }
            Action::MergeAll(id) => {
                if let Some(st) = sim.state() {
                    let (sys_id, emp) = match st.fleet(id) {
                        Some(f) => (f.system_id, f.empire_id),
                        None => return,
                    };
                    if let Some(ctx) = sim.ctx() {
                        engine::merge_all_fleets_at(ctx.st, sys_id, emp);
                    }
                }
            }
            Action::SetTech(key) => {
                if let Some(ctx) = sim.ctx() {
                    engine::set_research(ctx.st, 0, &key);
                }
            }
            Action::DeclareWar(id) => {
                if let Some(ctx) = sim.ctx() {
                    engine::declare_war(ctx.st, 0, id, ctx.toasts);
                }
            }
            Action::SuePeace(id) => {
                if let Some(ctx) = sim.ctx() {
                    engine::player_sue_peace(ctx.st, id, ctx.rng, ctx.toasts);
                }
            }
            Action::AcceptPeace(id) => {
                if let Some(ctx) = sim.ctx() {
                    engine::make_peace(ctx.st, 0, id);
                }
            }
            Action::Save => {
                sim.save();
                self.toast("Game saved".into());
            }
            Action::EndTurn => {
                sim.end_turn();
            }
            Action::FocusName => self.name_focus = true,
            Action::StartNewGame => self.start_new_game(sim),
            Action::LoadSave => {
                if sim.load() {
                    self.modal = None;
                    self.focus_home(sim);
                    self.toast("Saved game loaded".into());
                }
            }
            Action::CloseModal => self.modal = None,
            Action::OpenDesigner => {
                self.d_hull = data::hulls()[0].key.to_string();
                self.d_weapon = data::weapon_modules()[0].key.to_string();
                self.d_armor = data::armor_modules()[0].key.to_string();
                self.d_engine = data::engine_modules()[0].key.to_string();
                self.d_name.clear();
                self.d_name_focus = false;
                self.modal = Some(Modal::Designer);
            }
            Action::DesignNameFocus => self.d_name_focus = true,
            Action::DesignHull(k) => self.d_hull = k,
            Action::DesignWeapon(k) => self.d_weapon = k,
            Action::DesignArmor(k) => self.d_armor = k,
            Action::DesignEngine(k) => self.d_engine = k,
            Action::SaveDesign => {
                let name = if self.d_name.trim().is_empty() {
                    "Custom Design".to_string()
                } else {
                    self.d_name.trim().to_string()
                };
                if let Some(ctx) = sim.ctx() {
                    let key = engine::add_design(
                        ctx.st,
                        0,
                        &name,
                        &self.d_hull,
                        &self.d_weapon,
                        &self.d_armor,
                        &self.d_engine,
                    );
                    let dname = engine::ship_def(ctx.st, &key)
                        .map(|d| d.name)
                        .unwrap_or(name);
                    ctx.toasts.push(format!("Design saved: {}", dname));
                }
                self.modal = None;
            }
            Action::DeleteDesign(key) => {
                if let Some(ctx) = sim.ctx() {
                    engine::remove_design(ctx.st, &key);
                }
            }
            Action::Research => self.modal = Some(Modal::Research),
            Action::Diplomacy => self.modal = Some(Modal::Diplomacy),
            Action::Battles => self.modal = Some(Modal::Battles),
            Action::Help => self.modal = Some(Modal::Help),
            Action::Home => self.focus_home(sim),
            Action::New => self.modal = Some(Modal::New),
        }
    }

    fn minimap_jump(&mut self, pos: Vec2) {
        let (map, side, _panel, _top) = self.layout();
        let (mw, mh, sc, ox, oy) = self.minimap_geom();
        let r = rect(side.x + self.u(12.0), side.y + self.u(24.0), mw, mh);
        if !r.contains(pos) {
            return;
        }
        let mx = (pos.x - r.x) * (mw / r.w);
        let my = (pos.y - r.y) * (mh / r.h);
        self.cam_x = (mx - ox) / sc;
        self.cam_y = (my - oy) / sc;
        self.clamp_cam();
        let _ = map;
    }

    fn focus_home(&mut self, sim: &mut Sim) {
        if let Some(st) = sim.state() {
            if let Some(hid) = st.player().home_system_id {
                if let Some(home) = st.system(hid) {
                    self.cam_x = home.x;
                    self.cam_y = home.y;
                    self.zoom = 0.7;
                }
            }
        }
    }

    fn start_new_game(&mut self, sim: &mut Sim) {
        let name = if self.name_input.trim().is_empty() {
            "Terran Consortium".to_string()
        } else {
            self.name_input.trim().to_string()
        };
        sim.new_game(NewGameOpts {
            num_ai: self.opt_ai as usize,
            num_systems: self.opt_size as usize,
            difficulty: self.opt_diff.label().to_string(),
            player_name: name,
        });
        self.modal = None;
        self.focus_home(sim);
        self.toast("Welcome to Stellar Dominion".into());
    }

    fn toast(&mut self, msg: String) {
        self.toast = Some(msg);
        self.toast_until = get_time() + 2.2;
    }

    // ------------------------------------------------------------------
    // Frame
    // ------------------------------------------------------------------
    pub fn frame(&mut self, sim: &mut Sim) {
        self.scale = Self::compute_scale();
        for msg in sim.drain_toasts() {
            self.toast(msg);
        }
        self.handle_input(sim);
        self.draw(sim);
        if let Some(st) = sim.state() {
            if st.over && self.modal.is_none() {
                self.modal = Some(Modal::GameOver);
            }
        }
    }

    fn draw(&mut self, sim: &Sim) {
        let (map, side, panel, top) = self.layout();
        self.clickables.clear();
        clear_background(BG);

        let st = sim.state();
        if let Some(st) = st {
            self.draw_top_bar(st, top);
            self.draw_map(st, map);
            self.draw_sidebar(st, side);
            self.draw_panel(st, panel);
        }

        if self.modal.is_some() {
            self.modal_click_start = self.clickables.len();
            self.draw_modal(st);
        }

        self.draw_toast();
        if self.modal.is_none() && st.is_some() {
            self.draw_map_hint(map);
        }
    }

    fn draw_toast(&mut self) {
        let now = get_time();
        if let Some(msg) = self.toast.clone() {
            if now > self.toast_until {
                self.toast = None;
                return;
            }
            let tw = self.text_width(&msg, 14.0);
            let box_r = rect(
                screen_width() / 2.0 - tw / 2.0 - self.u(12.0),
                58.0,
                tw + self.u(24.0),
                26.0,
            );
            draw_rectangle(
                box_r.x,
                box_r.y,
                box_r.w,
                box_r.h,
                Color::from_rgba(14, 21, 38, 235),
            );
            draw_rectangle_lines(box_r.x, box_r.y, box_r.w, box_r.h, 1.0, ACCENT);
            self.text(
                &msg,
                box_r.x + self.u(12.0),
                box_r.y + self.u(18.0),
                14.0,
                TEXT,
            );
        }
    }

    fn draw_map_hint(&self, map: Rect) {
        let s = "Click a system or fleet to select | with a fleet selected, click a system to move | drag to pan | scroll to zoom | right-click to deselect";
        let tw = self.text_width(s, 12.0);
        let box_r = rect(
            screen_width() / 2.0 - tw / 2.0 - self.u(8.0),
            map.y + map.h - self.u(26.0),
            tw + self.u(16.0),
            18.0,
        );
        draw_rectangle(
            box_r.x,
            box_r.y,
            box_r.w,
            box_r.h,
            Color::from_rgba(8, 12, 24, 200),
        );
        self.text(
            s,
            box_r.x + self.u(8.0),
            box_r.y + self.u(13.0),
            12.0,
            MUTED,
        );
    }

    // ------------------------------------------------------------------
    // Top bar
    // ------------------------------------------------------------------
    fn draw_top_bar(&mut self, st: &State, top: Rect) {
        draw_rectangle(top.x, top.y, top.w, top.h, PANEL);
        // accent strip along the top edge, hairline border below
        draw_rectangle(top.x, top.y, top.w, self.u(2.0), ACCENT);
        draw_line(0.0, top.h - 1.0, top.w, top.h - 1.0, 1.0, BORDER);

        let mid = top.h * 0.5;
        let mut x = self.u(14.0);
        self.text("STELLAR DOMINION", x, mid + self.u(5.0), 18.0, ACCENT);
        x += self.text_width("STELLAR DOMINION", 18.0) + self.u(20.0);
        // thin separator between the title and the resource readout
        draw_line(
            x - self.u(11.0),
            mid - self.u(9.0),
            x - self.u(11.0),
            mid + self.u(9.0),
            1.0,
            BORDER,
        );

        let p = st.player();
        let inc = p.last_income.clone();
        let pool = p.research_pool as i32;
        let research = match &p.current_tech {
            Some(t) => format!(
                "{}/{}",
                p.tech_progress as i32,
                data::tech(t).map(|x| x.cost).unwrap_or(0)
            ),
            None => "-".to_string(),
        };
        let items = vec![
            format!("Turn {}", st.turn),
            format!(
                "Credits {}  +{}",
                p.credits as i32,
                inc.credits.round() as i32
            ),
            format!(
                "Minerals {}  +{}",
                p.minerals as i32,
                inc.minerals.round() as i32
            ),
            format!(
                "Research {}  +{}{}",
                research,
                inc.research.round() as i32,
                if pool > 0 {
                    format!("  ({} banked)", pool)
                } else {
                    String::new()
                }
            ),
        ];
        for s in items {
            self.text(&s, x, mid + self.u(4.0), 14.0, TEXT);
            x += self.text_width(&s, 14.0) + self.u(16.0);
        }

        let buttons: Vec<(&str, Action, Style)> = vec![
            ("Research", Action::Research, Style::Normal),
            ("Diplomacy", Action::Diplomacy, Style::Normal),
            ("Battles", Action::Battles, Style::Normal),
            ("Home", Action::Home, Style::Normal),
            ("Save", Action::Save, Style::Normal),
            ("New Game", Action::New, Style::Normal),
            ("Help", Action::Help, Style::Normal),
            ("End Turn", Action::EndTurn, Style::Primary),
        ];
        let widths: Vec<f32> = buttons
            .iter()
            .map(|(l, _, _)| self.text_width(l, 14.0) + self.u(22.0))
            .collect();
        let total: f32 = widths.iter().sum::<f32>() + self.u(8.0) * (buttons.len() as f32 - 1.0);
        let mut bx = (screen_width() - self.u(14.0) - total).max(x + self.u(10.0));
        for ((label, action, style), w) in buttons.iter().zip(widths.iter()) {
            self.button(
                rect(bx, mid - self.u(16.0), *w, 32.0),
                label,
                action.clone(),
                true,
                *style,
                14.0,
            );
            bx += *w + self.u(8.0);
        }
    }

    // ------------------------------------------------------------------
    // Map
    // ------------------------------------------------------------------
    fn draw_map(&mut self, st: &State, map: Rect) {
        draw_rectangle(map.x, map.y, map.w, map.h, c(4, 6, 12));
        let zoom_f = self.zoom.sqrt();

        // galactic band
        let band_c = self.w2s(map, galaxy::WIDTH / 2.0, galaxy::HEIGHT / 2.0);
        let band_w = 2400.0 * self.zoom;
        let band_h = band_w * 0.32;
        draw_rectangle(
            band_c.x - band_w / 2.0,
            band_c.y - band_h / 2.0,
            band_w.max(4.0),
            band_h.max(4.0),
            Color::from_rgba(28, 42, 84, 90),
        );

        // nebulae
        for n in &self.nebulae {
            let p = self.w2s(map, n.x, n.y);
            let target_r = n.r * self.zoom * 0.55;
            if target_r < 2.0 {
                continue;
            }
            if p.x + target_r < map.x
                || p.x - target_r > map.x + map.w
                || p.y + target_r < map.y
                || p.y - target_r > map.y + map.h
            {
                continue;
            }
            draw_circle(p.x, p.y, target_r, with_alpha(n.color, 26));
            draw_circle(p.x, p.y, target_r * 0.6, with_alpha(n.color, 22));
        }

        // starfield
        for s in &self.starfield {
            let p = self.w2s(map, s.x, s.y);
            if p.x < map.x || p.x > map.x + map.w || p.y < map.y || p.y > map.y + map.h {
                continue;
            }
            let b = (200.0 * s.a) as u8;
            draw_circle(
                p.x,
                p.y,
                s.r.max(1.0),
                Color::from_rgba(b, b, b.saturating_add(30), 255),
            );
        }

        // move lines
        for f in &st.fleets {
            let m = match &f.moves {
                Some(m) => m,
                None => continue,
            };
            if !self.fleet_visible(st, f) {
                continue;
            }
            let a = self.w2s(map, f.x, f.y);
            let to = match st.system(m.to_system_id) {
                Some(s) => s,
                None => continue,
            };
            let b = self.w2s(map, to.x, to.y);
            let col = st
                .empire(f.empire_id)
                .map(|e| rgb(&e.color))
                .unwrap_or(TEXT);
            self.dashed_line(a, b, col);
        }

        for s in &st.systems {
            self.draw_system(st, map, s, zoom_f);
        }
        for f in &st.fleets {
            if self.fleet_visible(st, f) {
                self.draw_fleet(st, map, f, zoom_f);
            }
        }
        self.draw_selection(st, map);
    }

    fn dashed_line(&self, a: Vec2, b: Vec2, color: Color) {
        let length = (b - a).length();
        if length < 1.0 {
            return;
        }
        let steps = (length / 8.0) as i32;
        let mut i = 0;
        while i < steps {
            let t0 = i as f32 / steps as f32;
            let t1 = ((i + 1) as f32 / steps as f32).min(1.0);
            draw_line(
                a.x + (b.x - a.x) * t0,
                a.y + (b.y - a.y) * t0,
                a.x + (b.x - a.x) * t1,
                a.y + (b.y - a.y) * t1,
                1.0,
                color,
            );
            i += 2;
        }
    }

    fn dashed_circle(&self, center: Vec2, radius: f32, color: Color) {
        let steps = 40;
        let mut i = 0;
        while i < steps {
            let a0 = (i as f32 / steps as f32) * std::f32::consts::TAU;
            let a1 = (((i + 1) as f32) / steps as f32) * std::f32::consts::TAU;
            draw_line(
                center.x + radius * a0.cos(),
                center.y + radius * a0.sin(),
                center.x + radius * a1.cos(),
                center.y + radius * a1.sin(),
                1.0,
                color,
            );
            i += 2;
        }
    }

    fn fleet_visible(&self, st: &State, f: &crate::state::Fleet) -> bool {
        st.system(f.system_id)
            .map(|s| s.explored_by.contains(&0))
            .unwrap_or(false)
    }

    fn draw_system(&mut self, st: &State, map: Rect, s: &crate::state::System, zoom_f: f32) {
        let p = self.w2s(map, s.x, s.y);
        if p.x < map.x - self.u(60.0)
            || p.x > map.x + map.w + self.u(60.0)
            || p.y < map.y - self.u(60.0)
            || p.y > map.y + map.h + self.u(60.0)
        {
            return;
        }
        let explored = s.explored_by.contains(&0);
        let r = (7.0 * s.star_radius * zoom_f).max(4.5);

        let base = rgb(&s.star_color);
        // glow
        let gr = r * 2.4;
        draw_circle(p.x, p.y, gr, with_alpha(base, 26));
        draw_circle(p.x, p.y, gr * 0.7, with_alpha(base, 30));

        let body = if explored { base } else { c(90, 100, 120) };
        draw_circle(p.x, p.y, r, body);

        if explored {
            if let Some(owner) = s.owner_empire_id {
                if let Some(e) = st.empire(owner) {
                    let col = rgb(&e.color);
                    draw_circle(p.x, p.y, r + self.u(4.0), with_alpha(col, 255));
                    draw_circle(p.x, p.y, r + 2.0, body);
                }
            }
        } else {
            draw_circle(
                p.x,
                p.y,
                r + self.u(3.0),
                Color::from_rgba(125, 141, 176, 255),
            );
            draw_circle(p.x, p.y, r + 1.5, body);
        }

        let label = if explored {
            s.name.clone()
        } else {
            "Unknown".to_string()
        };
        let col = if explored { TEXT } else { MUTED };
        let lw = self.text_width(&label, 13.0);
        self.text(
            &label,
            p.x - lw / 2.0,
            p.y + r + self.u(15.0) * zoom_f + 2.0,
            13.0,
            col,
        );

        let hit = rect(
            p.x + map.x - (r + self.u(8.0)),
            p.y + map.y - (r + self.u(8.0)),
            (r + self.u(8.0)) * 2.0,
            (r + self.u(8.0)) * 2.0,
        );
        self.clickables.push(Clickable {
            r: hit,
            action: Action::SelectSystem(s.id),
        });
    }

    fn fleet_draw_pos(&self, st: &State, f: &crate::state::Fleet) -> Vec2 {
        if f.moves.is_some() {
            return vec2(f.x, f.y);
        }
        let sys = match st.system(f.system_id) {
            Some(s) => s,
            None => return vec2(f.x, f.y),
        };
        let mut idle: Vec<&crate::state::Fleet> = st
            .fleets
            .iter()
            .filter(|x| x.system_id == sys.id && x.moves.is_none())
            .collect();
        idle.sort_by_key(|x| x.id);
        let idx = idle.iter().position(|x| x.id == f.id).unwrap_or(0);
        let angle = -std::f32::consts::FRAC_PI_2 + (idx % 8) as f32 * (std::f32::consts::TAU / 8.0);
        vec2(sys.x + angle.cos() * 46.0, sys.y + angle.sin() * 46.0)
    }

    fn draw_fleet(&mut self, st: &State, map: Rect, f: &crate::state::Fleet, zoom_f: f32) {
        let pos = self.fleet_draw_pos(st, f);
        let p = self.w2s(map, pos.x, pos.y);
        let e = match st.empire(f.empire_id) {
            Some(e) => e,
            None => return,
        };
        let color = rgb(&e.color);
        let fs = (9.0 * zoom_f).max(7.0);
        let dest = f.moves.as_ref().and_then(|m| st.system(m.to_system_id));
        let angle = match dest {
            Some(d) => (d.y - f.y).atan2(d.x - f.x),
            None => -std::f32::consts::FRAC_PI_2,
        };
        draw_poly(p.x, p.y, 4, fs, angle + std::f32::consts::FRAC_PI_2, color);
        draw_poly(
            p.x,
            p.y,
            4,
            fs * 0.55,
            angle + std::f32::consts::FRAC_PI_2,
            BLACK,
        );

        let cnt = format!("{}", f.ships.len());
        self.text(&cnt, p.x + fs + 2.0, p.y - fs, 13.0, c(235, 242, 255));

        let hit = rect(
            p.x + map.x - self.u(12.0),
            p.y + map.y - self.u(12.0),
            24.0,
            24.0,
        );
        self.clickables.push(Clickable {
            r: hit,
            action: Action::SelectFleet(f.id),
        });
    }

    fn draw_selection(&self, st: &State, map: Rect) {
        let sel = match &st.selected {
            Some(s) => s,
            None => return,
        };
        if sel.kind == "system" {
            if let Some(s) = st.system(sel.id) {
                let p = self.w2s(map, s.x, s.y);
                let r = (9.0 * s.star_radius * self.zoom.sqrt()).max(6.0) + self.u(8.0);
                self.dashed_circle(p, r, WHITE);
            }
        } else if let Some(f) = st.fleet(sel.id) {
            let pos = self.fleet_draw_pos(st, f);
            let p = self.w2s(map, pos.x, pos.y);
            self.dashed_circle(p, 12.0, WHITE);
        }
    }

    // ------------------------------------------------------------------
    // Sidebar
    // ------------------------------------------------------------------
    fn draw_sidebar(&mut self, st: &State, side: Rect) {
        draw_rectangle(side.x, side.y, side.w, side.h, PANEL);
        draw_line(side.x, side.y, side.x, side.y + side.h, 1.0, BORDER);

        let mut y = side.y + self.u(6.0);
        self.text(
            "GALAXY",
            side.x + self.u(20.0),
            y + self.u(9.0),
            13.0,
            ACCENT2,
        );
        draw_rectangle(
            side.x + self.u(12.0),
            y + self.u(3.0),
            self.u(3.0),
            self.u(13.0),
            ACCENT2,
        );
        y += 18.0;

        let mm = rect(
            side.x + self.u(12.0),
            y,
            MINIMAP_W * self.scale,
            MINIMAP_H * self.scale,
        );
        self.draw_minimap(st, mm);
        let hit = rect(mm.x - 1.0, mm.y - 1.0, mm.w + 2.0, mm.h + 2.0);
        self.clickables.push(Clickable {
            r: hit,
            action: Action::Minimap,
        });
        y += MINIMAP_H * self.scale + self.u(10.0);

        self.text(
            "EVENT LOG",
            side.x + self.u(20.0),
            y + self.u(9.0),
            13.0,
            ACCENT2,
        );
        draw_rectangle(
            side.x + self.u(12.0),
            y + self.u(3.0),
            self.u(3.0),
            self.u(13.0),
            ACCENT2,
        );
        y += 18.0;

        let log_rect = rect(
            side.x + self.u(6.0),
            y,
            side.w - self.u(12.0),
            side.y + side.h - y - self.u(6.0),
        );
        draw_rectangle(log_rect.x, log_rect.y, log_rect.w, log_rect.h, BG2);

        let line_h = self.u(17.0);
        let avail_w = log_rect.w - self.u(46.0);
        let mut display: Vec<(String, String, Color)> = Vec::new();
        for entry in st.logs.iter().rev().take(120).rev() {
            let col = match entry.kind.as_str() {
                "good" => GOOD,
                "bad" => BAD,
                "war" => WAR,
                "research" => RESEARCH,
                "build" => GOLD,
                _ => TEXT,
            };
            let tag = format!("[{}]", entry.turn);
            for (j, line) in self.wrap(&entry.msg, 12.0, avail_w).iter().enumerate() {
                display.push((
                    if j == 0 { tag.clone() } else { String::new() },
                    line.clone(),
                    col,
                ));
            }
        }
        let max_scroll = ((display.len() as f32 * line_h) - log_rect.h).max(0.0);
        self.log_scroll = self.log_scroll.clamp(0.0, max_scroll);
        let mut i = 0;
        while i < display.len() {
            let yy = i as f32 * line_h - self.log_scroll;
            if yy >= -line_h && yy <= log_rect.h {
                let (tag, line, col) = &display[i];
                if !tag.is_empty() {
                    self.text(
                        tag,
                        log_rect.x + 2.0,
                        log_rect.y + yy + self.u(10.0),
                        12.0,
                        c(84, 98, 138),
                    );
                }
                self.text(
                    line,
                    log_rect.x + self.u(40.0),
                    log_rect.y + yy + self.u(10.0),
                    12.0,
                    *col,
                );
            }
            i += 1;
        }
    }

    fn draw_minimap(&self, st: &State, mm: Rect) {
        let sc =
            ((mm.w - self.u(16.0)) / galaxy::WIDTH).min((mm.h - self.u(12.0)) / galaxy::HEIGHT);
        let ox = (mm.w - galaxy::WIDTH * sc) / 2.0;
        let oy = (mm.h - galaxy::HEIGHT * sc) / 2.0;
        draw_rectangle(mm.x, mm.y, mm.w, mm.h, c(7, 11, 22));
        for s in &st.systems {
            let x = mm.x + ox + s.x * sc;
            let y = mm.y + oy + s.y * sc;
            let explored = s.explored_by.contains(&0);
            let col = if explored {
                match s.owner_empire_id {
                    Some(o) => st
                        .empire(o)
                        .map(|e| rgb(&e.color))
                        .unwrap_or(c(141, 156, 196)),
                    None => c(141, 156, 196),
                }
            } else {
                c(58, 69, 96)
            };
            draw_circle(x, y, self.u(2.0), col);
        }
        for f in &st.fleets {
            if !self.fleet_visible(st, f) {
                continue;
            }
            let pos = self.fleet_draw_pos(st, f);
            let x = mm.x + ox + pos.x * sc;
            let y = mm.y + oy + pos.y * sc;
            let col = st
                .empire(f.empire_id)
                .map(|e| rgb(&e.color))
                .unwrap_or(TEXT);
            draw_rectangle(
                x - self.u(1.0),
                y - self.u(1.0),
                self.u(3.0),
                self.u(3.0),
                col,
            );
        }
        let (map, _side, _panel, _top) = self.layout();
        let tl = self.s2w(map, 0.0, 0.0);
        let br = self.s2w(map, map.w, map.h);
        let vx = mm.x + ox + tl.x * sc;
        let vy = mm.y + oy + tl.y * sc;
        let vw = (br.x - tl.x) * sc;
        let vh = (br.y - tl.y) * sc;
        draw_rectangle_lines(vx, vy, vw, vh, self.scale.max(1.0), ACCENT);
    }

    // ------------------------------------------------------------------
    // Bottom panel
    // ------------------------------------------------------------------
    fn draw_panel(&mut self, st: &State, panel: Rect) {
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, PANEL);
        draw_line(0.0, panel.y, panel.w, panel.y, 1.0, BORDER);

        let x0 = panel.x + self.u(14.0);
        let y0 = panel.y + self.u(10.0);
        let w = panel.w - self.u(28.0);

        let sel = st.selected.clone();
        match sel {
            None => self.panel_overview(st, x0, y0, w),
            Some(s) if s.kind == "system" => {
                if let Some(sys) = st.system(s.id) {
                    let sys = sys.clone();
                    self.panel_system(st, &sys, x0, y0, w);
                }
            }
            Some(s) => {
                if let Some(f) = st.fleet(s.id) {
                    let f = f.clone();
                    self.panel_fleet(st, &f, x0, y0, w);
                }
            }
        }
    }

    fn panel_overview(&mut self, st: &State, x: f32, y: f32, _w: f32) {
        let p = st.player();
        self.text(&p.name, x, y + self.u(12.0), 18.0, ACCENT);
        self.text(&p.desc, x, y + self.u(34.0), 13.0, MUTED);
        let stats: Vec<(&str, String)> = vec![
            ("Colonies", format!("{}", st.empire_colonies(0).len())),
            (
                "Systems controlled",
                format!("{}", st.empire_systems(0).len()),
            ),
            ("Fleets", format!("{}", st.empire_fleets(0).len())),
            ("Ships", format!("{}", engine::empire_ship_count(st, 0))),
            (
                "Fleet strength",
                format!("{}", engine::empire_strength(st, 0).round() as i32),
            ),
            (
                "Research",
                p.current_tech
                    .as_ref()
                    .and_then(|t| data::tech(t))
                    .map(|t| t.name.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
        ];
        let mut sx = x;
        let sy = y + self.u(56.0);
        for (label, val) in stats {
            self.text(label, sx, sy + self.u(10.0), 12.0, MUTED);
            self.text(&val, sx, sy + self.u(23.0), 14.0, TEXT);
            sx += 150.0;
        }
        self.hint(
            "Select a system to inspect it, or a fleet to command it.  Press Enter to end the turn.",
            x,
            y + self.u(94.0),
            MUTED,
        );
    }

    fn panel_system(&mut self, st: &State, s: &crate::state::System, x: f32, y: f32, _w: f32) {
        let explored = s.explored_by.contains(&0);
        let star_name = data::star_by_key(&s.star_class)
            .map(|s| s.name)
            .unwrap_or("");
        self.text(&s.name, x, y + self.u(12.0), 18.0, ACCENT);

        if !explored {
            self.text("Unexplored system", x, y + self.u(34.0), 13.0, MUTED);
            self.hint(
                "Send a fleet within sensor range to reveal this system's planets and owner.",
                x,
                y + self.u(56.0),
                MUTED,
            );
            return;
        }

        let pt = data::planet_type(&s.planet.kind);
        let owner = s.owner_empire_id.and_then(|o| st.empire(o)).cloned();
        self.text(
            &format!("{} | {} world", star_name, pt.map(|p| p.name).unwrap_or("")),
            x,
            y + self.u(34.0),
            13.0,
            MUTED,
        );

        let size_name = ["Small", "Medium", "Large"][(s.planet.size.max(1) - 1).min(2) as usize];
        let stats: Vec<(&str, String, Color)> = vec![
            (
                "Planet",
                pt.map(|p| p.name).unwrap_or("").to_string(),
                pt.map(|p| rgb(p.color)).unwrap_or(TEXT),
            ),
            ("Size", size_name.to_string(), TEXT),
            ("Richness", format!("{:.2}x", s.planet.richness), TEXT),
            ("Max population", format!("{}", s.planet.max_pop), TEXT),
            (
                "Owner",
                owner
                    .as_ref()
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unclaimed".to_string()),
                owner.as_ref().map(|o| rgb(&o.color)).unwrap_or(MUTED),
            ),
        ];
        let mut sx = x;
        let sy = y + self.u(54.0);
        for (label, val, col) in stats {
            self.text(label, sx, sy + self.u(10.0), 12.0, MUTED);
            self.text(&val, sx, sy + self.u(23.0), 14.0, col);
            sx += 140.0;
        }

        let mut ly = y + self.u(94.0);
        if let Some(a) = &s.anomaly {
            self.text(
                &format!("Anomaly - {}: {}", a.name, a.desc),
                x,
                ly + self.u(11.0),
                13.0,
                GOLD,
            );
            ly += 18.0;
        }

        let col = s.planet.colony.clone();
        if let Some(col) = col {
            let c = st.empire(col.empire_id).cloned();
            if let Some(c) = c {
                let prod = engine::colony_production(s, &col);
                self.text(
                    &format!("COLONY - {}", c.name),
                    x,
                    ly + self.u(11.0),
                    13.0,
                    MUTED,
                );
                self.text(
                    &format!(
                        "Population {} / {}",
                        col.population as i32,
                        engine::max_pop(s, Some(&c))
                    ),
                    x + self.u(230.0),
                    ly + self.u(11.0),
                    13.0,
                    TEXT,
                );
                self.text(
                    &format!(
                        "Income +{:.1} min | +{:.1} cr | +{:.1} rp",
                        prod.minerals, prod.credits, prod.research
                    ),
                    x + self.u(400.0),
                    ly + self.u(11.0),
                    13.0,
                    TEXT,
                );
                ly += 20.0;
                let mut bx = x;
                for b in &col.buildings {
                    let label = data::building(b).map(|x| x.name).unwrap_or(b);
                    let bw = self.text_width(label, 13.0) + self.u(14.0);
                    let r = rect(bx, ly, bw, 20.0);
                    draw_rectangle(r.x, r.y, r.w, r.h, PANEL2);
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, BORDER);
                    self.text(label, bx + self.u(7.0), ly + self.u(14.0), 13.0, TEXT);
                    bx += bw + self.u(5.0);
                }
                ly += 26.0;
                if !col.queue.is_empty() {
                    self.text("Construction", x, ly + self.u(11.0), 13.0, MUTED);
                    ly += 15.0;
                    for (i, key) in col.queue.iter().enumerate() {
                        let ship = engine::ship_def(st, key);
                        let name = ship
                            .as_ref()
                            .map(|d| d.name.clone())
                            .unwrap_or_else(|| key.clone());
                        let prog = if i == 0 {
                            format!(
                                " ({}/{})",
                                col.queue_progress,
                                ship.map(|d| d.build_turns).unwrap_or(0)
                            )
                        } else {
                            String::new()
                        };
                        self.text(
                            &format!("{}{}", name, prog),
                            x,
                            ly + self.u(11.0),
                            13.0,
                            if i == 0 { GOLD } else { MUTED },
                        );
                        ly += 14.0;
                    }
                }
            }
        } else {
            let p = st.player();
            if engine::can_colonize(p, s) {
                self.hint(
                    "Uncolonized. Send a Colony Ship to found a colony here.",
                    x,
                    ly + self.u(11.0),
                    MUTED,
                );
            } else if pt.map(|p| p.habitability <= 0.0).unwrap_or(true) {
                self.hint(
                    "This world cannot support a colony.",
                    x,
                    ly + self.u(11.0),
                    MUTED,
                );
            } else {
                self.hint(
                    "This hostile world requires Xeno Colonization research.",
                    x,
                    ly + self.u(11.0),
                    MUTED,
                );
            }
        }

        // Build options for the player's own colonies
        if let Some(col) = &s.planet.colony {
            let c = match st.empire(col.empire_id) {
                Some(c) => c.clone(),
                None => return,
            };
            if !c.is_player {
                return;
            }
            let bx0 = x + self.u(760.0);
            let mut by = y + self.u(54.0);
            self.text("BUILDINGS", bx0, by + self.u(10.0), 13.0, MUTED);
            by += 17.0;
            let mut bx = bx0;
            for b in data::buildings() {
                if !b.repeat && col.buildings.iter().any(|x| x == b.key) {
                    continue;
                }
                if col.buildings.len() as i32 >= engine::slots(s) {
                    continue;
                }
                let label = format!("{} {}min", b.name, b.cost);
                let bw = self.text_width(&label, 13.0) + self.u(14.0);
                let enabled = c.minerals >= b.cost as f32;
                self.button(
                    rect(bx, by, bw, 22.0),
                    &label,
                    Action::BuildBuilding(b.key.to_string()),
                    enabled,
                    Style::Small,
                    13.0,
                );
                bx += bw + self.u(5.0);
            }
            by += 28.0;
            if col.buildings.iter().any(|b| b == "shipyard") {
                self.text("SHIPS", bx0, by + self.u(10.0), 13.0, MUTED);
                self.button(
                    rect(bx0 + self.u(78.0), by - self.u(3.0), self.u(118.0), 18.0),
                    "+ Design a Ship",
                    Action::OpenDesigner,
                    true,
                    Style::Small,
                    12.0,
                );
                by += 17.0;
                let mut bx = bx0;
                let mut items: Vec<(String, String)> = data::ship_classes()
                    .iter()
                    .map(|k| (k.key.to_string(), k.name.to_string()))
                    .collect();
                for key in engine::list_designs(st, 0) {
                    let name = engine::ship_def(st, &key)
                        .map(|d| d.name)
                        .unwrap_or_else(|| key.clone());
                    items.push((key, name));
                }
                for (key, name) in items {
                    let ship = match engine::ship_def(st, &key) {
                        Some(d) => d,
                        None => continue,
                    };
                    if let Some(t) = &ship.tech {
                        if !c.techs.iter().any(|x| x == t) {
                            continue;
                        }
                    }
                    let (credits, minerals) = engine::ship_cost(st, &c, &key);
                    let tag = if data::ship_class(&key).is_some() {
                        ""
                    } else {
                        "* "
                    };
                    let label = format!(
                        "{}{} {}cr {}min {}t",
                        tag, name, credits, minerals, ship.build_turns
                    );
                    let bw = self.text_width(&label, 13.0) + self.u(14.0);
                    let enabled = c.credits >= credits as f32 && c.minerals >= minerals as f32;
                    self.button(
                        rect(bx, by, bw, 22.0),
                        &label,
                        Action::QueueShip(key),
                        enabled,
                        Style::Small,
                        13.0,
                    );
                    bx += bw + self.u(5.0);
                }
            }
        }
    }

    fn panel_fleet(&mut self, st: &State, f: &crate::state::Fleet, x: f32, y: f32, w: f32) {
        let e = match st.empire(f.empire_id) {
            Some(e) => e.clone(),
            None => return,
        };
        let sys = st.system(f.system_id).cloned();
        self.text(
            &format!("Fleet - {}", e.name),
            x,
            y + self.u(12.0),
            18.0,
            ACCENT,
        );
        let mut sub = format!(
            "At {}",
            sys.as_ref().map(|s| s.name.clone()).unwrap_or_default()
        );
        if let Some(m) = &f.moves {
            if let Some(dest) = st.system(m.to_system_id) {
                sub.push_str(&format!(
                    "  ->  {} ({} turn{})",
                    dest.name,
                    m.turns_left,
                    if m.turns_left == 1 { "" } else { "s" }
                ));
            }
        }
        sub.push_str(&format!(
            "   |   Speed {}   |   Strength {}",
            engine::fleet_speed(st, f) as i32,
            engine::fleet_strength(st, f).round() as i32
        ));
        self.text(&sub, x, y + self.u(34.0), 13.0, MUTED);

        self.text(
            &format!("Ships ({})", f.ships.len()),
            x,
            y + self.u(54.0),
            13.0,
            MUTED,
        );
        let mut bx = x;
        let mut by = y + self.u(70.0);
        for ship in &f.ships {
            let stt = engine::effective_ship_stats(st, &e, &ship.cls);
            let label = stt.name.clone();
            let bw = self.text_width(&label, 13.0) + self.u(14.0) + 60.0;
            let r = rect(bx, by, bw, 20.0);
            draw_rectangle(r.x, r.y, r.w, r.h, PANEL2);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, BORDER);
            self.text(&label, bx + self.u(7.0), by + self.u(14.0), 13.0, TEXT);
            let pct = ((100.0 * ship.hp / stt.hp).clamp(0.0, 100.0)) as i32;
            draw_rectangle(
                bx + bw - self.u(58.0),
                by + self.u(6.0),
                50.0,
                8.0,
                c(34, 42, 61),
            );
            draw_rectangle(
                bx + bw - self.u(58.0),
                by + self.u(6.0),
                50.0 * pct as f32 / 100.0,
                8.0,
                GOOD,
            );
            bx += bw + self.u(5.0);
            if bx > x + w - bw {
                bx = x;
                by += 24.0;
            }
        }
        by += 30.0;

        if !e.is_player {
            let war = engine::at_war(st, e.id, 0);
            self.hint(
                if war {
                    "Hostile fleet. Engage it by moving a fleet into its system."
                } else {
                    "You are not at war with this empire."
                },
                x,
                by + self.u(11.0),
                MUTED,
            );
            return;
        }

        let others: Vec<crate::state::Fleet> = st
            .fleets
            .iter()
            .filter(|o| {
                o.id != f.id
                    && o.empire_id == e.id
                    && o.moves.is_none()
                    && o.system_id == f.system_id
            })
            .cloned()
            .collect();

        self.button(
            rect(x, by, 96.0, 24.0),
            "Split Fleet",
            Action::SplitFleet,
            f.ships.len() >= 2,
            Style::Small,
            14.0,
        );
        self.button(
            rect(x + self.u(102.0), by, self.u(60.0), 24.0),
            "Stop",
            Action::StopFleet,
            f.moves.is_some(),
            Style::Small,
            14.0,
        );
        if !others.is_empty() {
            self.button(
                rect(x + self.u(168.0), by, self.u(150.0), 24.0),
                &format!("Merge All Here ({})", others.len() + 1),
                Action::MergeAll(f.id),
                true,
                Style::Small,
                14.0,
            );
        }
        self.hint(
            "Click a system on the map to move this fleet there.",
            x + self.u(340.0),
            by + self.u(16.0),
            MUTED,
        );
        by += 32.0;
        if !others.is_empty() {
            self.text("Other fleets here", x, by + self.u(10.0), 13.0, MUTED);
            by += 16.0;
            for o in &others {
                let names: Vec<String> = o
                    .ships
                    .iter()
                    .map(|s| {
                        engine::ship_def(st, &s.cls)
                            .map(|d| d.name)
                            .unwrap_or_default()
                    })
                    .collect();
                self.text(&names.join(", "), x, by + self.u(11.0), 13.0, TEXT);
                self.button(
                    rect(x + self.u(260.0), by - self.u(2.0), self.u(60.0), 20.0),
                    "Merge",
                    Action::MergeInto(f.id, o.id),
                    true,
                    Style::Small,
                    13.0,
                );
                by += 20.0;
            }
        }
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Modals
// ---------------------------------------------------------------------------
impl Ui {
    fn modal_box(&self, w: f32, h: f32, bw: f32, bh: f32) -> Rect {
        let s = self.scale;
        let bw = (bw * s).min(w * 0.94);
        let bh = (bh * s).min(h * 0.94);
        let r = rect((w - bw) / 2.0, (h - bh) / 2.0, bw, bh);
        draw_rectangle(r.x, r.y, r.w, r.h, PANEL);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, s.max(1.0), BORDER);
        r
    }

    fn close_button(&mut self, r: Rect) {
        self.button(
            rect(
                right(r) - self.u(78.0),
                bottom(r) - self.u(42.0),
                self.u(66.0),
                26.0,
            ),
            "Close",
            Action::CloseModal,
            true,
            Style::Small,
            14.0,
        );
    }

    fn draw_modal(&mut self, st: Option<&State>) {
        let w = screen_width();
        let h = screen_height();
        draw_rectangle(0.0, 0.0, w, h, Color::from_rgba(3, 5, 12, 200));
        match self.modal {
            Some(Modal::GameOver) => self.modal_gameover(st, w, h),
            Some(Modal::New) => self.modal_new(w, h),
            Some(Modal::Research) => {
                if let Some(st) = st {
                    self.modal_research(st, w, h);
                }
            }
            Some(Modal::Diplomacy) => {
                if let Some(st) = st {
                    self.modal_diplomacy(st, w, h);
                }
            }
            Some(Modal::Battles) => {
                if let Some(st) = st {
                    self.modal_battles(st, w, h);
                }
            }
            Some(Modal::Help) => self.modal_help(w, h),
            Some(Modal::Designer) => {
                if let Some(st) = st {
                    self.modal_designer(st, w, h);
                }
            }
            None => {}
        }
    }

    fn modal_new(&mut self, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 460.0, 420.0);
        self.text(
            "Stellar Dominion",
            box_r.x + self.u(20.0),
            box_r.y + self.u(30.0),
            18.0,
            ACCENT,
        );
        let body = "A turn-based 4X space strategy game. Explore, expand, exploit and exterminate across a procedurally generated galaxy.";
        let mut yy = box_r.y + self.u(58.0);
        for line in self.wrap(body, 14.0, box_r.w - self.u(40.0)) {
            self.text(&line, box_r.x + self.u(20.0), yy, 14.0, TEXT);
            yy += 16.0;
        }
        yy += 10.0;

        self.text(
            "Empire name",
            box_r.x + self.u(20.0),
            yy + self.u(11.0),
            13.0,
            MUTED,
        );
        yy += 18.0;
        let r = rect(box_r.x + self.u(20.0), yy, box_r.w - self.u(40.0), 26.0);
        draw_rectangle(r.x, r.y, r.w, r.h, PANEL2);
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            1.0,
            if self.name_focus { ACCENT } else { BORDER },
        );
        self.text(
            &self.name_input,
            r.x + self.u(8.0),
            r.y + self.u(18.0),
            14.0,
            TEXT,
        );
        self.clickables.push(Clickable {
            r,
            action: Action::FocusName,
        });
        yy += 36.0;

        let rows: Vec<(&str, String, Action)> = vec![
            (
                "Galaxy size",
                format!("{}", self.opt_size),
                Action::CycleSize,
            ),
            ("Rivals", format!("{}", self.opt_ai), Action::CycleAi),
            (
                "Difficulty",
                self.opt_diff.label().to_string(),
                Action::CycleDiff,
            ),
        ];
        for (label, val, act) in rows {
            self.text(
                label,
                box_r.x + self.u(20.0),
                yy + self.u(11.0),
                13.0,
                MUTED,
            );
            self.button(
                rect(
                    box_r.x + self.u(160.0),
                    yy - self.u(3.0),
                    self.u(120.0),
                    24.0,
                ),
                &val,
                act,
                true,
                Style::Small,
                14.0,
            );
            yy += 30.0;
        }

        if engine::has_save() {
            self.button(
                rect(
                    box_r.x + self.u(20.0),
                    bottom(box_r) - self.u(48.0),
                    self.u(130.0),
                    28.0,
                ),
                "Load Saved Game",
                Action::LoadSave,
                true,
                Style::Small,
                14.0,
            );
        }
        self.button(
            rect(
                right(box_r) - self.u(150.0),
                bottom(box_r) - self.u(50.0),
                self.u(130.0),
                30.0,
            ),
            "Start Game",
            Action::StartNewGame,
            true,
            Style::Primary,
            14.0,
        );
    }

    fn modal_research(&mut self, st: &State, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 680.0, 560.0);
        self.text(
            "Research",
            box_r.x + self.u(20.0),
            box_r.y + self.u(30.0),
            18.0,
            ACCENT,
        );
        let mut intro = "Research points accumulate each turn from your colonies.".to_string();
        if st.player().research_pool > 0.0 {
            intro.push_str(&format!(
                "  {} RP banked.",
                st.player().research_pool as i32
            ));
        }
        self.text(
            &intro,
            box_r.x + self.u(20.0),
            box_r.y + self.u(54.0),
            13.0,
            MUTED,
        );

        let p = st.player().clone();
        let mut y = box_r.y + self.u(76.0);
        for cat in data::TECH_CATS {
            self.text(
                &cat.to_uppercase(),
                box_r.x + self.u(20.0),
                y + self.u(10.0),
                13.0,
                MUTED,
            );
            y += 18.0;
            for t in data::techs() {
                if t.cat != cat {
                    continue;
                }
                let done = p.techs.iter().any(|x| x == t.key);
                let current = p.current_tech.as_deref() == Some(t.key);
                let locked = !t.requires.iter().all(|r| p.techs.iter().any(|x| x == r));
                let col = if done {
                    GOOD
                } else if current {
                    RESEARCH
                } else if locked {
                    MUTED
                } else {
                    TEXT
                };
                self.text(t.name, box_r.x + self.u(28.0), y + self.u(12.0), 14.0, col);
                let mut desc = t.desc.to_string();
                if !t.requires.is_empty() && !locked {
                    let names: Vec<&str> = t
                        .requires
                        .iter()
                        .filter_map(|r| data::tech(r))
                        .map(|x| x.name)
                        .collect();
                    desc.push_str(&format!("  (requires {})", names.join(", ")));
                }
                self.text(
                    &desc,
                    box_r.x + self.u(230.0),
                    y + self.u(13.0),
                    12.0,
                    MUTED,
                );
                let status = if done {
                    "Researched".to_string()
                } else if current {
                    "In progress".to_string()
                } else if locked {
                    "Locked".to_string()
                } else {
                    format!("{} RP", t.cost)
                };
                self.text(
                    &status,
                    box_r.x + self.u(470.0),
                    y + self.u(12.0),
                    13.0,
                    col,
                );
                if !done && !current && !locked {
                    self.button(
                        rect(
                            right(box_r) - self.u(120.0),
                            y - self.u(3.0),
                            self.u(96.0),
                            22.0,
                        ),
                        "Research",
                        Action::SetTech(t.key.to_string()),
                        true,
                        Style::Small,
                        13.0,
                    );
                }
                if current {
                    let bw = 200.0;
                    let pct = (100.0 * p.tech_progress / t.cost as f32).clamp(0.0, 100.0);
                    draw_rectangle(
                        box_r.x + self.u(470.0),
                        y + self.u(27.0),
                        bw,
                        5.0,
                        c(34, 42, 61),
                    );
                    draw_rectangle(
                        box_r.x + self.u(470.0),
                        y + self.u(27.0),
                        bw * pct / 100.0,
                        5.0,
                        RESEARCH,
                    );
                    self.text(
                        &format!("{} / {}", p.tech_progress as i32, t.cost),
                        box_r.x + self.u(678.0),
                        y + self.u(24.0),
                        12.0,
                        MUTED,
                    );
                }
                y += 24.0;
            }
            y += 6.0;
        }
        self.close_button(box_r);
    }

    fn modal_diplomacy(&mut self, st: &State, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 560.0, 380.0);
        self.text(
            "Diplomacy",
            box_r.x + self.u(20.0),
            box_r.y + self.u(30.0),
            18.0,
            ACCENT,
        );
        self.text(
            "Relations with the other empires of the galaxy.",
            box_r.x + self.u(20.0),
            box_r.y + self.u(54.0),
            13.0,
            MUTED,
        );
        let p = st.player().clone();
        let my_str = engine::empire_strength(st, p.id).round() as i32;
        let mut y = box_r.y + self.u(80.0);
        for e in &st.empires {
            if e.id == p.id || !e.alive {
                continue;
            }
            let at_war = p.at_war_with.contains(&e.id);
            let offered = p.peace_offers.contains(&e.id);
            let their_str = engine::empire_strength(st, e.id).round() as i32;
            self.text(
                &e.name,
                box_r.x + self.u(20.0),
                y + self.u(12.0),
                14.0,
                rgb(&e.color),
            );
            self.text(
                if at_war { "AT WAR" } else { "PEACE" },
                box_r.x + self.u(200.0),
                y + self.u(12.0),
                13.0,
                if at_war { WAR } else { GOOD },
            );
            self.text(
                &format!(
                    "Strength {} (you: {})  |  Colonies {}  |  Ships {}",
                    their_str,
                    my_str,
                    st.empire_colonies(e.id).len(),
                    engine::empire_ship_count(st, e.id)
                ),
                box_r.x + self.u(20.0),
                y + self.u(29.0),
                12.0,
                MUTED,
            );
            if offered {
                self.text(
                    "Peace offer received.",
                    box_r.x + self.u(20.0),
                    y + self.u(43.0),
                    12.0,
                    GOOD,
                );
            }
            let by = y + if offered { 60.0 } else { 44.0 };
            let act = if offered {
                Action::AcceptPeace(e.id)
            } else if at_war {
                Action::SuePeace(e.id)
            } else {
                Action::DeclareWar(e.id)
            };
            let label = if offered {
                "Accept Peace"
            } else if at_war {
                "Sue for Peace"
            } else {
                "Declare War"
            };
            let style = if !offered && !at_war {
                Style::Danger
            } else {
                Style::Small
            };
            self.button(
                rect(right(box_r) - self.u(170.0), y, self.u(150.0), 22.0),
                label,
                act,
                true,
                style,
                13.0,
            );
            y = by + self.u(8.0);
        }
        self.close_button(box_r);
    }

    fn modal_battles(&mut self, st: &State, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 640.0, 520.0);
        self.text(
            "Battle Reports",
            box_r.x + self.u(20.0),
            box_r.y + self.u(30.0),
            18.0,
            ACCENT,
        );
        self.text(
            "Recent engagements across the galaxy, newest first.",
            box_r.x + self.u(20.0),
            box_r.y + self.u(54.0),
            13.0,
            MUTED,
        );
        let mut y = box_r.y + self.u(78.0);
        let battles: Vec<&crate::state::BattleRec> = st.battles.iter().rev().collect();
        if battles.is_empty() {
            self.hint(
                "No battles have been fought yet.",
                box_r.x + self.u(20.0),
                y + self.u(11.0),
                MUTED,
            );
        }
        for b in battles {
            if y > bottom(box_r) - self.u(70.0) {
                self.hint("...", box_r.x + self.u(20.0), y + self.u(11.0), MUTED);
                break;
            }
            self.text(
                &b.system,
                box_r.x + self.u(20.0),
                y + self.u(12.0),
                14.0,
                TEXT,
            );
            self.text(
                if b.kind == "colony" {
                    "COLONY ASSAULT"
                } else {
                    "SPACE BATTLE"
                },
                box_r.x + self.u(180.0),
                y + self.u(13.0),
                12.0,
                if b.kind == "colony" { WAR } else { MUTED },
            );
            self.text(
                &format!("Turn {}", b.turn),
                right(box_r) - self.u(90.0),
                y + self.u(12.0),
                12.0,
                RESEARCH,
            );
            self.text(
                &b.participants.join(" vs "),
                box_r.x + self.u(20.0),
                y + self.u(27.0),
                13.0,
                MUTED,
            );
            self.text(
                &format!(
                    "Winner: {}   |   Losses: {} ship{}",
                    b.winner.clone().unwrap_or_else(|| "none".to_string()),
                    b.losses,
                    if b.losses == 1 { "" } else { "s" }
                ),
                box_r.x + self.u(20.0),
                y + self.u(42.0),
                12.0,
                TEXT,
            );
            y += 52.0;
        }
        self.close_button(box_r);
    }

    fn modal_help(&mut self, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 620.0, 560.0);
        self.text(
            "How to Play",
            box_r.x + self.u(20.0),
            box_r.y + self.u(30.0),
            18.0,
            ACCENT,
        );
        let mut y = box_r.y + self.u(58.0);
        let lines: [(&str, bool); 29] = [
            ("Goal", true),
            ("Win by eliminating all rivals, or by controlling 55% of colonizable worlds after turn 20.", false),
            ("", false),
            ("Controls", true),
            ("Click a system or fleet to select it. With a fleet selected, click a system to move there.", false),
            ("Drag to pan, scroll to zoom, click the minimap to jump, right-click or Esc to deselect.", false),
            ("Enter ends the turn. Ctrl+S saves.", false),
            ("", false),
            ("Expansion", true),
            ("Build a Colony Ship at a colony with a Shipyard, then send it to an uncolonized world.", false),
            ("Colonies grow population over time; bigger populations produce more of everything.", false),
            ("", false),
            ("Construction", true),
            ("Colonies have limited building slots. Mines boost minerals, Markets credits, Labs research,", false),
            ("Habitats speed growth, Shipyards enable ships, Planetary Shields add defense platforms.", false),
            ("", false),
            ("Combat", true),
            ("Hostile fleets in the same system fight at the end of the turn. Destroy the defenders and", false),
            ("the attacker captures the colony. Fleet speed comes from the slowest ship.", false),
            ("", false),
            ("Exploration", true),
            ("Systems start unexplored. Move fleets within sensor range to reveal them - and to claim", false),
            ("anomalies, which only a fleet survey (not colony sensors) can uncover.", false),
            ("", false),
            ("Diplomacy", true),
            ("Rival empires expand, build fleets and eventually declare war. Sue for peace when ahead.", false),
            ("", false),
            ("Ship Designer", true),
            ("Open it from a colony with a Shipyard to combine hulls, weapons, armour and engines.", false),
        ];
        for (text, header) in lines {
            if header {
                self.text(text, box_r.x + self.u(20.0), y + self.u(10.0), 13.0, MUTED);
                y += 18.0;
            } else if !text.is_empty() {
                self.text(text, box_r.x + self.u(20.0), y + self.u(11.0), 13.0, TEXT);
                y += 16.0;
            } else {
                y += 6.0;
            }
        }
        self.close_button(box_r);
    }

    // A row builder needs the module table, the selection and the tech state;
    // grouping them would only add indirection.
    #[allow(clippy::too_many_arguments)]
    fn module_row(
        &mut self,
        x: f32,
        y: f32,
        label: &str,
        rows: &[(&str, &str, &str, Option<&str>)],
        current: &str,
        tag: char,
        techs: &[String],
    ) -> f32 {
        self.text(label, x, y + self.u(14.0), 13.0, MUTED);
        let mut bx = x + self.u(66.0);
        for (key, name, desc, tech) in rows {
            let locked = tech.map(|t| !techs.iter().any(|x| x == t)).unwrap_or(false);
            let selected = *key == current;
            let text = format!("{}{}", name, if locked { " *" } else { "" });
            let bw = self.text_width(&text, 13.0) + self.u(14.0);
            let r = rect(bx, y, bw, 22.0);
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if selected { c(28, 68, 88) } else { PANEL2 },
            );
            draw_rectangle_lines(
                r.x,
                r.y,
                r.w,
                r.h,
                1.0,
                if selected { ACCENT } else { BORDER },
            );
            self.text(
                &text,
                bx + self.u(7.0),
                y + self.u(15.0),
                13.0,
                if locked { c(90, 100, 125) } else { TEXT },
            );
            if !locked {
                let act = match tag {
                    'h' => Action::DesignHull(key.to_string()),
                    'w' => Action::DesignWeapon(key.to_string()),
                    'a' => Action::DesignArmor(key.to_string()),
                    _ => Action::DesignEngine(key.to_string()),
                };
                self.clickables.push(Clickable { r, action: act });
            }
            bx += bw + self.u(5.0);
            let _ = desc;
        }
        let sel = rows.iter().find(|r| r.0 == current).unwrap_or(&rows[0]);
        self.text(sel.2, x + self.u(66.0), y + self.u(37.0), 12.0, MUTED);
        y + self.u(54.0)
    }

    fn modal_designer(&mut self, st: &State, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 580.0, 560.0);
        self.text(
            "Ship Designer",
            box_r.x + self.u(20.0),
            box_r.y + self.u(30.0),
            18.0,
            ACCENT,
        );
        self.text(
            "Combine a hull with weapon, armour and engine modules.",
            box_r.x + self.u(20.0),
            box_r.y + self.u(54.0),
            13.0,
            MUTED,
        );

        let empire = st.player().clone();
        let techs = empire.techs.clone();
        let mut y = box_r.y + self.u(76.0);

        self.text(
            "Design name",
            box_r.x + self.u(20.0),
            y + self.u(15.0),
            13.0,
            MUTED,
        );
        let r = rect(
            box_r.x + self.u(130.0),
            y - self.u(3.0),
            box_r.w - self.u(150.0),
            24.0,
        );
        draw_rectangle(r.x, r.y, r.w, r.h, PANEL2);
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            1.0,
            if self.d_name_focus { ACCENT } else { BORDER },
        );
        if self.d_name.is_empty() {
            self.text(
                "Custom Design",
                r.x + self.u(8.0),
                r.y + self.u(17.0),
                14.0,
                MUTED,
            );
        } else {
            self.text(
                &self.d_name,
                r.x + self.u(8.0),
                r.y + self.u(17.0),
                14.0,
                TEXT,
            );
        }
        self.clickables.push(Clickable {
            r,
            action: Action::DesignNameFocus,
        });
        y += 32.0;

        let hull_rows: Vec<(&str, &str, &str, Option<&str>)> = data::hulls()
            .iter()
            .map(|m| (m.key, m.name, m.desc, m.tech))
            .collect();
        let weapon_rows: Vec<(&str, &str, &str, Option<&str>)> = data::weapon_modules()
            .iter()
            .map(|m| (m.key, m.name, m.desc, m.tech))
            .collect();
        let armor_rows: Vec<(&str, &str, &str, Option<&str>)> = data::armor_modules()
            .iter()
            .map(|m| (m.key, m.name, m.desc, m.tech))
            .collect();
        let engine_rows: Vec<(&str, &str, &str, Option<&str>)> = data::engine_modules()
            .iter()
            .map(|m| (m.key, m.name, m.desc, m.tech))
            .collect();
        let cur_hull = self.d_hull.clone();
        let cur_weapon = self.d_weapon.clone();
        let cur_armor = self.d_armor.clone();
        let cur_engine = self.d_engine.clone();
        y = self.module_row(
            box_r.x + self.u(20.0),
            y,
            "Hull",
            &hull_rows,
            &cur_hull,
            'h',
            &techs,
        );
        y = self.module_row(
            box_r.x + self.u(20.0),
            y,
            "Weapon",
            &weapon_rows,
            &cur_weapon,
            'w',
            &techs,
        );
        y = self.module_row(
            box_r.x + self.u(20.0),
            y,
            "Armour",
            &armor_rows,
            &cur_armor,
            'a',
            &techs,
        );
        y = self.module_row(
            box_r.x + self.u(20.0),
            y,
            "Engine",
            &engine_rows,
            &cur_engine,
            'e',
            &techs,
        );

        let name = if self.d_name.trim().is_empty() {
            "Custom Design"
        } else {
            self.d_name.trim()
        };
        let d = data::compute_design(
            name,
            &self.d_hull,
            &self.d_weapon,
            &self.d_armor,
            &self.d_engine,
        );
        let py = y + self.u(6.0);
        draw_rectangle(
            box_r.x + self.u(20.0),
            py,
            box_r.w - self.u(40.0),
            44.0,
            PANEL2,
        );
        self.text(
            &format!(
                "HP {}   |   Attack {}   |   Speed {}   |   {}cr {}min   |   {} turns",
                d.hp, d.attack, d.speed, d.credits, d.minerals, d.build_turns
            ),
            box_r.x + self.u(30.0),
            py + self.u(16.0),
            14.0,
            TEXT,
        );
        let mut req: Vec<String> = Vec::new();
        for key in [&self.d_hull, &self.d_weapon, &self.d_armor, &self.d_engine] {
            let tech = data::hull(key)
                .map(|h| h.tech)
                .or_else(|| data::module_by_key(data::weapon_modules(), key).map(|m| m.tech))
                .or_else(|| data::module_by_key(data::armor_modules(), key).map(|m| m.tech))
                .or_else(|| data::module_by_key(data::engine_modules(), key).map(|m| m.tech))
                .flatten();
            if let Some(t) = tech {
                if !techs.iter().any(|x| x == t) {
                    if let Some(td) = data::tech(t) {
                        req.push(td.name.to_string());
                    }
                }
            }
        }
        self.text(
            if req.is_empty() {
                "No special research required"
            } else {
                "Requires: "
            },
            box_r.x + self.u(30.0),
            py + self.u(34.0),
            12.0,
            if req.is_empty() { GOOD } else { WAR },
        );
        if !req.is_empty() {
            self.text(
                &req.join(", "),
                box_r.x + self.u(30.0) + self.text_width("Requires: ", 12.0),
                py + self.u(34.0),
                12.0,
                WAR,
            );
        }

        let mut dy = py + self.u(56.0);
        self.text(
            "YOUR DESIGNS",
            box_r.x + self.u(20.0),
            dy + self.u(10.0),
            13.0,
            MUTED,
        );
        dy += 16.0;
        let designs = engine::list_designs(st, empire.id);
        if designs.is_empty() {
            self.hint(
                "No custom designs yet - configure one above and save it.",
                box_r.x + self.u(20.0),
                dy + self.u(11.0),
                MUTED,
            );
            dy += 16.0;
        }
        for key in designs {
            if dy > bottom(box_r) - self.u(58.0) {
                break;
            }
            let dd = engine::ship_def(st, &key);
            let (dname, ddesc) = match dd {
                Some(x) => (x.name, x.desc),
                None => (key.clone(), String::new()),
            };
            self.text(
                &dname,
                box_r.x + self.u(20.0),
                dy + self.u(11.0),
                13.0,
                TEXT,
            );
            self.text(
                &ddesc,
                box_r.x + self.u(200.0),
                dy + self.u(12.0),
                12.0,
                MUTED,
            );
            self.button(
                rect(
                    right(box_r) - self.u(96.0),
                    dy - self.u(2.0),
                    self.u(76.0),
                    20.0,
                ),
                "Delete",
                Action::DeleteDesign(key.clone()),
                true,
                Style::Danger,
                12.0,
            );
            dy += 22.0;
        }

        self.button(
            rect(
                right(box_r) - self.u(290.0),
                bottom(box_r) - self.u(42.0),
                self.u(130.0),
                28.0,
            ),
            "Save Design",
            Action::SaveDesign,
            true,
            Style::Primary,
            14.0,
        );
        self.close_button(box_r);
    }

    fn modal_gameover(&mut self, st: Option<&State>, w: f32, h: f32) {
        let box_r = self.modal_box(w, h, 460.0, 200.0);
        let (won, winner_name, turns) = match st {
            Some(st) => (
                st.winner == Some(0),
                st.winner
                    .and_then(|id| st.empire(id))
                    .map(|e| e.name.clone()),
                st.turn,
            ),
            None => (false, None, 0),
        };
        let title = if won { "Victory!" } else { "Defeat" };
        self.text(
            title,
            box_r.x + self.u(20.0),
            box_r.y + self.u(40.0),
            18.0,
            if won { GOOD } else { BAD },
        );
        let msg = if won {
            "Your empire dominates the galaxy. The stars themselves bow to your banner.".to_string()
        } else if let Some(name) = winner_name {
            format!("{} has conquered your empire.", name)
        } else {
            "Your empire has been wiped out.".to_string()
        };
        let mut yy = box_r.y + self.u(70.0);
        for line in self.wrap(&msg, 14.0, box_r.w - self.u(40.0)) {
            self.text(&line, box_r.x + self.u(20.0), yy, 14.0, TEXT);
            yy += 16.0;
        }
        self.text(
            &format!("Turns played: {}", turns),
            box_r.x + self.u(20.0),
            yy + self.u(20.0),
            13.0,
            MUTED,
        );
        self.button(
            rect(
                right(box_r) - self.u(130.0),
                bottom(box_r) - self.u(46.0),
                self.u(110.0),
                28.0,
            ),
            "New Game",
            Action::OpenNew,
            true,
            Style::Primary,
            14.0,
        );
    }
}
