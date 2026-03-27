# Laurus WASM デモ

laurus-wasm を使用したブラウザ上での全文検索を実演する
シングルページアプリケーションです。

## 実行方法

1. WASM パッケージをビルドします:

   ```bash
   cd laurus-wasm
   wasm-pack build --target web --dev
   ```

2. HTTP サーバーでファイルを配信します（WASM は `file://` では動作しません）:

   ```bash
   # Python
   python3 -m http.server 8080

   # Node.js (npx)
   npx serve .
   ```

3. ブラウザで <http://localhost:8080/examples/> を開きます。

## デモの内容

- OPFS 永続化ストレージを使用した検索インデックスを作成
  （ページリロード後もデータが保持されます）
- 初回アクセス時に 8 件のサンプルドキュメントを投入。
  既存データが OPFS にある場合はロードのみ
- Transformers.js（all-MiniLM-L6-v2）による 384 次元セマンティック
  Embedding をコールバック Embedder 経由で自動生成
- 統合クエリ DSL 対応の検索ボックスを提供:
  - Lexical 検索: `rust`、`title:wasm`、`"memory safety"`
  - Vector 検索: `embedding:"how to make code faster"`、`embedding:python`
  - Hybrid 検索: `rust embedding:"systems programming"`
- 新しいドキュメントをインタラクティブに追加可能
- 関連度スコア付きの検索結果を表示
- すべての操作をコンソールパネルにログ出力
