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

- `title` と `body` フィールドを持つインメモリ検索インデックスを作成
- Rust、WASM、検索に関する 5 件のサンプルドキュメントを読み込み
- DSL クエリ対応の検索ボックスを提供（例: `"title:rust"`、`"browser programming"`）
- 新しいドキュメントをインタラクティブに追加可能
- 関連度スコア付きの検索結果を表示
- すべての操作をコンソールパネルにログ出力
