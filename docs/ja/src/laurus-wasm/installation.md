# インストール

## npm / yarn / pnpm

```bash
npm install laurus-wasm
# または
yarn add laurus-wasm
# または
pnpm add laurus-wasm
```

## CDN（ES Module）

```html
<script type="module">
  import init, { Index, Schema } from 'https://unpkg.com/laurus-wasm/laurus_wasm.js';
  await init();
  // ...
</script>
```

## ソースからビルド

前提条件:

- [Rust](https://rustup.rs/)（stable）
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

```bash
git clone https://github.com/mosuka/laurus.git
cd laurus/laurus-wasm

# バンドラー向け（webpack、vite 等）
wasm-pack build --target bundler --release

# ブラウザ直接利用向け（<script type="module">）
wasm-pack build --target web --release
```

出力は `pkg/` ディレクトリに生成されます。

## ブラウザ対応状況

laurus-wasm は以下をサポートするブラウザが必要です:

- WebAssembly（すべてのモダンブラウザ）
- ES Modules

OPFS 永続化には以下のブラウザが対応しています:

| ブラウザ | 最小バージョン |
| -------- | -------------- |
| Chrome   | 102+           |
| Firefox  | 111+           |
| Safari   | 15.2+          |
| Edge     | 102+           |
