# laurus-php サンプル

このディレクトリには `laurus` PHP バインディングの実行可能なサンプルが含まれています。

## 前提条件

- Rust ツールチェイン (`rustup` -- <https://rustup.rs>)
- PHP 8.1+ (<https://www.php.net>)
- Composer (<https://getcomposer.org>) (テスト実行時のみ)

## セットアップ

すべてのサンプルは、ネイティブ拡張をビルドした後に
`laurus-php/` ディレクトリから実行してください。

```bash
cd laurus-php

# PHP 拡張をビルド（リリースモード）
cargo build --release
```

## サンプル

### 基本サンプル（追加依存なし）

一度ビルドすれば、以下のサンプルを実行できます:

| サンプル | 説明 |
| :--- | :--- |
| [quickstart.php](quickstart.php) | 最小限の全文検索: インデックス作成、検索、更新 |
| [lexical_search.php](lexical_search.php) | 全レキシカルクエリ型: Term, Phrase, Fuzzy, Wildcard, NumericRange, Geo, Boolean, Span |
| [synonym_graph_filter.php](synonym_graph_filter.php) | 解析パイプラインでの同義語展開 |

```bash
php -d extension=target/release/liblaurus_php.so examples/quickstart.php
php -d extension=target/release/liblaurus_php.so examples/lexical_search.php
php -d extension=target/release/liblaurus_php.so examples/synonym_graph_filter.php
```

> **ヒント:** 毎回 `-d extension=...` を指定する代わりに、`php.ini` に
> `extension=liblaurus_php.so` を追加し、`.so` ファイルを PHP 拡張ディレクトリ
> (`php -i | grep extension_dir`) にコピーしてください。

---

### ベクトル検索 -- 組み込みエンベッダー

laurus 組み込みの `CandleBert` エンベッダー（[Candle](https://github.com/huggingface/candle) 経由）を使用します。
テキストはインデックス時とクエリ時に Rust エンジンが自動的にベクトル化します。
**外部のエンベディングライブラリは不要です。**

`embeddings-candle` フィーチャーを有効にしてビルドしてください:

```bash
cargo build --release --features embeddings-candle
```

| サンプル | 説明 |
| :--- | :--- |
| [vector_search.php](vector_search.php) | laurus 組み込み BERT エンベッダーによるセマンティック類似検索 |
| [hybrid_search.php](hybrid_search.php) | RRF/WeightedSum フュージョンによるレキシカル + ベクトルハイブリッド検索 |
| [search_app.php](search_app.php) | Lexical / Vector / Hybrid モード切替付きブラウザベース検索 UI |

```bash
php -d extension=target/release/liblaurus_php.so examples/vector_search.php
php -d extension=target/release/liblaurus_php.so examples/hybrid_search.php
```

ウェブベースの検索アプリはローカルサーバーを起動します:

```bash
php -d extension=target/release/liblaurus_php.so -S localhost:8080 examples/search_app.php
```

ブラウザで <http://localhost:8080> を開いてください。

> **注意:** 初回実行時に Hugging Face Hub からモデルの重みがダウンロードされます
> (`sentence-transformers/all-MiniLM-L6-v2`, 約 90 MB)。2 回目以降はローカルキャッシュが使用されます。

---

### ベクトル検索 -- 外部エンベッダー

`VectorQuery` 経由で事前計算済みベクトルを使用します。スキーマにエンベッダーを登録せず、
呼び出し側が外部でエンベディングを管理します。

追加フィーチャーなしでビルドしてください（標準リリースビルド）:

```bash
cargo build --release
```

| サンプル | 説明 |
| :--- | :--- |
| [external_embedder.php](external_embedder.php) | ランダムフォールバックベクトルによる事前計算済みベクトル検索（外部依存なし） |
| [search_with_openai.php](search_with_openai.php) | OpenAI Embeddings API（raw curl）を使用したリアルベクトル検索 |
| [multimodal_search.php](multimodal_search.php) | マルチモーダル検索: 画像バイト + ベクトル埋め込みを保存し、テキストと画像を横断検索 |

```bash
php -d extension=target/release/liblaurus_php.so examples/external_embedder.php
php -d extension=target/release/liblaurus_php.so examples/multimodal_search.php
```

OpenAI サンプルは API キーが必要です:

```bash
export OPENAI_API_KEY=your-api-key-here
php -d extension=target/release/liblaurus_php.so examples/search_with_openai.php
```
