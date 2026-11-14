# Web build

`index.html` + `mq_js_bundle.js` + `stellar-dominion.wasm` make up the
GitHub Pages build of Stellar Dominion.

- `mq_js_bundle.js` is macroquad's own JS glue, vendored unmodified from the
  `macroquad` crate (`macroquad-0.4.16/js/mq_js_bundle.js`, MIT OR Apache-2.0).
  It provides the WebGL context and the `env` imports miniquad's wasm backend
  expects, then calls the wasm's `main` export.
- `stellar-dominion.wasm` is produced by CI
  (`cargo build --release --target wasm32-unknown-unknown`) and is git-ignored.

To try it locally:

```bash
cd rust/web
cargo build --release --target wasm32-unknown-unknown   # from rust/
cp ../target/wasm32-unknown-unknown/release/stellar-dominion.wasm .
python -m http.server 8000
# open http://localhost:8000
```

A server is required — opening `index.html` straight from disk will not work,
because the wasm is fetched over HTTP.
