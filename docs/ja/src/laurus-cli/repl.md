# REPL（対話モード）

REPL は、毎回 `laurus` コマンドをフルで入力することなく、インデックスを操作できる対話型セッションを提供します。

## REPL の起動

```bash
laurus --data-dir ./my_index repl
```

```text
Laurus REPL (type 'help' for commands, 'quit' to exit)
laurus>
```

REPL は起動時にインデックスを開き、セッション中ロードされた状態を維持します。

## 利用可能なコマンド

| コマンド | 説明 |
| :--- | :--- |
| `search <query>` | インデックスを検索（最大10件の結果を返す） |
| `doc add <id> <json>` | ドキュメントを追加 |
| `doc get <id>` | ID でドキュメントを取得 |
| `doc delete <id>` | ID でドキュメントを削除 |
| `commit` | 保留中の変更をコミット |
| `stats` | インデックスの統計情報を表示 |
| `help` | 利用可能なコマンドを表示 |
| `quit` / `exit` | REPL を終了 |

search コマンドは常に最大10件の結果を返します。REPL ではこの上限は現在変更できません。

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

### ドキュメントの追加とコミット

```text
laurus> doc add doc4 {"title":"New Document","body":"Some content here."}
Document 'doc4' added.
laurus> commit
Changes committed.
```

### ドキュメントの取得

```text
laurus> doc get doc4
╭──────┬───────────────────────────────────────────────╮
│ ID   │ Fields                                        │
├──────┼───────────────────────────────────────────────┤
│ doc4 │ body: Some content here., title: New Document │
╰──────┴───────────────────────────────────────────────╯
```

### ドキュメントの削除

```text
laurus> doc delete doc4
Document 'doc4' deleted.
laurus> commit
Changes committed.
```

### 統計情報の表示

```text
laurus> stats
Document count: 3
```

## 機能

- **行編集** — 矢印キー、Home/End キー、および標準的な readline ショートカット
- **履歴** — 上下矢印キーで以前のコマンドを呼び出し
- **Ctrl+C / Ctrl+D** — REPL を正常に終了
