# スペル修正

Laurusにはスペル修正システムが組み込まれており、誤入力されたクエリ単語の修正候補を提案し、「もしかして？（Did you mean?）」機能を提供します。

## 概要

スペル修正器は、編集距離（Levenshtein距離（Levenshtein Distance））と単語頻度データを組み合わせて修正候補を提案します。以下の機能をサポートしています。

- **単語レベルの候補提案** -- 個々の誤入力単語を修正
- **自動修正** -- 高信頼度の修正を自動的に適用
- **「もしかして？」** -- ユーザーに代替クエリを提案
- **クエリ学習** -- ユーザーのクエリから学習して候補を改善
- **カスタム辞書** -- 独自の単語リストを使用

## 基本的な使い方

### SpellingCorrector

```rust
use laurus::spelling::corrector::SpellingCorrector;

// 組み込みの英語辞書で修正器を作成
let mut corrector = SpellingCorrector::new();

// クエリを修正
let result = corrector.correct("programing langauge");

// 候補が利用可能か確認
if result.has_suggestions() {
    for (word, suggestions) in &result.word_suggestions {
        println!("'{}' -> {:?}", word, suggestions);
    }
}

// 最良の修正済みクエリを取得
if let Some(corrected) = result.query() {
    println!("Corrected: {}", corrected);
}
```

### 「もしかして？」

`DidYouMean` ラッパーは検索UIに適した高レベルのインターフェースを提供します。

```rust
use laurus::spelling::corrector::{SpellingCorrector, DidYouMean};

let corrector = SpellingCorrector::new();
let mut did_you_mean = DidYouMean::new(corrector);

if let Some(suggestion) = did_you_mean.suggest("programing") {
    println!("Did you mean: {}?", suggestion);
}
```

## 設定

`CorrectorConfig` を使用して動作をカスタマイズできます。

```rust
use laurus::spelling::corrector::{CorrectorConfig, SpellingCorrector};

let config = CorrectorConfig {
    max_distance: 2,              // 最大編集距離（デフォルト: 2）
    max_suggestions: 5,           // 単語あたりの最大候補数（デフォルト: 5）
    min_frequency: 1,             // 最小単語頻度しきい値（デフォルト: 1）
    auto_correct: false,          // 自動修正を有効化（デフォルト: false）
    auto_correct_threshold: 0.8,  // 自動修正の信頼度しきい値（デフォルト: 0.8）
    use_index_terms: true,        // インデックスの単語を辞書として使用（デフォルト: true）
    learn_from_queries: true,     // ユーザーのクエリから学習（デフォルト: true）
};
```

### 設定オプション

| オプション | 型 | デフォルト | 説明 |
| :--- | :--- | :--- | :--- |
| `max_distance` | `usize` | `2` | 候補提案のための最大Levenshtein編集距離 |
| `max_suggestions` | `usize` | `5` | 単語あたりの最大候補数 |
| `min_frequency` | `u32` | `1` | 候補として提案されるために必要な辞書内の最小頻度 |
| `auto_correct` | `bool` | `false` | trueの場合、しきい値を超える修正を自動的に適用 |
| `auto_correct_threshold` | `f64` | `0.8` | 自動修正に必要な信頼度スコア（0.0--1.0） |
| `use_index_terms` | `bool` | `true` | 検索インデックスの単語を辞書として使用 |
| `learn_from_queries` | `bool` | `true` | ユーザーの検索クエリから新しい単語を学習 |

## CorrectionResult

`correct()` メソッドは詳細な情報を含む `CorrectionResult` を返します。

| フィールド | 型 | 説明 |
| :--- | :--- | :--- |
| `original` | `String` | 元のクエリ文字列 |
| `corrected` | `Option<String>` | 修正済みクエリ（自動修正が適用された場合） |
| `word_suggestions` | `HashMap<String, Vec<Suggestion>>` | 誤入力単語ごとにグループ化された候補 |
| `confidence` | `f64` | 全体の信頼度スコア（0.0--1.0） |
| `auto_corrected` | `bool` | 自動修正が適用されたかどうか |

### ヘルパーメソッド

| メソッド | 戻り値 | 説明 |
| :--- | :--- | :--- |
| `has_suggestions()` | `bool` | いずれかの単語に候補がある場合true |
| `best_suggestion()` | `Option<&Suggestion>` | 最もスコアの高い単一の候補 |
| `query()` | `Option<String>` | 修正が行われた場合の修正済みクエリ文字列 |
| `should_show_did_you_mean()` | `bool` | 「もしかして？」プロンプトを表示すべきかどうか |

## カスタム辞書

組み込みの英語辞書の代わりに独自の辞書を提供できます。

```rust
use laurus::spelling::corrector::SpellingCorrector;
use laurus::spelling::dictionary::SpellingDictionary;

// カスタム辞書を構築
let mut dictionary = SpellingDictionary::new();
dictionary.add_word("elasticsearch", 100);
dictionary.add_word("lucene", 80);
dictionary.add_word("laurus", 90);

let corrector = SpellingCorrector::with_dictionary(dictionary);
```

## インデックス単語からの学習

`use_index_terms` が有効な場合、修正器は検索インデックスの単語から学習できます。

```rust
let mut corrector = SpellingCorrector::new();

// インデックスの単語を修正器に提供
let index_terms = vec!["rust", "programming", "search", "engine"];
corrector.learn_from_terms(&index_terms);
```

これにより、ドメイン固有の語彙が組み込まれ、候補の品質が向上します。

## 統計情報

`stats()` で修正器の状態を監視できます。

```rust
let stats = corrector.stats();
println!("Dictionary words: {}", stats.dictionary_words);
println!("Total frequency: {}", stats.dictionary_total_frequency);
println!("Learned queries: {}", stats.queries_learned);
```

## 次のステップ

- [Lexical検索](../concepts/search/lexical_search.md) -- クエリタイプを使用した全文検索
- [Query DSL](../concepts/query_dsl.md) -- 人間が読みやすいクエリ構文
