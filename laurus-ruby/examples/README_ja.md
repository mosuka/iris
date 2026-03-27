# laurus-ruby サンプル集

このディレクトリには `laurus` Ruby バインディングの
実行可能なサンプルが含まれています。

## 前提条件

- Rust ツールチェーン（`rustup` — <https://rustup.rs>）
- Ruby 3.2 以上（<https://www.ruby-lang.org>）
- Bundler

## セットアップ

すべてのサンプルは `laurus-ruby/` ディレクトリで
ネイティブ拡張をビルドした後に動かせます。

```bash
cd laurus-ruby

# 依存パッケージのインストール
bundle install

# ネイティブ拡張のビルド
bundle exec rake compile
```

## サンプル一覧

### 基本サンプル（追加依存なし）

一度ビルドすれば、以下のサンプルをすべて実行できます。

| サンプル | 説明 |
| :--- | :--- |
| [quickstart.rb](quickstart.rb) | 最小構成の全文検索: インデックス・検索・更新 |
| [lexical_search.rb](lexical_search.rb) | 全 Lexical クエリタイプ: Term、Phrase、Fuzzy、Wildcard、NumericRange、Geo、Boolean、Span |
| [synonym_graph_filter.rb](synonym_graph_filter.rb) | 解析パイプラインでの同義語展開 |

```bash
ruby -Ilib examples/quickstart.rb
ruby -Ilib examples/lexical_search.rb
ruby -Ilib examples/synonym_graph_filter.rb
```

---

### ベクトル検索 — 組み込み Embedder

laurus 内蔵の `CandleBert` Embedder（[Candle](https://github.com/huggingface/candle) 経由）を使用します。
テキストのベクトル化はインデックス時・検索時に Rust エンジンが自動で行います。
**外部の埋め込みライブラリは不要です。**

`embeddings-candle` フィーチャーフラグを付けてビルドします。

```bash
bundle exec rake compile  # Cargo.toml に embeddings-candle フィーチャーを含めること
```

| サンプル | 説明 |
| :--- | :--- |
| [vector_search.rb](vector_search.rb) | 組み込み BERT Embedder によるセマンティック類似度検索 |
| [hybrid_search.rb](hybrid_search.rb) | RRF・WeightedSum フュージョンによる Lexical + Vector ハイブリッド検索 |

```bash
ruby -Ilib examples/vector_search.rb
ruby -Ilib examples/hybrid_search.rb
```

> **注意:** 初回実行時に Hugging Face Hub からモデルをダウンロードします
> （`sentence-transformers/all-MiniLM-L6-v2`、約 90 MB）。
> 2回目以降はローカルキャッシュが使われます。

---

## リリースビルド

本番環境向けのパフォーマンスが必要な場合は、
リリースプロファイルでビルドします。

```bash
bundle exec rake compile:release
```
