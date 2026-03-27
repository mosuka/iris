# Installation

## npm / yarn / pnpm

```bash
npm install laurus-wasm
# or
yarn add laurus-wasm
# or
pnpm add laurus-wasm
```

## CDN (ES Module)

```html
<script type="module">
  import init, { Index, Schema } from 'https://unpkg.com/laurus-wasm/laurus_wasm.js';
  await init();
  // ...
</script>
```

## Build from Source

Prerequisites:

- [Rust](https://rustup.rs/) (stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

```bash
git clone https://github.com/mosuka/laurus.git
cd laurus/laurus-wasm

# For use with bundlers (webpack, vite, etc.)
wasm-pack build --target bundler --release

# For direct browser use (<script type="module">)
wasm-pack build --target web --release
```

The output will be in the `pkg/` directory.

## Browser Compatibility

laurus-wasm requires a browser that supports:

- WebAssembly (all modern browsers)
- ES Modules

For OPFS persistence, the following browsers are supported:

| Browser | Minimum Version |
| ------- | --------------- |
| Chrome  | 102+            |
| Firefox | 111+            |
| Safari  | 15.2+           |
| Edge    | 102+            |
