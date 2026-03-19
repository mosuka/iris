# Examples

このディレクトリには、laurus を使ったデータセットのインデックスと検索のサンプルスクリプトが含まれています。

## 前提条件

### データセット

サンプルでは [meilisearch/datasets](https://github.com/meilisearch/datasets) のデータセットを使用します。
laurus プロジェクトの隣にリポジトリをクローンしてください:

```bash
cd ..
git clone https://github.com/meilisearch/datasets.git
```

期待されるディレクトリ構成:

```text
parent/
├── datasets/       # meilisearch/datasets のクローン
│   └── datasets/
│       └── movies/
│           └── movies.json
└── laurus/         # このプロジェクト
    └── examples/
```

### ツール

- [jq](https://jqlang.org/) — インデックススクリプトで JSON データセットの解析に使用します。

## Movies

Meilisearch の movies データセットから約 32,000 件の映画をインデックスして検索します。

```bash
# 1. インデックスを作成
bash examples/movies/scripts/create_index.sh

# 2. 全映画をインデックス
bash examples/movies/scripts/index_movies.sh

# 3. 検索例を実行
bash examples/movies/scripts/search_movies.sh
```

スキーマ定義は [examples/movies/schema.toml](movies/schema.toml) を参照してください。
