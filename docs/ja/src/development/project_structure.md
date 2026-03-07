# プロジェクト構成

Laurus は 3 つのクレートを持つ Cargo ワークスペースとして構成されています。

## ワークスペースレイアウト

```text
laurus/                          # リポジトリルート
├── Cargo.toml                   # ワークスペース定義
├── laurus/                      # コア検索エンジンライブラリ
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs               # パブリック API とモジュール宣言
│   │   ├── engine.rs            # Engine, EngineBuilder, SearchRequest
│   │   ├── analysis/            # テキスト解析パイプライン
│   │   ├── lexical/             # 転置インデックス（Inverted Index）と Lexical 検索
│   │   ├── vector/              # ベクトルインデックス（Flat, HNSW, IVF）
│   │   ├── embedding/           # Embedder 実装
│   │   ├── storage/             # ストレージバックエンド（memory, file, mmap）
│   │   ├── store/               # ドキュメントログ（WAL）
│   │   ├── spelling/            # スペル修正
│   │   ├── data/                # DataValue, Document 型
│   │   └── error.rs             # LaurusError 型
│   └── examples/                # 実行可能なサンプル
├── laurus-cli/                  # コマンドラインインターフェース
│   ├── Cargo.toml
│   └── src/
│       └── main.rs              # CLI エントリーポイント（clap）
├── laurus-server/               # gRPC サーバー + HTTP ゲートウェイ
│   ├── Cargo.toml
│   ├── proto/                   # Protobuf サービス定義
│   └── src/
│       ├── lib.rs               # サーバーライブラリ
│       ├── config.rs            # TOML 設定
│       ├── grpc/                # gRPC サービス実装
│       └── gateway/             # HTTP/JSON ゲートウェイ（axum）
└── docs/                        # mdBook ドキュメント
    ├── book.toml
    └── src/
        └── SUMMARY.md           # 目次
```

## クレートの役割

| クレート | 種類 | 説明 |
| :--- | :--- | :--- |
| `laurus` | ライブラリ | Lexical 検索、ベクトル検索、ハイブリッド検索を備えたコア検索エンジン |
| `laurus-cli` | バイナリ | インデックス管理、ドキュメント CRUD、検索、REPL のための CLI ツール |
| `laurus-server` | ライブラリ + バイナリ | オプションの HTTP/JSON ゲートウェイ付き gRPC サーバー |

`laurus-cli` と `laurus-server` はどちらも `laurus` ライブラリクレートに依存しています。

## 設計規約

- **モジュールスタイル**: ファイルベースのモジュール（Rust 2018 edition スタイル）、`mod.rs` は使用しない
  - `src/tokenizer.rs` + `src/tokenizer/dictionary.rs`
  - 不可: `src/tokenizer/mod.rs`
- **エラーハンドリング**: ライブラリのエラー型には `thiserror`、`anyhow` はバイナリクレートのみ
- **`unwrap()` / `expect()` 禁止**: 本番コードでは使用不可（テストでは使用可）
- **非同期**: すべてのパブリック API は Tokio ランタイムで async/await を使用
- **Unsafe**: すべての `unsafe` ブロックに `// SAFETY: ...` コメントが必須
- **ドキュメント**: すべてのパブリックな型、関数、列挙型にドキュメントコメント（`///`）が必須
- **ライセンス**: 依存クレートは MIT または Apache-2.0 互換であること
