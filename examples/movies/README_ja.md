# Movies サンプル

[Meilisearch movies データセット](https://github.com/meilisearch/datasets) の約 32,000 件の映画データをインデックスして検索するサンプルです。

## スキーマ

[schema.toml](schema.toml) で以下のフィールドを定義しています:

| フィールド | 型 | インデックス | 保存 | 説明 |
| --------- | ---- | ---------- | ---- | ---- |
| `title` | Text | あり | あり | 映画タイトル |
| `overview` | Text | あり | あり | あらすじ |
| `genres` | Text | あり | あり | カンマ区切りのジャンル一覧 |
| `poster` | Text | なし | あり | ポスター画像の URL |
| `release_date` | Integer | あり | あり | Unix タイムスタンプ |

デフォルト検索フィールド: `title`, `overview`

## 使い方

### 1. インデックスの作成

```bash
bash examples/movies/scripts/create_index.sh
```

release バイナリをビルドし、スキーマを使って `examples/movies/index/` に空のインデックスを作成します。

### 2. 映画データの投入

```bash
bash examples/movies/scripts/index_movies.sh
```

データセットの `movies.json` を読み込み、各レコードを laurus ドキュメントに変換して REPL にパイプで一括投入します。1,000 件ごとにコミットされます。

### 3. 検索例の実行

```bash
bash examples/movies/scripts/search_movies.sh
```

以下の検索例を実行します:

- `star wars` — デフォルトフィールドに対する全文検索
- `title:nemo` — フィールド指定検索
- `genres:comedy` — ジャンルで検索
- `overview:robot` — あらすじ内を検索
- JSON 形式での出力

### 手動検索

直接コマンドで検索することもできます:

```bash
./target/release/laurus --data-dir examples/movies/index search "title:matrix" --limit 10
```

対話モードで操作する場合:

```bash
./target/release/laurus --data-dir examples/movies/index repl
```

## ファイル構成

```text
examples/movies/
├── README.md
├── README_ja.md
├── schema.toml          # インデックスのスキーマ定義
├── scripts/
│   ├── create_index.sh  # インデックスの作成
│   ├── index_movies.sh  # データセットの投入
│   └── search_movies.sh # 検索例
└── index/               # 生成されるインデックスデータ（git 管理外）
```
