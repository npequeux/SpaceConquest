/* Stellar Dominion — small helpers shared everywhere */
window.SS = window.SS || {};
(function () {
  let _uid = 1;

  SS.util = {
    uid: () => _uid++,
    rand: (a, b) => a + Math.random() * (b - a),
    randInt: (a, b) => Math.floor(a + Math.random() * (b - a + 1)),
    choice: (arr) => arr[Math.floor(Math.random() * arr.length)],
    dist: (ax, ay, bx, by) => Math.hypot(ax - bx, ay - by),
    clamp: (v, lo, hi) => Math.max(lo, Math.min(hi, v)),
    sum: (arr) => arr.reduce((a, b) => a + b, 0),
    groupBy: (arr, fn) => {
      const out = {};
      for (const x of arr) {
        const k = fn(x);
        (out[k] = out[k] || []).push(x);
      }
      return out;
    },
    fmt: (n) => Math.round(n).toLocaleString('en-US'),
    esc: (s) => String(s).replace(/[&<>"']/g, (c) => ({
      '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
    }[c])),
  };
})();
