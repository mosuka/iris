# 開発環境のセットアップ

このページでは `laurus-php` バインディングのローカル開発環境の構築、ビルド、テストスイートの実行方法について説明します。

## 前提条件

- **Rust** 1.85 以降（Cargo 含む）
- **PHP** 8.1 以降（開発ヘッダー付き: `php-dev` / `php-devel`）
- **Composer**（依存関係管理用）
- リポジトリがローカルにクローンされていること

```bash
git clone https://github.com/mosuka/laurus.git
cd laurus
```

## ビルド

### 開発ビルド

Rust ネイティブ拡張をデバッグモードでコンパイルします。Rust ソースを変更した場合は再実行してください。

```bash
cd laurus-php
cargo build
```

ビルド成果物は `../target/debug/liblaurus_php.so` に生成されます。

### リリースビルド

```bash
cd laurus-php
cargo build --release
```

ビルド成果物は `../target/release/liblaurus_php.so` に生成されます。

### ビルドの確認

```bash
php -d extension=../target/release/liblaurus_php.so -r "
use Laurus\Index;
\$index = new Index();
print_r(\$index->stats());
"
# Array ( [document_count] => 0 [vector_fields] => Array ( ) )
```

## テスト

テストは [PHPUnit](https://phpunit.de/) を使用しており、`tests/` ディレクトリにあります。

```bash
# テスト依存関係をインストール
composer install

# 全テスト実行
php -d extension=../target/release/liblaurus_php.so vendor/bin/phpunit tests/
```

特定のテストファイルを実行する場合：

```bash
php -d extension=../target/release/liblaurus_php.so vendor/bin/phpunit tests/LaurusTest.php
```

## Lint とフォーマット

```bash
# Rust lint（Clippy）
cargo clippy -p laurus-php -- -D warnings

# Rust フォーマットチェック
cargo fmt -p laurus-php --check

# フォーマット適用
cargo fmt -p laurus-php
```

## クリーンアップ

```bash
# ビルド成果物を削除
cargo clean

# Composer 依存関係を削除
rm -rf vendor/
```

## プロジェクト構成

```text
laurus-php/
├── Cargo.toml          # Rust クレートマニフェスト
├── composer.json       # Composer パッケージ定義
├── composer.lock       # ロックされた依存関係バージョン
├── src/                # Rust ソース（ext-php-rs バインディング）
│   ├── lib.rs          # モジュール登録
│   ├── index.rs        # Index クラス
│   ├── schema.rs       # Schema クラス
│   ├── query.rs        # クエリクラス
│   ├── search.rs       # SearchRequest / SearchResult / Fusion
│   ├── analysis.rs     # Tokenizer / Filter / Token
│   ├── convert.rs      # PHP <-> DataValue 変換
│   └── errors.rs       # エラーマッピング
├── tests/              # PHPUnit テスト
│   └── LaurusTest.php
└── examples/           # 実行可能な PHP サンプル
```
