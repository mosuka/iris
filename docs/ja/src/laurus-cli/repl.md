# REPL（対話モード）

REPL は、毎回 `laurus` コマンドをフルで入力することなく、インデックスを操作できる対話型セッションを提供します。

## REPL の起動

```bash
laurus --index-dir ./my_index repl
```

```text
Laurus REPL (type 'help' for commands, 'quit' to exit)
laurus>
```

REPL は起動時にインデックスを開き、セッション中ロードされた状態を維持します。

## 利用可能なコマンド

コマンドは CLI と同じ `<操作> <リソース>` の順序に従います。

| コマンド | 説明 |
| :--- | :--- |
| `search <query>` | インデックスを検索 |
| `add field <name> <json>` | スキーマにフィールドを追加 |
| `add doc <id> <json>` | ドキュメントを追加 |
| `get stats` | インデックスの統計情報を表示 |
| `get schema` | 現在のスキーマを表示 |
| `get doc <id>` | ID でドキュメントを取得 |
| `delete field <name>` | スキーマからフィールドを削除 |
| `delete doc <id>` | ID でドキュメントを削除 |
| `commit` | 保留中の変更をコミット |
| `help` | 利用可能なコマンドを表示 |
| `quit` / `exit` | REPL を終了 |

## 使用例

### 検索

```text
laurus> search body:rust
╭──────┬────────┬────────────────────────────────────╮
│ ID   │ Score  │ Fields                             │
├──────┼────────┼────────────────────────────────────┤
│ doc1 │ 0.8532 │ body: Rust is a systems..., title… │
╰──────┴────────┴────────────────────────────────────╯
```

### フィールドの管理

```text
laurus> add field category {"Text": {"indexed": true, "stored": true}}
Field 'category' added.
laurus> delete field category
Field 'category' deleted.
```

### ドキュメントの追加とコミット

```text
laurus> add doc doc4 {"title":"New Document","body":"Some content here."}
Document 'doc4' added.
laurus> commit
Changes committed.
```

### 情報の取得

```text
laurus> get stats
Document count: 3

laurus> get schema
{
  "fields": { ... },
  "default_fields": ["title", "body"]
}

laurus> get doc doc4
╭──────┬───────────────────────────────────────────────╮
│ ID   │ Fields                                        │
├──────┼───────────────────────────────────────────────┤
│ doc4 │ body: Some content here., title: New Document │
╰──────┴───────────────────────────────────────────────╯
```

### ドキュメントの削除

```text
laurus> delete doc doc4
Document 'doc4' deleted.
laurus> commit
Changes committed.
```

## 機能

- **行編集** — 矢印キー、Home/End キー、および標準的な readline ショートカット
- **履歴** — 上下矢印キーで以前のコマンドを呼び出し
- **Ctrl+C / Ctrl+D** — REPL を正常に終了
