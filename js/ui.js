/* Stellar Dominion — canvas map, input, panels, modals */
window.SS = window.SS || {};
(function () {
  const U = SS.util;
  const D = SS.data;
  const G = SS.game;

  const UI = (SS.ui = {});
  let canvas, ctx, dpr = 1;
  let cam = { x: 1800, y: 1200, zoom: 0.45 };
  let starfield = [];
  let nebulae = [];

  const EMPIRE_VIEW = 0; // player empire id

  // minimap geometry (canvas is 256x148, scaled by CSS)
  const MM_W = 256;
  const MM_H = 148;
  const MM_SCALE = Math.min((MM_W - 16) / SS.galaxy.WIDTH, (MM_H - 12) / SS.galaxy.HEIGHT);
  const MM_OX = (MM_W - SS.galaxy.WIDTH * MM_SCALE) / 2;
  const MM_OY = (MM_H - SS.galaxy.HEIGHT * MM_SCALE) / 2;

  /* ============================================================
   * Init
   * ============================================================ */
  UI.init = function () {
    canvas = document.getElementById('map');
    ctx = canvas.getContext('2d');
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
    makeStarfield();
    bindInput();
    bindButtons();
    bindPanelActions();
    bindModalActions();
    showNewGameModal();
    requestAnimationFrame(loop);
  };

  function resizeCanvas() {
    const rect = canvas.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
  }

  function makeStarfield() {
    starfield = [];
    for (let i = 0; i < 1400; i++) {
      starfield.push({
        x: Math.random() * SS.galaxy.WIDTH,
        y: Math.random() * SS.galaxy.HEIGHT,
        r: Math.random() * 1.5 + 0.4,
        a: Math.random() * 0.6 + 0.25,
      });
    }
    nebulae = [];
    const colors = ['#1b2a5e', '#3a1b5e', '#0e3a4a', '#4a1b2a', '#1b4a3a', '#2a2a6e'];
    for (let i = 0; i < 9; i++) {
      nebulae.push({
        x: Math.random() * SS.galaxy.WIDTH,
        y: Math.random() * SS.galaxy.HEIGHT,
        r: U.rand(400, 950),
        color: U.choice(colors),
      });
    }
  }

  /* ============================================================
   * Camera helpers
   * ============================================================ */
  function worldToScreen(wx, wy) {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    return { x: (wx - cam.x) * cam.zoom + w / 2, y: (wy - cam.y) * cam.zoom + h / 2 };
  }

  function screenToWorld(sx, sy) {
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    return { x: (sx - w / 2) / cam.zoom + cam.x, y: (sy - h / 2) / cam.zoom + cam.y };
  }

  function clampCam() {
    cam.x = U.clamp(cam.x, -300, SS.galaxy.WIDTH + 300);
    cam.y = U.clamp(cam.y, -300, SS.galaxy.HEIGHT + 300);
  }

  UI.focusHome = function () {
    const g = G.state;
    if (!g) return;
    const p = G.player();
    const home = G.getSystem(p.homeSystemId);
    if (home) {
      cam.x = home.x;
      cam.y = home.y;
      cam.zoom = 0.7;
    }
  };

  UI.setCamera = function (x, y, zoom) {
    if (x != null) cam.x = x;
    if (y != null) cam.y = y;
    if (zoom != null) cam.zoom = U.clamp(zoom, 0.12, 2.6);
    clampCam();
  };

  UI.zoomBy = function (factor) {
    cam.zoom = U.clamp(cam.zoom * factor, 0.12, 2.6);
  };

  UI.getCamera = function () {
    return { x: cam.x, y: cam.y, zoom: cam.zoom };
  };

  /* ============================================================
   * Input
   * ============================================================ */
  function bindInput() {
    let dragging = false;
    let moved = false;
    let lastX = 0;
    let lastY = 0;
    let downX = 0;
    let downY = 0;

    canvas.addEventListener('mousedown', (e) => {
      dragging = true;
      moved = false;
      downX = lastX = e.clientX;
      downY = lastY = e.clientY;
    });

    window.addEventListener('mousemove', (e) => {
      if (dragging) {
        const dx = e.clientX - lastX;
        const dy = e.clientY - lastY;
        if (Math.abs(e.clientX - downX) + Math.abs(e.clientY - downY) > 4) moved = true;
        if (moved) {
          cam.x -= dx / cam.zoom;
          cam.y -= dy / cam.zoom;
          clampCam();
        }
        lastX = e.clientX;
        lastY = e.clientY;
      } else {
        updateHover(e.clientX, e.clientY);
      }
    });

    window.addEventListener('mouseup', (e) => {
      if (dragging && !moved && e.target === canvas) handleClick(e.clientX, e.clientY);
      dragging = false;
    });

    canvas.addEventListener(
      'wheel',
      (e) => {
        e.preventDefault();
        const rect = canvas.getBoundingClientRect();
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;
        const before = screenToWorld(mx, my);
        cam.zoom *= e.deltaY < 0 ? 1.12 : 0.89;
        cam.zoom = U.clamp(cam.zoom, 0.12, 2.6);
        const after = screenToWorld(mx, my);
        cam.x += before.x - after.x;
        cam.y += before.y - after.y;
        clampCam();
      },
      { passive: false }
    );

    canvas.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      if (G.state) G.select(null);
    });

    // minimap click-to-jump
    const mm = document.getElementById('minimap');
    if (mm) {
      mm.addEventListener('click', (e) => {
        const rect = mm.getBoundingClientRect();
        if (!rect.width || !G.state) return;
        const mx = (e.clientX - rect.left) * (MM_W / rect.width);
        const my = (e.clientY - rect.top) * (MM_H / rect.height);
        UI.setCamera((mx - MM_OX) / MM_SCALE, (my - MM_OY) / MM_SCALE, null);
      });
    }

    window.addEventListener('keydown', (e) => {
      const modalOpen = !document.getElementById('modal-overlay').classList.contains('hidden');
      if (e.key === 'Escape') {
        if (modalOpen) hideModal();
        else if (G.state) G.select(null);
      }
      if (e.key === 'Enter' && !modalOpen && G.state && !G.state.over) {
        G.endTurn();
      }
    });
  }

  function mousePos(e) {
    const rect = canvas.getBoundingClientRect();
    return { x: e.clientX - rect.left, y: e.clientY - rect.top };
  }

  function updateHover(cx, cy) {
    const g = G.state;
    if (!g) return;
    const p = mousePos({ clientX: cx, clientY: cy });
    const w = screenToWorld(p.x, p.y);
    let cursor = 'crosshair';
    const f = fleetAtWorld(w.x, w.y);
    const s = systemAtWorld(w.x, w.y);
    if (f || s) cursor = 'pointer';
    canvas.style.cursor = cursor;
  }

  function systemAtWorld(wx, wy) {
    const g = G.state;
    for (const s of g.systems) {
      const r = 14 + 8 * s.starRadius;
      if (U.dist(wx, wy, s.x, s.y) <= r) return s;
    }
    return null;
  }

  function fleetDrawPos(f) {
    if (f.moves) return { x: f.x, y: f.y };
    const sys = G.getSystem(f.systemId);
    if (!sys) return { x: f.x, y: f.y };
    const idle = G.fleetsAt(sys.id).filter((x) => !x.moves);
    const idx = Math.max(0, idle.indexOf(f));
    const angle = -Math.PI / 2 + (idx % 8) * (Math.PI * 2 / 8);
    const r = 46;
    return { x: sys.x + Math.cos(angle) * r, y: sys.y + Math.sin(angle) * r };
  }

  function fleetAtWorld(wx, wy) {
    const g = G.state;
    for (const f of g.fleets) {
      if (!isFleetVisible(f)) continue;
      const pos = fleetDrawPos(f);
      if (U.dist(wx, wy, pos.x, pos.y) <= 20) return f;
    }
    return null;
  }

  function isFleetVisible(f) {
    const g = G.state;
    const sys = G.getSystem(f.systemId);
    return !!(sys && sys.exploredBy.has(EMPIRE_VIEW));
  }

  function handleClick(cx, cy) {
    const g = G.state;
    if (!g || g.over) return;
    const p = mousePos({ clientX: cx, clientY: cy });
    const w = screenToWorld(p.x, p.y);

    const fleet = fleetAtWorld(w.x, w.y);
    if (fleet) {
      G.select('fleet', fleet.id);
      return;
    }

    const sys = systemAtWorld(w.x, w.y);
    if (sys) {
      const sel = g.selected;
      if (sel && sel.type === 'fleet') {
        const fl = G.getFleet(sel.id);
        if (fl && fl.empireId === EMPIRE_VIEW && fl.systemId !== sys.id) {
          if (G.issueMove(fl, sys)) {
            UI.toast(`Fleet moving to ${sys.name}`);
            UI.refreshAll();
            return;
          }
        }
      }
      G.select('system', sys.id);
      return;
    }

    G.select(null);
  }

  /* ============================================================
   * Top bar buttons
   * ============================================================ */
  function bindButtons() {
    document.getElementById('btn-endturn').addEventListener('click', () => G.endTurn());
    document.getElementById('btn-research').addEventListener('click', () => {
      if (G.state && !G.state.over) researchModal();
    });
    document.getElementById('btn-diplomacy').addEventListener('click', () => {
      if (G.state && !G.state.over) diplomacyModal();
    });
    document.getElementById('btn-battles').addEventListener('click', () => {
      if (G.state && !G.state.over) battlesModal();
    });
    document.getElementById('btn-save').addEventListener('click', () => {
      if (G.state) G.saveGame();
    });
    document.getElementById('btn-new').addEventListener('click', showNewGameModal);
    document.getElementById('btn-home').addEventListener('click', () => {
      if (G.state) UI.focusHome();
    });
    document.getElementById('btn-help').addEventListener('click', showHelpModal);
  }

  /* ============================================================
   * Render loop
   * ============================================================ */
  function loop() {
    draw();
    requestAnimationFrame(loop);
  }

  function draw() {
    const g = G.state;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // background
    ctx.fillStyle = '#04060c';
    ctx.fillRect(0, 0, w, h);

    if (!g) return;

    // galactic band (soft elliptical glow across the map)
    const bandC = worldToScreen(SS.galaxy.WIDTH / 2, SS.galaxy.HEIGHT / 2);
    ctx.save();
    ctx.translate(bandC.x, bandC.y);
    ctx.rotate(-0.32);
    ctx.scale(1, 0.3);
    const bandR = 2000 * cam.zoom;
    const bandGrad = ctx.createRadialGradient(0, 0, 0, 0, 0, bandR);
    bandGrad.addColorStop(0, 'rgba(96,116,190,0.11)');
    bandGrad.addColorStop(0.55, 'rgba(72,84,150,0.05)');
    bandGrad.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = bandGrad;
    ctx.beginPath();
    ctx.arc(0, 0, bandR, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();

    // nebulae
    for (const n of nebulae) {
      const p = worldToScreen(n.x, n.y);
      const r = n.r * cam.zoom;
      if (p.x + r < 0 || p.x - r > w || p.y + r < 0 || p.y - r > h) continue;
      const grad = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, r);
      grad.addColorStop(0, n.color + '33');
      grad.addColorStop(1, n.color + '00');
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      ctx.fill();
    }

    // starfield
    for (const st of starfield) {
      const p = worldToScreen(st.x, st.y);
      if (p.x < 0 || p.x > w || p.y < 0 || p.y > h) continue;
      ctx.globalAlpha = st.a;
      ctx.fillStyle = '#cfd8ff';
      ctx.beginPath();
      ctx.arc(p.x, p.y, st.r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;

    // fleet move lines
    for (const f of g.fleets) {
      if (!f.moves || !isFleetVisible(f)) continue;
      const from = worldToScreen(f.x, f.y);
      const toSys = G.getSystem(f.moves.toSystemId);
      if (!toSys) continue;
      const to = worldToScreen(toSys.x, toSys.y);
      const e = G.getEmpire(f.empireId);
      ctx.strokeStyle = e ? e.color : '#fff';
      ctx.globalAlpha = 0.45;
      ctx.setLineDash([5, 5]);
      ctx.lineWidth = 1.2;
      ctx.beginPath();
      ctx.moveTo(from.x, from.y);
      ctx.lineTo(to.x, to.y);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.globalAlpha = 1;
    }

    // systems
    for (const s of g.systems) {
      drawSystem(s);
    }

    // fleets
    for (const f of g.fleets) {
      if (!isFleetVisible(f)) continue;
      drawFleet(f);
    }

    // selection highlight
    drawSelection();

    // minimap
    drawMinimap();
  }

  function drawMinimap() {
    const mm = document.getElementById('minimap');
    if (!mm) return;
    const mctx = mm.getContext('2d');
    const g = G.state;
    mctx.clearRect(0, 0, MM_W, MM_H);
    mctx.fillStyle = '#070b16';
    mctx.fillRect(0, 0, MM_W, MM_H);
    if (!g) return;

    for (const s of g.systems) {
      const x = MM_OX + s.x * MM_SCALE;
      const y = MM_OY + s.y * MM_SCALE;
      const explored = s.exploredBy.has(EMPIRE_VIEW);
      if (explored && s.ownerEmpireId !== null) {
        const e = G.getEmpire(s.ownerEmpireId);
        mctx.fillStyle = e ? e.color : '#ffffff';
      } else if (explored) {
        mctx.fillStyle = '#8d9cc4';
      } else {
        mctx.fillStyle = '#3a4560';
      }
      mctx.beginPath();
      mctx.arc(x, y, 2, 0, Math.PI * 2);
      mctx.fill();
    }

    for (const f of g.fleets) {
      if (!isFleetVisible(f)) continue;
      const pos = fleetDrawPos(f);
      const x = MM_OX + pos.x * MM_SCALE;
      const y = MM_OY + pos.y * MM_SCALE;
      const e = G.getEmpire(f.empireId);
      mctx.fillStyle = e ? e.color : '#ffffff';
      mctx.fillRect(x - 1.5, y - 1.5, 3, 3);
    }

    // viewport rectangle
    const tl = screenToWorld(0, 0);
    const br = screenToWorld(canvas.clientWidth, canvas.clientHeight);
    mctx.strokeStyle = 'rgba(77,208,225,0.75)';
    mctx.lineWidth = 1;
    mctx.strokeRect(
      MM_OX + tl.x * MM_SCALE,
      MM_OY + tl.y * MM_SCALE,
      (br.x - tl.x) * MM_SCALE,
      (br.y - tl.y) * MM_SCALE
    );
  }

  function drawSystem(s) {
    const p = worldToScreen(s.x, s.y);
    const explored = s.exploredBy.has(EMPIRE_VIEW);
    const zoomF = Math.sqrt(cam.zoom);
    const r = Math.max(4.5, 7 * s.starRadius * zoomF);
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    if (p.x < -60 || p.x > w + 60 || p.y < -60 || p.y > h + 60) return;

    // glow
    ctx.beginPath();
    ctx.arc(p.x, p.y, r * 2.4, 0, Math.PI * 2);
    ctx.fillStyle = hexToRgba(s.starColor, explored ? 0.16 : 0.08);
    ctx.fill();

    // core
    ctx.beginPath();
    ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
    ctx.fillStyle = explored ? s.starColor : '#5a6478';
    ctx.fill();

    // owner ring
    if (explored && s.ownerEmpireId !== null) {
      const e = G.getEmpire(s.ownerEmpireId);
      if (e) {
        ctx.strokeStyle = e.color;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(p.x, p.y, r + 4, 0, Math.PI * 2);
        ctx.stroke();
      }
    } else if (!explored) {
      // unknown system marker
      ctx.strokeStyle = 'rgba(125,141,176,0.55)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(p.x, p.y, r + 3, 0, Math.PI * 2);
      ctx.stroke();
    }

    // name
    ctx.font = `${Math.max(10, 12 * zoomF)}px "Segoe UI", sans-serif`;
    ctx.textAlign = 'center';
    ctx.fillStyle = explored ? 'rgba(214,226,245,0.85)' : 'rgba(125,141,176,0.65)';
    ctx.fillText(explored ? s.name : 'Unknown', p.x, p.y + r + 15 * zoomF + 2);
  }

  function drawFleet(f) {
    const pos = fleetDrawPos(f);
    const p = worldToScreen(pos.x, pos.y);
    const e = G.getEmpire(f.empireId);
    const color = e ? e.color : '#fff';
    const zoomF = Math.sqrt(cam.zoom);
    const fs = Math.max(7, 9 * zoomF);
    const dest = f.moves ? G.getSystem(f.moves.toSystemId) : null;
    const angle = dest ? Math.atan2(dest.y - f.y, dest.x - f.x) : -Math.PI / 2;

    ctx.save();
    ctx.translate(p.x, p.y);
    ctx.rotate(angle + Math.PI / 2);
    ctx.beginPath();
    ctx.moveTo(fs, 0);
    ctx.lineTo(-fs * 0.7, fs * 0.62);
    ctx.lineTo(-fs * 0.4, 0);
    ctx.lineTo(-fs * 0.7, -fs * 0.62);
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = 'rgba(4,6,12,0.9)';
    ctx.lineWidth = 1.2;
    ctx.stroke();
    ctx.restore();

    // ship count
    ctx.font = `bold ${Math.max(10, 11 * zoomF)}px "Segoe UI", sans-serif`;
    ctx.textAlign = 'left';
    ctx.fillStyle = 'rgba(235,242,255,0.95)';
    ctx.fillText(String(f.ships.length), p.x + fs + 2, p.y - fs);
  }

  function drawSelection() {
    const g = G.state;
    const sel = g.selected;
    if (!sel) return;

    if (sel.type === 'system') {
      const s = G.getSystem(sel.id);
      if (!s) return;
      const p = worldToScreen(s.x, s.y);
      const r = Math.max(6, 9 * s.starRadius * cam.zoom * 0.9) + 8;
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.setLineDash([4, 3]);
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
    } else if (sel.type === 'fleet') {
      const f = G.getFleet(sel.id);
      if (!f) return;
      const pos = fleetDrawPos(f);
      const p = worldToScreen(pos.x, pos.y);
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(p.x, p.y, 12, 0, Math.PI * 2);
      ctx.stroke();
    }
  }

  function hexToRgba(hex, alpha) {
    const h = hex.replace('#', '');
    const r = parseInt(h.substring(0, 2), 16);
    const g = parseInt(h.substring(2, 4), 16);
    const b = parseInt(h.substring(4, 6), 16);
    return `rgba(${r},${g},${b},${alpha})`;
  }

  /* ============================================================
   * Top bar / panels
   * ============================================================ */
  UI.refreshAll = function () {
    refreshTopbar();
    UI.refreshSelection();
    UI.refreshLog();
  };

  function refreshTopbar() {
    const g = G.state;
    if (!g) return;
    const p = G.player();
    document.getElementById('tb-turn').textContent = 'Turn ' + g.turn;
    document.getElementById('tb-credits').innerHTML =
      `Credits ${U.fmt(p.credits)} <span class="gain">+${Math.round(p.lastIncome.credits)}</span>`;
    document.getElementById('tb-minerals').innerHTML =
      `Minerals ${U.fmt(p.minerals)} <span class="gain">+${Math.round(p.lastIncome.minerals)}</span>`;
    const pool = Math.round(p.researchPool || 0);
    document.getElementById('tb-research').innerHTML =
      `Research ${p.currentTech ? Math.floor(p.techProgress) + '/' + D.TECHS[p.currentTech].cost : '—'}` +
      (pool ? ` <span class="gain">(${pool} banked)</span>` : '') +
      ` <span class="gain">+${Math.round(p.lastIncome.research)}</span>`;
    document.getElementById('tb-tech').textContent = p.currentTech
      ? 'Researching: ' + D.TECHS[p.currentTech].name
      : 'No active research';
  }

  UI.refreshSelection = function () {
    const g = G.state;
    const el = document.getElementById('sel-content');
    if (!g) {
      el.innerHTML = '<div class="hint">No game in progress.</div>';
      return;
    }
    const sel = g.selected;
    if (sel && sel.type === 'fleet' && !G.getFleet(sel.id)) g.selected = null;
    if (sel && sel.type === 'system' && !G.getSystem(sel.id)) g.selected = null;

    const s = g.selected;
    if (!s) el.innerHTML = overviewPanel();
    else if (s.type === 'system') el.innerHTML = systemPanel(G.getSystem(s.id));
    else if (s.type === 'fleet') el.innerHTML = fleetPanel(G.getFleet(s.id));
  };

  UI.refreshLog = function () {
    const g = G.state;
    const el = document.getElementById('log');
    if (!g) {
      el.innerHTML = '';
      return;
    }
    el.innerHTML = g.logs
      .slice(-80)
      .map(
        (l) =>
          `<div class="log-entry ${l.type}"><span class="lt">[${l.turn}]</span>${U.esc(l.msg)}</div>`
      )
      .join('');
    el.scrollTop = el.scrollHeight;
  };

  function overviewPanel() {
    const p = G.player();
    const colonies = G.empireColonies(p.id).length;
    const systems = G.empireSystems(p.id).length;
    const fleets = G.empireFleets(p.id).length;
    const ships = G.empireShipCount(p.id);
    const str = Math.round(G.empireStrength(p.id));
    return `
      <div class="sel-title">${U.esc(p.name)}</div>
      <div class="sel-sub">${U.esc(p.desc)}</div>
      <div class="stat-grid">
        <div>Colonies <b>${colonies}</b></div>
        <div>Systems controlled <b>${systems}</b></div>
        <div>Fleets <b>${fleets}</b></div>
        <div>Ships <b>${ships}</b></div>
        <div>Fleet strength <b>${str}</b></div>
        <div>Research <b>${p.currentTech ? D.TECHS[p.currentTech].name : 'none'}</b></div>
      </div>
      <div class="hint" style="margin-top:10px">Select a system to inspect it, or a fleet to command it. Press Enter to end the turn.</div>
    `;
  }

  function systemPanel(sys) {
    if (!sys) return '';
    const p = G.player();
    const explored = sys.exploredBy.has(EMPIRE_VIEW);
    const star = D.STAR_CLASSES.find((x) => x.key === sys.starClass);
    let html = `<div class="sel-title">${U.esc(sys.name)}</div>`;

    if (!explored) {
      html += `<div class="sel-sub">Unexplored system</div>
        <div class="hint">Send a fleet within sensor range to reveal this system's planets and owner.</div>`;
      return html;
    }

    const pt = D.PLANET_TYPES[sys.planet.type];
    const owner = sys.ownerEmpireId !== null ? G.getEmpire(sys.ownerEmpireId) : null;
    const sizeName = ['Small', 'Medium', 'Large'][sys.planet.size - 1] || '—';

    html += `<div class="sel-sub">${star ? star.name : ''} · ${pt.name} world</div>`;
    html += `<div class="stat-grid">
      <div>Planet <b style="color:${pt.color}">${pt.name}</b></div>
      <div>Size <b>${sizeName}</b></div>
      <div>Richness <b>${sys.planet.richness.toFixed(2)}×</b></div>
      <div>Max population <b>${sys.planet.maxPop}</b></div>
      <div>Owner <b style="color:${owner ? owner.color : '#7d8db0'}">${owner ? U.esc(owner.name) : 'Unclaimed'}</b></div>
    </div>`;

    if (sys.anomaly) {
      html += `<div class="sel-row" style="margin-top:6px"><span class="sel-card" style="border-color:#ffd54f">Anomaly — <b>${U.esc(sys.anomaly.name)}</b>: ${U.esc(sys.anomaly.desc)}</span></div>`;
    }

    if (sys.planet.colony) {
      const col = sys.planet.colony;
      const c = G.getEmpire(col.empireId);
      const prod = G.colonyProduction(sys, col, c);
      html += `<div class="sel-section"><h4>Colony — ${U.esc(c.name)}</h4>
        <div class="stat-grid">
          <div>Population <b>${Math.floor(col.population)}</b> / ${G.maxPop(sys, c)}</div>
          <div>Income <b>+${prod.minerals.toFixed(1)} min · +${prod.credits.toFixed(1)} cr · +${prod.research.toFixed(1)} rp</b></div>
        </div>
        <div class="sel-row" style="margin-top:6px">`;
      if (col.buildings.length) {
        for (const b of col.buildings) {
          html += `<span class="sel-card">${D.BUILDINGS[b].name}</span>`;
        }
      } else {
        html += '<span class="hint">No buildings yet</span>';
      }
      html += `</div>`;

      if (col.queue.length) {
        html += `<div class="sel-section"><h4>Construction</h4>`;
        col.queue.forEach((k, i) => {
          const ship = D.SHIPS[k];
          const prog = i === 0 ? ` (${col.queueProgress}/${ship.buildTurns})` : '';
          html += `<div class="queue-item ${i === 0 ? 'active' : ''}">${ship.name}${prog}</div>`;
        });
        html += `</div>`;
      }

      if (c.isPlayer) html += buildOptions(sys, col, c);
      html += `</div>`;
    } else {
      const canCol = G.canColonize(p, sys);
      html += `<div class="hint" style="margin-top:8px">${
        canCol
          ? 'This world is uncolonized. Send a Colony Ship to found a colony here.'
          : pt.habitability <= 0
          ? 'This world cannot support a colony.'
          : 'This hostile world requires Xeno Colonization research.'
      }</div>`;
    }
    return html;
  }

  function buildOptions(sys, col, empire) {
    let html = `<div class="sel-section"><h4>Build</h4><div class="sel-row">`;
    for (const key of Object.keys(D.BUILDINGS)) {
      const b = D.BUILDINGS[key];
      if (!b.repeat && col.buildings.includes(key)) continue;
      if (col.buildings.length >= G.slots(sys)) continue;
      const afford = empire.minerals >= b.cost;
      html += `<button class="btn btn-small" data-action="build-building" data-arg="${key}" ${afford ? '' : 'disabled'} title="${b.desc}">${b.name} <span class="cost">${b.cost} min</span></button>`;
    }
    html += `</div>`;
    if (col.buildings.includes('shipyard')) {
      html += `<div class="sel-row" style="margin-top:6px">`;
      for (const key of Object.keys(D.SHIPS)) {
        const ship = D.SHIPS[key];
        if (ship.tech && !empire.techs[ship.tech]) continue;
        const cost = G.shipCost(empire, key);
        const afford = empire.credits >= cost.credits && empire.minerals >= cost.minerals;
        html += `<button class="btn btn-small" data-action="queue-ship" data-arg="${key}" ${afford ? '' : 'disabled'} title="${ship.desc}">${ship.name} <span class="cost">${cost.credits}cr ${cost.minerals}min · ${ship.buildTurns}t</span></button>`;
      }
      html += `</div>`;
    }
    html += `</div>`;
    return html;
  }

  function fleetPanel(fleet) {
    if (!fleet) return '';
    const e = G.getEmpire(fleet.empireId);
    const sys = G.getSystem(fleet.systemId);
    const dest = fleet.moves ? G.getSystem(fleet.moves.toSystemId) : null;
    let html = `<div class="sel-title">Fleet — ${U.esc(e.name)}</div>`;
    html += `<div class="sel-sub">At ${U.esc(sys.name)}${
      dest ? ` → ${U.esc(dest.name)} (${fleet.moves.turnsLeft} turn${fleet.moves.turnsLeft > 1 ? 's' : ''})` : ''
    } · Speed ${G.fleetSpeed(fleet)} · Strength ${Math.round(G.fleetStrength(fleet))}</div>`;

    html += `<div class="sel-section"><h4>Ships (${fleet.ships.length})</h4><div class="sel-row">`;
    for (const s of fleet.ships) {
      const st = G.effectiveShipStats(e, s.cls);
      const pct = U.clamp(Math.round((100 * s.hp) / st.hp), 0, 100);
      html += `<span class="sel-card">${D.SHIPS[s.cls].name}<span class="hp-bar"><i style="width:${pct}%"></i></span></span>`;
    }
    html += `</div></div>`;

    if (e.isPlayer) {
      const others = G.fleetsAt(fleet.systemId).filter(
        (f) => f.id !== fleet.id && f.empireId === e.id && !f.moves
      );
      html += `<div class="sel-row">
        <button class="btn btn-small" data-action="split-fleet" ${fleet.ships.length < 2 ? 'disabled' : ''}>Split Fleet</button>
        <button class="btn btn-small" data-action="stop-fleet" ${fleet.moves ? '' : 'disabled'}>Stop</button>
        ${others.length ? `<button class="btn btn-small" data-action="merge-all" data-arg="${fleet.id}">Merge All Here (${others.length + 1})</button>` : ''}
        <span class="hint">Click a system on the map to move this fleet there.</span>
      </div>`;
      if (others.length) {
        html += `<div class="sel-section"><h4>Other fleets here</h4><div class="sel-row">`;
        for (const o of others) {
          const names = o.ships.map((s) => D.SHIPS[s.cls].name).join(', ');
          html += `<span class="sel-card">${U.esc(names)}
            <button class="btn btn-small" data-action="merge-into" data-arg="${fleet.id}:${o.id}">Merge</button>
          </span>`;
        }
        html += `</div></div>`;
      }
    } else {
      html += `<div class="hint">Hostile fleet. ${
        G.atWar(e.id, EMPIRE_VIEW) ? 'Engage it by moving a fleet into its system.' : 'You are not at war with this empire.'
      }</div>`;
    }
    return html;
  }

  /* ============================================================
   * Panel actions (delegation)
   * ============================================================ */
  function bindPanelActions() {
    document.getElementById('sel-content').addEventListener('click', (ev) => {
      const btn = ev.target.closest('[data-action]');
      if (!btn) return;
      const action = btn.getAttribute('data-action');
      const arg = btn.getAttribute('data-arg');
      const g = G.state;
      if (!g) return;

      if (action === 'build-building') {
        if (g.selected && g.selected.type === 'system') {
          G.buildBuilding(G.getSystem(g.selected.id), arg);
          UI.refreshAll();
        }
      } else if (action === 'queue-ship') {
        if (g.selected && g.selected.type === 'system') {
          G.queueShip(G.getSystem(g.selected.id), arg);
          UI.refreshAll();
        }
      } else if (action === 'split-fleet') {
        if (g.selected && g.selected.type === 'fleet') {
          G.splitFleet(G.getFleet(g.selected.id));
          UI.refreshAll();
        }
      } else if (action === 'stop-fleet') {
        if (g.selected && g.selected.type === 'fleet') {
          G.stopFleet(G.getFleet(g.selected.id));
          UI.refreshAll();
        }
      } else if (action === 'merge-into') {
        const parts = arg.split(':').map(Number);
        G.mergeFleets(parts[0], parts[1]);
        UI.refreshAll();
      } else if (action === 'merge-all') {
        const f = G.getFleet(Number(arg));
        if (f) G.mergeAllFleetsAt(f.systemId, f.empireId);
        UI.refreshAll();
      }
    });
  }

  /* ============================================================
   * Modals
   * ============================================================ */
  function showModal(html) {
    document.getElementById('modal').innerHTML = html;
    document.getElementById('modal-overlay').classList.remove('hidden');
  }

  function hideModal() {
    document.getElementById('modal-overlay').classList.add('hidden');
    document.getElementById('modal').innerHTML = '';
  }

  UI.hideModal = hideModal;

  function bindModalActions() {
    document.getElementById('modal').addEventListener('click', (ev) => {
      const btn = ev.target.closest('[data-action]');
      if (!btn) return;
      handleModalAction(btn.getAttribute('data-action'), btn.getAttribute('data-arg'));
    });
    document.getElementById('modal-overlay').addEventListener('click', (ev) => {
      if (ev.target.id === 'modal-overlay') hideModal();
    });
  }

  function handleModalAction(action, arg) {
    const g = G.state;
    const p = g ? G.player() : null;

    if (action === 'close-modal') hideModal();
    else if (action === 'set-tech' && p) {
      G.setResearch(p, arg);
      researchModal();
      refreshTopbar();
    } else if (action === 'declare-war' && p) {
      G.declareWar(p.id, Number(arg));
      diplomacyModal();
      UI.refreshAll();
    } else if (action === 'sue-peace' && p) {
      G.playerSuePeace(G.getEmpire(Number(arg)));
      diplomacyModal();
      UI.refreshAll();
    } else if (action === 'accept-peace' && p) {
      G.makePeace(p.id, Number(arg));
      diplomacyModal();
      UI.refreshAll();
    } else if (action === 'start-new-game') {
      startNewGame();
    } else if (action === 'load-save') {
      if (G.loadGame()) {
        hideModal();
        UI.focusHome();
        UI.refreshAll();
        UI.toast('Saved game loaded');
      }
    } else if (action === 'new-game-modal') {
      showNewGameModal();
    }
  }

  /* ---------------- new game ---------------- */
  function showNewGameModal() {
    const hasSave = G.hasSave();
    const html = `
      <h2>Stellar Dominion</h2>
      <p>A turn-based 4X space strategy game. Explore the galaxy, expand your empire,
      exploit rich worlds, research new technology, and exterminate your rivals.</p>
      <h3>New Game</h3>
      <div class="form-row"><label>Empire name</label>
        <input id="ng-name" value="Terran Consortium" maxlength="30"></div>
      <div class="form-row"><label>Galaxy size</label>
        <select id="ng-size">
          <option value="34">Small — 34 systems</option>
          <option value="46" selected>Medium — 46 systems</option>
          <option value="60">Large — 60 systems</option>
        </select></div>
      <div class="form-row"><label>Rival empires</label>
        <select id="ng-ai">
          <option value="1">1 rival</option>
          <option value="2">2 rivals</option>
          <option value="3" selected>3 rivals</option>
        </select></div>
      <div class="form-row"><label>Difficulty</label>
        <select id="ng-diff">
          <option value="easy">Easy</option>
          <option value="normal" selected>Normal</option>
          <option value="hard">Hard</option>
        </select></div>
      <div class="modal-actions">
        ${hasSave ? '<button class="btn" data-action="load-save">Load Saved Game</button>' : ''}
        <button class="btn btn-primary" data-action="start-new-game">Start Game</button>
      </div>
    `;
    showModal(html);
  }

  function startNewGame() {
    const name = document.getElementById('ng-name').value.trim() || 'Terran Consortium';
    const size = Number(document.getElementById('ng-size').value);
    const numAI = Number(document.getElementById('ng-ai').value);
    const difficulty = document.getElementById('ng-diff').value;
    G.newGame({ numAI, numSystems: size, difficulty, playerName: name });
    hideModal();
    UI.focusHome();
    UI.refreshAll();
    UI.toast('Welcome to Stellar Dominion');
  }

  /* ---------------- research ---------------- */
  function researchModal() {
    const p = G.player();
    let html = `<h2>Research</h2>
      <p>Research points accumulate each turn from your colonies${
        (p.researchPool || 0) > 0 ? ` — <b>${Math.floor(p.researchPool)}</b> RP banked` : ''
      }. Choose a technology to work on.</p>`;
    for (const cat of D.TECH_CATS) {
      html += `<h3>${cat}</h3>`;
      for (const key of Object.keys(D.TECHS)) {
        const t = D.TECHS[key];
        if (t.cat !== cat) continue;
        const done = !!p.techs[key];
        const current = p.currentTech === key;
        const locked = !t.requires.every((r) => p.techs[r]);
        const cls = done ? 'done' : current ? 'current' : locked ? 'locked' : '';
        html += `<div class="tech-row ${cls}">
          <div class="tech-info">
            <div class="tech-name">${t.name}</div>
            <div class="tech-desc">${t.desc}${
              t.requires.length && !locked ? ' <em>(requires ' + t.requires.map((r) => D.TECHS[r].name).join(', ') + ')</em>' : ''
            }</div>
            ${current ? `<div class="progress-bar"><i style="width:${U.clamp(100 * p.techProgress / t.cost, 0, 100)}%"></i></div>
            <div class="tech-cost">${Math.floor(p.techProgress)} / ${t.cost} RP</div>` : ''}
          </div>
          <div class="tech-cost">${
            done ? 'Researched' : locked ? 'Locked' : current ? 'In progress' : t.cost + ' RP'
          }</div>
          ${!done && !current && !locked ? `<button class="btn btn-small" data-action="set-tech" data-arg="${key}">Research</button>` : ''}
        </div>`;
      }
    }
    html += `<div class="modal-actions"><button class="btn" data-action="close-modal">Close</button></div>`;
    showModal(html);
  }

  /* ---------------- diplomacy ---------------- */
  function diplomacyModal() {
    const p = G.player();
    let html = `<h2>Diplomacy</h2>
      <p>Relations with the other empires of the galaxy. War is declared by the bold — or by you.</p>`;
    for (const e of G.state.empires) {
      if (e.id === p.id || !e.alive) continue;
      const atWar = p.atWarWith.has(e.id);
      const offered = p.peaceOffers.includes(e.id);
      const myStr = Math.round(G.empireStrength(p.id));
      const theirStr = Math.round(G.empireStrength(e.id));
      html += `<div class="dip-row">
        <div>
          <div class="dip-name" style="color:${e.color}">${U.esc(e.name)}</div>
          <div class="dip-status">
            ${atWar ? '<span class="tag war">At War</span>' : '<span class="tag peace">Peace</span>'}
            Strength ${theirStr} (you: ${myStr}) · Colonies ${G.empireColonies(e.id).length} · Ships ${G.empireShipCount(e.id)}
          </div>
          ${offered ? '<div class="dip-status" style="color:#66bb6a">Peace offer received — accept or keep fighting.</div>' : ''}
        </div>
        <div class="sel-row">
          ${offered ? `<button class="btn btn-small" data-action="accept-peace" data-arg="${e.id}">Accept Peace</button>` : ''}
          ${atWar
            ? `<button class="btn btn-small" data-action="sue-peace" data-arg="${e.id}">Sue for Peace</button>`
            : `<button class="btn btn-small btn-danger" data-action="declare-war" data-arg="${e.id}">Declare War</button>`}
        </div>
      </div>`;
    }
    html += `<div class="modal-actions"><button class="btn" data-action="close-modal">Close</button></div>`;
    showModal(html);
  }

  /* ---------------- battle reports ---------------- */
  function battlesModal() {
    const g = G.state;
    let html = `<h2>Battle Reports</h2>
      <p>Recent engagements across the galaxy, newest first.</p>`;
    if (!g.battles.length) {
      html += '<p class="hint">No battles have been fought yet.</p>';
    } else {
      for (const b of g.battles.slice().reverse()) {
        html += `<div class="dip-row">
          <div>
            <div class="dip-name">${U.esc(b.system)}
              <span class="tag ${b.kind === 'colony' ? 'war' : ''}">${b.kind === 'colony' ? 'Colony assault' : 'Space battle'}</span>
            </div>
            <div class="dip-status">${U.esc(b.participants.join(' vs '))}</div>
            <div class="dip-status">Winner: <b>${b.winner ? U.esc(b.winner) : 'none'}</b> · Losses: ${b.losses} ship${b.losses === 1 ? '' : 's'}</div>
          </div>
          <div class="tech-cost">Turn ${b.turn}</div>
        </div>`;
      }
    }
    html += `<div class="modal-actions"><button class="btn" data-action="close-modal">Close</button></div>`;
    showModal(html);
  }

  /* ---------------- help ---------------- */
  function showHelpModal() {
    const html = `
      <h2>How to Play</h2>
      <p><b>Goal:</b> win by eliminating all rivals, or by controlling 55% of colonizable worlds after turn 20.</p>
      <h3>Turn structure</h3>
      <p>Each turn your colonies produce <b>minerals</b>, <b>credits</b> and <b>research</b>.
      Research completes technologies automatically; minerals and credits are stockpiled for construction.</p>
      <h3>Controls</h3>
      <p>
        • <b>Click</b> a system or fleet to select it.<br>
        • With a fleet selected, <b>click a system</b> to move there (takes several turns).<br>
        • <b>Drag</b> to pan, <b>scroll</b> to zoom, <b>right-click / Esc</b> to deselect.<br>
        • <b>Enter</b> ends the turn.
      </p>
      <h3>Expansion</h3>
      <p>Build a <b>Colony Ship</b> at a colony with a <b>Shipyard</b>, then send it to an uncolonized world.
      Colonies grow population over time — bigger populations produce more.</p>
      <h3>Construction</h3>
      <p>Colonies have limited building slots (planet size). Buildings cost minerals:
      <b>Mines</b> boost minerals, <b>Markets</b> credits, <b>Research Labs</b> research,
      <b>Habitats</b> speed population growth, <b>Shipyards</b> enable ship construction, and
      <b>Planetary Shields</b> add defense platforms in combat.</p>
      <h3>Combat</h3>
      <p>When hostile fleets meet, battle is resolved automatically at the end of the turn.
      If the defender is destroyed, the attacker captures the colony. Fleet speed comes from the
      slowest ship — scouts are fast, colony ships slow.</p>
      <h3>Exploration</h3>
      <p>Systems start unexplored. Move fleets within sensor range to reveal planets, owners and enemy fleets.</p>
      <div class="modal-actions"><button class="btn" data-action="close-modal">Close</button></div>
    `;
    showModal(html);
  }

  /* ---------------- game over ---------------- */
  UI.showGameOver = function (won) {
    const g = G.state;
    const winner = g.winner
      ? typeof g.winner === 'object'
        ? g.winner
        : G.getEmpire(g.winner)
      : null;
    const html = `
      <h2 style="color:${won ? '#66bb6a' : '#ef5350'}">${won ? 'Victory!' : 'Defeat'}</h2>
      <p>${
        won
          ? 'Your empire dominates the galaxy. The stars themselves bow to your banner.'
          : winner
          ? `${U.esc(winner.name)} has conquered your empire. The galaxy remembers your brief, bright flame.`
          : 'Your empire has been wiped out.'
      }</p>
      <p>Turns played: <b>${g.turn}</b></p>
      <div class="modal-actions">
        <button class="btn btn-primary" data-action="new-game-modal">New Game</button>
      </div>
    `;
    showModal(html);
  };

  /* ============================================================
   * Toast
   * ============================================================ */
  let toastTimer = null;
  UI.toast = function (msg) {
    const el = document.getElementById('toast');
    el.textContent = msg;
    el.classList.remove('hidden');
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.classList.add('hidden'), 2200);
  };
})();
