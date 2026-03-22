# laurus-mcp

[![Crates.io](https://img.shields.io/crates/v/laurus-mcp.svg)](https://crates.io/crates/laurus-mcp)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[Laurus](https://github.com/mosuka/laurus) 検索エンジンの [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) サーバーです。Claude などの AI アシスタントが標準 MCP stdio トランスポートを通じてドキュメントのインデックス登録や検索を行えるようにします。

## 機能

- **MCP stdio トランスポート** -- サブプロセスとして起動し、stdin/stdout 経由で AI クライアントと通信
- **gRPC クライアント** -- すべてのツール呼び出しを実行中の `laurus-server` インスタンスにプロキシ
- **全検索モード** -- Lexical（BM25）、Vector（HNSW/Flat/IVF）、ハイブリッド検索
- **動的接続** -- `connect` ツールで任意の laurus-server エンドポイントに接続可能
- **ドキュメントライフサイクル** -- MCP ツールを通じてドキュメントの上書き・追加・取得・削除・検索が可能

## アーキテクチャ

```text
AI クライアント（Claude など）
  └─ stdio (JSON-RPC) ─→ laurus-mcp
                            └─ gRPC ─→ laurus-server
                                         └─ ディスク上のインデックス
```

## クイックスタート

```bash
# laurus-server を起動
laurus serve --port 50051

# Claude Code で設定
claude mcp add laurus -- laurus mcp --endpoint http://localhost:50051
```

### Claude Desktop

Claude Desktop の設定ファイル（`claude_desktop_config.json`）に以下を追加してください:

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "laurus": {
      "command": "laurus",
      "args": ["mcp", "--endpoint", "http://localhost:50051"]
    }
  }
}
```

## MCP ツール

| ツール | 説明 |
| :--- | :--- |
| `connect` | laurus-server gRPC エンドポイントに接続 |
| `create_index` | スキーマを指定してインデックスを作成 |
| `get_stats` | インデックス統計を取得（ドキュメント数、ベクトルフィールド） |
| `get_schema` | 現在のインデックスのスキーマを取得 |
| `add_field` | インデックスにフィールドを動的に追加 |
| `delete_field` | スキーマからフィールドを削除 |
| `put_document` | ドキュメントを上書き（upsert） |
| `add_document` | ドキュメントを新しいチャンクとして追加（追記） |
| `get_documents` | ID で全ドキュメントを取得 |
| `delete_documents` | ID で全ドキュメントを削除 |
| `commit` | 保留中の変更をディスクにコミット |
| `search` | Laurus 統一クエリ DSL でドキュメントを検索（Lexical / Vector / ハイブリッド） |

## ドキュメント

- [MCP サーバーガイド](https://mosuka.github.io/laurus/ja/laurus-mcp.html)
- [はじめに](https://mosuka.github.io/laurus/ja/laurus-mcp/getting_started.html)
- [ツールリファレンス](https://mosuka.github.io/laurus/ja/laurus-mcp/tools.html)

## ライセンス

このプロジェクトは MIT ライセンスの下で公開されています。詳細は [LICENSE](../LICENSE) ファイルを参照してください。
