# MCP ツールリファレンス

laurus MCP サーバーは以下のツールを公開しています。

## connect

実行中の laurus-server gRPC エンドポイントに接続します。`--endpoint` フラグなしでサーバーを起動した場合や、実行時に別の laurus-server に切り替える場合に、他のツールを使用する前にこのツールを呼び出してください。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `endpoint` | string | はい | gRPC エンドポイント URL（例: `http://localhost:50051`） |

### 例

```text
Tool: connect
endpoint: "http://localhost:50051"
```

結果: `Connected to laurus-server at http://localhost:50051.`

---

## create_index

指定されたスキーマで新しい検索インデックスを作成します。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `schema_json` | string | はい | JSON 文字列としてのスキーマ定義 |

### スキーマ JSON フォーマット

FieldOption は serde の externally-tagged 表現を使用します（バリアント名がキーになります）：

```json
{
  "fields": {
    "title":     { "Text":    { "indexed": true, "stored": true } },
    "body":      { "Text":    {} },
    "score":     { "Float":   {} },
    "count":     { "Integer": {} },
    "active":    { "Boolean": {} },
    "created":   { "DateTime": {} },
    "embedding": { "Hnsw":    { "dimension": 384 } }
  }
}
```

### 例

```text
Tool: create_index
schema_json: {"fields": {"title": {"Text": {}}, "body": {"Text": {}}}}
```

結果: `Index created successfully at /path/to/index.`

---

## get_stats

現在の検索インデックスの統計情報（ドキュメント数、ベクトルフィールド情報など）を取得します。

### パラメーター

なし。

### 結果

```json
{
  "document_count": 42,
  "vector_fields": ["embedding"]
}
```

---

## get_schema

現在のインデックスのスキーマ（全フィールド定義と設定）を取得します。

### パラメーター

なし。

### 結果

```json
{
  "fields": {
    "title": { "Text": { "indexed": true, "stored": true } },
    "body": { "Text": {} },
    "embedding": { "Hnsw": { "dimension": 384 } }
  },
  "default_fields": ["title", "body"]
}
```

---

## put_document

インデックスにドキュメントを上書き（upsert）します。同じ ID のドキュメントが既に存在する場合、全チャンクが削除されてから新しいドキュメントがインデックスされます。ドキュメント追加後は `commit` を呼び出してください。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `id` | string | はい | 外部ドキュメント識別子 |
| `document` | object | はい | JSON オブジェクトとしてのドキュメントフィールド |

### 例

```text
Tool: put_document
id: "doc-1"
document: {"title": "Hello World", "body": "これはテストドキュメントです。"}
```

結果: `Document 'doc-1' put (upserted). Call commit to persist changes.`

---

## add_document

インデックスにドキュメントを新しいチャンクとして追加します。`put_document` とは異なり、同じ ID の既存ドキュメントを削除せずに追記します。大きなドキュメントをチャンクに分割する際に便利です。ドキュメント追加後は `commit` を呼び出してください。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `id` | string | はい | 外部ドキュメント識別子 |
| `document` | object | はい | JSON オブジェクトとしてのドキュメントフィールド |

### 例

```text
Tool: add_document
id: "doc-1"
document: {"title": "Hello World - Part 2", "body": "これは続きです。"}
```

結果: `Document 'doc-1' added as chunk. Call commit to persist changes.`

---

## get_documents

外部 ID で全ドキュメント（チャンクを含む）を取得します。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `id` | string | はい | 外部ドキュメント識別子 |

### 結果

```json
{
  "id": "doc-1",
  "documents": [
    { "title": "Hello World", "body": "これはテストドキュメントです。" }
  ]
}
```

---

## delete_documents

外部 ID で全ドキュメント（チャンクを含む）を削除します。削除後は `commit` を呼び出してください。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `id` | string | はい | 外部ドキュメント識別子 |

結果: `Documents 'doc-1' deleted. Call commit to persist changes.`

---

## commit

保留中の変更をディスクにコミットします。変更を検索可能かつ永続的にするため、`put_document`、`add_document`、または `delete_documents` の後に必ず呼び出してください。

### パラメーター

なし。

結果: `Changes committed successfully.`

---

## add_field

インデックスにフィールドを追加します。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `name` | string | はい | フィールド名 |
| `field_option_json` | string | はい | JSON 形式のフィールド設定 |

### 例

```json
{
  "name": "category",
  "field_option_json": "{\"Text\": {\"indexed\": true, \"stored\": true}}"
}
```

---

## delete_field

インデックスからフィールドを削除します。既にインデックスされたデータは残りますが、削除されたフィールドにはアクセスできなくなります。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `name` | string | はい | 削除するフィールド名 |

### 例

```text
Tool: delete_field
name: "category"
```

結果: `Field 'category' deleted.`

---

## search

laurus クエリ DSL を使用してドキュメントを検索します。

### パラメーター

| 名前 | 型 | 必須 | 説明 |
| :--- | :--- | :--- | :--- |
| `query` | string | はい | laurus クエリ DSL による検索クエリ |
| `limit` | integer | いいえ | 最大結果数（デフォルト: 10） |
| `offset` | integer | いいえ | ページネーション用スキップ数（デフォルト: 0） |

### クエリ DSL の例

| クエリ | 説明 |
| :--- | :--- |
| `hello` | デフォルトフィールド全体のターム検索 |
| `title:hello` | フィールド指定のターム検索 |
| `title:hello AND body:world` | ブール AND |
| `"exact phrase"` | フレーズ検索 |
| `roam~2` | ファジー検索（編集距離 2） |
| `count:[1 TO 10]` | 範囲検索 |
| `title:helo~1` | フィールド指定のファジー検索 |

### 結果

```json
{
  "total": 2,
  "results": [
    {
      "id": "doc-1",
      "score": 3.14,
      "document": { "title": "Hello World", "body": "..." }
    },
    {
      "id": "doc-2",
      "score": 1.57,
      "document": { "title": "Hello Again", "body": "..." }
    }
  ]
}
```

---

## 典型的なワークフロー

```text
1. connect          → 実行中の laurus-server に接続
2. create_index     → スキーマを定義（インデックスが存在しない場合）
3. add_field        → フィールドを追加（必要に応じて）
   delete_field     → フィールドを削除（必要に応じて）
4. put_document     → ドキュメントを上書き（必要に応じて繰り返し）
   add_document     → ドキュメントチャンクを追記（必要に応じて）
5. commit           → 変更をディスクに永続化
6. search           → インデックスを検索
7. get_documents    → ID でドキュメントを取得
8. delete_documents → ドキュメントを削除
9. commit           → 変更を永続化
```
