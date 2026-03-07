# 拡張性

Laurusはコアコンポーネントにトレイトベースの抽象化を採用しています。これらのトレイトを実装することで、カスタムAnalyzer、Embedder、およびStorageバックエンドを提供できます。

## カスタムAnalyzer

`Analyzer` トレイトを実装して、カスタムテキスト解析パイプラインを作成します。

```rust
use laurus::analysis::analyzer::analyzer::Analyzer;
use laurus::analysis::token::{Token, TokenStream};
use laurus::Result;

#[derive(Debug)]
struct ReverseAnalyzer;

impl Analyzer for ReverseAnalyzer {
    fn analyze(&self, text: &str) -> Result<TokenStream> {
        let tokens: Vec<Token> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, word)| Token {
                text: word.chars().rev().collect(),
                position: i,
                ..Default::default()
            })
            .collect();
        Ok(Box::new(tokens.into_iter()))
    }

    fn name(&self) -> &str {
        "reverse"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### 必須メソッド

| メソッド | 説明 |
| :--- | :--- |
| `analyze(&self, text: &str) -> Result<TokenStream>` | テキストをトークンストリームに変換 |
| `name(&self) -> &str` | このAnalyzerの一意な識別子を返す |
| `as_any(&self) -> &dyn Any` | 具象型へのダウンキャストを可能にする |

### カスタムAnalyzerの使用

Analyzerを `EngineBuilder` に渡します。

```rust
use std::sync::Arc;

let analyzer = Arc::new(ReverseAnalyzer);
let engine = Engine::builder(storage, schema)
    .analyzer(analyzer)
    .build()
    .await?;
```

フィールドごとのAnalyzerには `PerFieldAnalyzer` でラップします。

```rust
use laurus::analysis::analyzer::per_field::PerFieldAnalyzer;
use laurus::analysis::analyzer::standard::StandardAnalyzer;

let mut per_field = PerFieldAnalyzer::new(Arc::new(StandardAnalyzer::new()?));
per_field.add_analyzer("custom_field", Arc::new(ReverseAnalyzer));

let engine = Engine::builder(storage, schema)
    .analyzer(Arc::new(per_field))
    .build()
    .await?;
```

## カスタムEmbedder

`Embedder` トレイトを実装して、独自のベクトルEmbeddingモデルを統合します。

```rust
use async_trait::async_trait;
use laurus::embedding::embedder::{Embedder, EmbedInput, EmbedInputType};
use laurus::vector::core::vector::Vector;
use laurus::{LaurusError, Result};

#[derive(Debug)]
struct MyEmbedder {
    dimension: usize,
}

#[async_trait]
impl Embedder for MyEmbedder {
    async fn embed(&self, input: &EmbedInput<'_>) -> Result<Vector> {
        match input {
            EmbedInput::Text(text) => {
                // Embeddingロジックをここに記述
                let vector = vec![0.0f32; self.dimension];
                Ok(Vector::new(vector))
            }
            _ => Err(LaurusError::invalid_argument(
                "this embedder only supports text input",
            )),
        }
    }

    fn supported_input_types(&self) -> Vec<EmbedInputType> {
        vec![EmbedInputType::Text]
    }

