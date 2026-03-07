# 検索（Search）

このセクションでは、インデキシングされたデータに対するクエリの実行方法を説明します。Laurus は 3 つの検索モードをサポートしており、それぞれ独立して使用することも、組み合わせて使用することもできます。

## トピック

### [Lexical 検索](search/lexical_search.md)

転置インデックスを使用したキーワードベースの検索について説明します。

- すべてのクエリタイプ: Term、Phrase、Boolean、Fuzzy、Wildcard、Range、Geo、Span
- BM25 スコアリングとフィールドブースト
- テキストベースのクエリのための Query DSL の使用方法

### [Vector 検索](search/vector_search.md)

ベクトルエンベディングを使用した意味的類似性検索について説明します。

- VectorSearchRequestBuilder API
- マルチフィールド Vector 検索とスコアモード
- フィルター付き Vector 検索

### [ハイブリッド検索](search/hybrid_search.md)

Lexical 検索と Vector 検索を組み合わせた、両方の長所を活かす検索について説明します。

- SearchRequestBuilder API
- フュージョンアルゴリズム（RRF、WeightedSum）
- フィルター付きハイブリッド検索
- offset/limit によるページネーション

スペル修正については、ライブラリセクションの [Spelling Correction](../laurus/spelling_correction.md) を参照してください。
