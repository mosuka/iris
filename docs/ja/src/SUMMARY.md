# Summary

- [はじめに](README.md)
- [アーキテクチャ概要](architecture.md)

# Getting Started

- [はじめに](getting_started.md)
  - [インストール](getting_started/installation.md)
  - [クイックスタート](getting_started/quickstart.md)
  - [サンプル](getting_started/examples.md)

# コアコンセプト

- [スキーマとフィールド](concepts/schema_and_fields.md)
- [テキスト解析](concepts/analysis.md)
- [エンベディング](concepts/embedding.md)
- [ストレージ](concepts/storage.md)
- [インデクシング](concepts/indexing.md)
  - [Lexical インデクシング](concepts/indexing/lexical_indexing.md)
  - [Vector インデクシング](concepts/indexing/vector_indexing.md)
- [検索](concepts/search.md)
  - [Lexical 検索](concepts/search/lexical_search.md)
  - [Vector 検索](concepts/search/vector_search.md)
  - [ハイブリッド検索](concepts/search/hybrid_search.md)
- [Query DSL](concepts/query_dsl.md)

# laurus

- [ライブラリ概要](laurus.md)
  - [Engine](laurus/engine.md)
  - [スコアリングとランキング](laurus/scoring.md)
  - [ファセット](laurus/faceting.md)
  - [ハイライト](laurus/highlighting.md)
  - [スペル修正](laurus/spelling_correction.md)
  - [ID 管理](laurus/id_management.md)
  - [永続化と WAL](laurus/persistence.md)
  - [削除とコンパクション](laurus/deletions.md)
  - [エラーハンドリング](laurus/error_handling.md)
  - [拡張性](laurus/extensibility.md)
  - [API リファレンス](laurus/api_reference.md)

# laurus-cli

- [CLI 概要](laurus-cli.md)
  - [インストール](laurus-cli/installation.md)
  - [ハンズオンチュートリアル](laurus-cli/tutorial.md)
  - [コマンド](laurus-cli/commands.md)
  - [スキーマフォーマット](laurus-cli/schema_format.md)
  - [REPL](laurus-cli/repl.md)

# laurus-server

- [サーバー概要](laurus-server.md)
  - [はじめに](laurus-server/getting_started.md)
  - [ハンズオンチュートリアル](laurus-server/tutorial.md)
  - [設定](laurus-server/configuration.md)
  - [gRPC API リファレンス](laurus-server/grpc_api.md)
  - [HTTP Gateway](laurus-server/http_gateway.md)

# 開発ガイド

- [ビルドとテスト](development/build_and_test.md)
- [Feature Flags](development/feature_flags.md)
- [プロジェクト構成](development/project_structure.md)