    fn name(&self) -> &str {
        "my-embedder"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### 必須メソッド

| メソッド | 説明 |
| :--- | :--- |
| `async embed(&self, input: &EmbedInput) -> Result<Vector>` | 指定された入力に対するEmbeddingベクトルを生成 |
| `supported_input_types(&self) -> Vec<EmbedInputType>` | サポートする入力タイプを宣言（`Text`、`Image`） |
| `as_any(&self) -> &dyn Any` | ダウンキャストを可能にする |

### オプションメソッド

| メソッド | デフォルト | 説明 |
| :--- | :--- | :--- |
| `async embed_batch(&self, inputs) -> Result<Vec<Vector>>` | `embed` への逐次呼び出し | バッチ最適化のためにオーバーライド |
| `name(&self) -> &str` | `"unknown"` | ログ出力用の識別子 |
| `supports(&self, input_type) -> bool` | `supported_input_types` をチェック | 入力タイプのサポート確認 |
| `supports_text() -> bool` | `Text` を確認 | テキストサポートの簡略確認 |
| `supports_image() -> bool` | `Image` を確認 | 画像サポートの簡略確認 |
| `is_multimodal() -> bool` | テキストと画像の両方 | マルチモーダル確認 |

### カスタムEmbedderの使用

```rust
let embedder = Arc::new(MyEmbedder { dimension: 384 });
let engine = Engine::builder(storage, schema)
    .embedder(embedder)
    .build()
    .await?;
```

フィールドごとのEmbedderには `PerFieldEmbedder` でラップします。

```rust
use laurus::embedding::per_field::PerFieldEmbedder;

let mut per_field = PerFieldEmbedder::new(Arc::new(MyEmbedder { dimension: 384 }));
per_field.add_embedder("image_vec", Arc::new(ClipEmbedder::new()?));

let engine = Engine::builder(storage, schema)
    .embedder(Arc::new(per_field))
    .build()
    .await?;
```

## カスタムStorage

`Storage` トレイトを実装して、新しいストレージバックエンドを追加します。

```rust
use laurus::storage::{Storage, StorageInput, StorageOutput, LoadingMode, FileMetadata};
use laurus::Result;

#[derive(Debug)]
struct S3Storage {
    bucket: String,
    prefix: String,
}

impl Storage for S3Storage {
    fn loading_mode(&self) -> LoadingMode {
        LoadingMode::Eager  // S3は完全なダウンロードが必要
    }

    fn open_input(&self, name: &str) -> Result<Box<dyn StorageInput>> {
        // S3からダウンロードしてリーダーを返す
        todo!()
    }

    fn create_output(&self, name: &str) -> Result<Box<dyn StorageOutput>> {
        // S3へのアップロードストリームを作成
        todo!()
    }

    fn create_output_append(&self, name: &str) -> Result<Box<dyn StorageOutput>> {
        todo!()
    }

    fn file_exists(&self, name: &str) -> bool {
        todo!()
    }

    fn delete_file(&self, name: &str) -> Result<()> {
        todo!()
    }

    fn list_files(&self) -> Result<Vec<String>> {
        todo!()
    }

    fn file_size(&self, name: &str) -> Result<u64> {
        todo!()
    }

    fn metadata(&self, name: &str) -> Result<FileMetadata> {
        todo!()
    }

    fn rename_file(&self, old_name: &str, new_name: &str) -> Result<()> {
        todo!()
    }

    fn create_temp_output(&self, prefix: &str) -> Result<(String, Box<dyn StorageOutput>)> {
        todo!()
    }

    fn sync(&self) -> Result<()> {
        todo!()
    }

    fn close(&mut self) -> Result<()> {
        todo!()
    }
}
```

### 必須メソッド

| メソッド | 説明 |
| :--- | :--- |
| `open_input(name) -> Result<Box<dyn StorageInput>>` | ファイルを読み取り用にオープン |
| `create_output(name) -> Result<Box<dyn StorageOutput>>` | ファイルを書き込み用に作成 |
| `create_output_append(name) -> Result<Box<dyn StorageOutput>>` | ファイルを追記用にオープン |
| `file_exists(name) -> bool` | ファイルの存在を確認 |
| `delete_file(name) -> Result<()>` | ファイルを削除 |
| `list_files() -> Result<Vec<String>>` | すべてのファイルを一覧表示 |
| `file_size(name) -> Result<u64>` | ファイルサイズをバイト単位で取得 |
| `metadata(name) -> Result<FileMetadata>` | ファイルのメタデータを取得 |
| `rename_file(old, new) -> Result<()>` | ファイル名を変更 |
| `create_temp_output(prefix) -> Result<(String, Box<dyn StorageOutput>)>` | 一時ファイルを作成 |
| `sync() -> Result<()>` | 保留中の書き込みをすべてフラッシュ |
| `close(&mut self) -> Result<()>` | ストレージを閉じてリソースを解放 |

### オプションメソッド

| メソッド | デフォルト | 説明 |
| :--- | :--- | :--- |
| `loading_mode() -> LoadingMode` | `LoadingMode::Eager` | 推奨されるデータロードモード |

## スレッドセーフティ

3つのトレイトすべてが `Send + Sync` を要求します。つまり、実装はスレッド間で安全に共有できる必要があります。共有可能な可変状態には `Arc<Mutex<_>>` またはロックフリーデータ構造を使用してください。

## 次のステップ

- [エラーハンドリング](error_handling.md) -- カスタム実装でのエラー処理
- [テキスト解析](../concepts/analysis.md) -- 組み込みのAnalyzerとパイプラインコンポーネント
- [Embedding](../concepts/embedding.md) -- 組み込みのEmbedderオプション
- [Storage](../concepts/storage.md) -- 組み込みのStorageバックエンド
