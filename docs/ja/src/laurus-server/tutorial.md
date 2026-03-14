# ハンズオンチュートリアル

このチュートリアルでは、laurus-server を使った一連のワークフローを体験します。サーバーの起動、インデックスの作成、ドキュメントの登録、検索、更新、削除を順を追って説明します。すべての操作は HTTP Gateway 経由の `curl` コマンドで行います。

## 前提条件

- laurus CLI がインストール済み（[インストール](../getting_started/installation.md) を参照）
- `curl` が利用可能

## Step 1: サーバーの起動

HTTP Gateway を有効にして laurus-server を起動します:

```bash
laurus --data-dir ./tutorial_data serve --http-port 8080
```

gRPC サーバー（ポート 50051）と HTTP Gateway（ポート 8080）が起動したことを示すログが表示されます。

サーバーが正常に動作しているか確認します:

```bash
curl http://localhost:8080/v1/health
```

期待されるレスポンス:

```json
{"status":"SERVING_STATUS_SERVING"}
```

## Step 2: インデックスの作成

Lexical 検索用のテキストフィールドを含むスキーマでインデックスを作成します:

```bash
curl -X POST http://localhost:8080/v1/index \
  -H 'Content-Type: application/json' \
  -d '{
    "schema": {
      "fields": {
        "title": {"text": {"indexed": true, "stored": true, "term_vectors": false}},
        "body": {"text": {"indexed": true, "stored": true, "term_vectors": false}},
        "category": {"text": {"indexed": true, "stored": true, "term_vectors": false}}
      },
      "default_fields": ["title", "body"]
    }
  }'
```

3 つのテキストフィールドを持つインデックスが作成されます。`default_fields` を設定することで、フィールド指定なしのクエリは `title` と `body` の両方を検索します。

インデックスが作成されたことを確認します:

```bash
curl http://localhost:8080/v1/index
```

期待されるレスポンス:

```json
{"documentCount":"0","vectorFields":{}}
```

## Step 3: ドキュメントの登録

ドキュメントをインデックスに追加します。`PUT` を使って ID 指定でドキュメントを登録します:

```bash
curl -X PUT http://localhost:8080/v1/documents/doc001 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Introduction to Rust Programming",
        "body": "Rust is a modern systems programming language that focuses on safety, speed, and concurrency.",
        "category": "programming"
      }
    }
  }'
```

```bash
curl -X PUT http://localhost:8080/v1/documents/doc002 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Web Development with Rust",
        "body": "Building web applications with Rust has become increasingly popular. Frameworks like Actix and Rocket make it easy to create fast and secure web services.",
        "category": "web-development"
      }
    }
  }'
```

```bash
curl -X PUT http://localhost:8080/v1/documents/doc003 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Python for Data Science",
        "body": "Python is the most popular language for data science and machine learning. Libraries like NumPy and Pandas provide powerful tools for data analysis.",
        "category": "data-science"
      }
    }
  }'
```

## Step 4: 変更のコミット

ドキュメントはコミットするまで検索対象になりません。変更をコミットします:

```bash
curl -X POST http://localhost:8080/v1/commit
```

## Step 5: ドキュメントの検索

### 基本的な検索

"rust" を含むドキュメントを検索します:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "rust", "limit": 10}'
```

デフォルトフィールド（`title` と `body`）が検索されます。`doc001` と `doc002` が返されます。

### フィールド指定検索

`title` フィールドのみを検索します:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "title:python", "limit": 10}'
```

`doc003` のみが返されます。

### カテゴリ検索

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "category:programming", "limit": 10}'
```

`doc001` のみが返されます。

### ブーリアンクエリ

`AND`、`OR`、`NOT` で条件を組み合わせます:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "rust AND web", "limit": 10}'
```

"rust" と "web" の両方を含む `doc002` のみが返されます。

### フィールドブースト

`title` フィールドのスコアを引き上げて、タイトルの一致を優先します:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "rust",
    "limit": 10,
    "field_boosts": {"title": 2.0}
  }'
```

## Step 6: ドキュメントの取得

ID を指定して特定のドキュメントを取得します:

```bash
curl http://localhost:8080/v1/documents/doc001
```

期待されるレスポンス:

```json
{
  "documents": [
    {
      "fields": {
        "title": "Introduction to Rust Programming",
        "body": "Rust is a modern systems programming language that focuses on safety, speed, and concurrency.",
        "category": "programming"
      }
    }
  ]
}
```

## Step 7: ドキュメントの更新

同じ ID で `PUT` を実行するとドキュメント全体が置き換わります:

```bash
curl -X PUT http://localhost:8080/v1/documents/doc001 \
  -H 'Content-Type: application/json' \
  -d '{
    "document": {
      "fields": {
        "title": "Introduction to Rust Programming",
        "body": "Rust is a modern systems programming language that focuses on safety, speed, and concurrency. It provides memory safety without garbage collection.",
        "category": "programming"
      }
    }
  }'
```

コミットして確認します:

```bash
curl -X POST http://localhost:8080/v1/commit
curl http://localhost:8080/v1/documents/doc001
```

更新された `body` の内容が反映されています。

## Step 8: ドキュメントの削除

ID を指定してドキュメントを削除します:

```bash
curl -X DELETE http://localhost:8080/v1/documents/doc003
```

コミットして反映させます:

```bash
curl -X POST http://localhost:8080/v1/commit
```

ドキュメントが削除されたことを確認します:

```bash
curl http://localhost:8080/v1/documents/doc003
```

期待されるレスポンス:

```json
{"documents":[]}
```

検索結果にも表示されなくなります:

```bash
curl -X POST http://localhost:8080/v1/search \
  -H 'Content-Type: application/json' \
  -d '{"query": "python", "limit": 10}'
```

結果は返されません。

## Step 9: インデックス統計の確認

現在のインデックス統計を確認します:

```bash
curl http://localhost:8080/v1/index
```

`documentCount` は削除後の残りのドキュメント数を反映しています。

## Step 10: クリーンアップ

`Ctrl+C` でサーバーを停止します。サーバーはグレースフルシャットダウンを行い、保留中の変更をコミットしてから終了します。

チュートリアルで作成したデータを削除します:

```bash
rm -rf ./tutorial_data
```

## 次のステップ

- [ベクトル検索とハイブリッド検索](../concepts/search/hybrid_search.md)で意味的な類似検索を試す
- [gRPC API リファレンス](grpc_api.md)で API 仕様の詳細を確認する
- [設定](configuration.md)で本番環境向けの設定を行う
- `grpcurl` や gRPC クライアントライブラリを使ったプログラムからのアクセスについては[はじめに](getting_started.md)を参照
