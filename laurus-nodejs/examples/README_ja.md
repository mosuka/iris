# laurus-nodejs サンプル集

このディレクトリには `laurus-nodejs` Node.js バインディングの
実行可能なサンプルが含まれています。

## 前提条件

- Rust ツールチェーン（`rustup` — <https://rustup.rs>）
- Node.js 18 以上（<https://nodejs.org>）
- npm

## セットアップ

すべてのサンプルは `laurus-nodejs/` ディレクトリで
ネイティブモジュールをビルドした後に動かせます。

```bash
cd laurus-nodejs

# 依存パッケージのインストール
npm install

# ネイティブモジュールのビルド（デバッグモード、ビルドが速い）
npm run build:debug
```

## サンプル一覧

### 基本サンプル（追加依存なし）

一度ビルドすれば、以下のサンプルをすべて実行できます。

| サンプル | 説明 |
| :--- | :--- |
| [quickstart.mjs](quickstart.mjs) | 最小構成の検索: インデックス・検索・統計 |
| [lexical-search.mjs](lexical-search.mjs) | Lexical クエリタイプ: Term、Phrase、Fuzzy、Wildcard、DSL |
| [同義語展開の例](../README_ja.md#テキスト解析) | 同義語展開（README 参照） |

```bash
node examples/quickstart.mjs
node examples/lexical-search.mjs
```

---

### ベクトル検索 — 事前計算済みベクトル

事前計算済みの埋め込みベクトルを HNSW インデックスに
直接渡す方式です。
外部の埋め込みライブラリは不要です。

| サンプル | 説明 |
| :--- | :--- |
| [vector-search.mjs](vector-search.mjs) | 事前計算済みベクトルによる類似度検索 |

```bash
node examples/vector-search.mjs
```

---

### ハイブリッド検索

Lexical 検索と Vector 検索を RRF（Reciprocal Rank Fusion）
または WeightedSum で融合します。

| サンプル | 説明 |
| :--- | :--- |
| [hybrid-search.mjs](hybrid-search.mjs) | RRF・WeightedSum による Lexical + Vector ハイブリッド検索 |

```bash
node examples/hybrid-search.mjs
```

---

## リリースビルド

本番環境向けのパフォーマンスが必要な場合は、
リリースプロファイルでビルドします。

```bash
npm run build
```
