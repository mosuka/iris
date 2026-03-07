# ビルドとテスト

## 前提条件

- **Rust** 1.85 以降（edition 2024）
- **Cargo**（Rust に付属）
- **protobuf コンパイラ**（`protoc`）-- `laurus-server` のビルドに必要

## ビルド

```bash
# すべてのクレートをビルド
cargo build

# 特定の Feature を指定してビルド
cargo build --features embeddings-candle

# リリースモードでビルド
cargo build --release
```

## テスト

```bash
# すべてのテストを実行
cargo test

# 名前を指定して特定のテストを実行
cargo test <test_name>

# 特定のクレートのテストを実行
cargo test -p laurus
cargo test -p laurus-cli
cargo test -p laurus-server
```

## Lint

```bash
# clippy を警告エラー扱いで実行
cargo clippy -- -D warnings
```

## フォーマット

```bash
# フォーマットチェック
cargo fmt --check

# フォーマットを適用
cargo fmt
```

## ドキュメント

### API ドキュメント

```bash
# Rust API ドキュメントを生成して開く
cargo doc --no-deps --open
```

### mdBook ドキュメント

```bash
# ドキュメントサイトをビルド
mdbook build docs

# ローカルプレビューサーバーを起動 (http://localhost:3000)
mdbook serve docs

# Markdown ファイルを Lint
markdownlint-cli2 "docs/src/**/*.md"
```
