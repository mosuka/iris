# 開発

## 前提条件

- [Rust](https://rustup.rs/)（stable、`wasm32-unknown-unknown` ターゲット付き）
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/)（テストと npm publish 用）

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

## ビルド

```bash
cd laurus-wasm

# デバッグビルド（コンパイル高速）
wasm-pack build --target web --dev

# リリースビルド（最適化）
wasm-pack build --target web --release

# バンドラーターゲット（webpack、vite 等）
wasm-pack build --target bundler --release
```

## プロジェクト構成

```text
laurus-wasm/
├── Cargo.toml          # Rust 依存関係（wasm-bindgen、laurus コア）
├── package.json        # npm パッケージメタデータ
├── src/
│   ├── lib.rs          # モジュール宣言
│   ├── index.rs        # Index クラス（CRUD + 検索）
│   ├── schema.rs       # Schema ビルダー
│   ├── search.rs       # SearchRequest / SearchResult
│   ├── query.rs        # クエリ型定義
│   ├── convert.rs      # JsValue ↔ Document 変換
│   ├── analysis.rs     # トークナイザー / フィルターラッパー
│   ├── errors.rs       # LaurusError → JsValue 変換
│   └── storage.rs      # OPFS 永続化レイヤー
└── js/
    └── opfs_bridge.js  # Origin Private File System 用 JS グルーコード
```

## アーキテクチャノート

### ストレージ戦略

laurus-wasm は二層ストレージアプローチを採用しています:

1. **MemoryStorage**（ランタイム） -- すべての読み書き操作は Laurus の
   インメモリストレージを経由します。これは `Storage` トレイトの
   `Send + Sync` 要件を満たします。

2. **OPFS**（永続化） -- `commit()` 時に MemoryStorage の全状態が
   OPFS ファイルにシリアライズされます。`Index.open()` 時に OPFS
   ファイルが MemoryStorage にロードされます。

この設計により、JS ハンドルの `Send + Sync` 非互換性を回避しつつ、
コアエンジンを変更せずに永続化を実現しています。

### Feature Flags

`laurus` コアは Feature Flags で WASM をサポートしています:

```toml
# laurus-wasm はデフォルト機能なしで laurus に依存
laurus = { workspace = true, default-features = false }
```

これにより、ネイティブ専用の依存関係（tokio/full、rayon、memmap2 等）が
除外され、`#[cfg(target_arch = "wasm32")]` フォールバックで並列処理が
逐次処理に切り替わります。

## テスト

```bash
# ビルド確認
cargo build -p laurus-wasm --target wasm32-unknown-unknown

# Clippy
cargo clippy -p laurus-wasm --target wasm32-unknown-unknown -- -D warnings
```

ブラウザテストは `wasm-pack test` で実行できます:

```bash
wasm-pack test --headless --chrome
```
