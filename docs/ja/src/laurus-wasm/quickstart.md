# クイックスタート

## 基本的な使い方（インメモリ）

```javascript
import init, { Index, Schema } from 'laurus-wasm';

// WASM モジュールを初期化
await init();

// スキーマを定義
const schema = new Schema();
schema.addTextField("title");
schema.addTextField("body");
schema.setDefaultFields(["title", "body"]);

// インメモリインデックスを作成
const index = await Index.create(schema);

// ドキュメントを追加
await index.putDocument("doc1", {
  title: "Rust 入門",
  body: "Rust はシステムプログラミング言語です"
});
await index.putDocument("doc2", {
  title: "WebAssembly ガイド",
  body: "WASM はブラウザでネイティブに近いパフォーマンスを実現します"
});
await index.commit();

// 検索
const results = await index.search("rust");
for (const result of results) {
  console.log(`${result.id}: ${result.score}`);
  console.log(result.document);
}
```

## 永続化ストレージ（OPFS）

```javascript
import init, { Index, Schema } from 'laurus-wasm';

await init();

const schema = new Schema();
schema.addTextField("title");
schema.addTextField("body");

// 永続化インデックスを開く（ページリロード後もデータが保持される）
const index = await Index.open("my-index", schema);

// ドキュメントを追加
await index.putDocument("doc1", {
  title: "Hello",
  body: "World"
});

// commit() で自動的に OPFS に永続化される
await index.commit();

// 次のページロード時、Index.open("my-index") でデータが復元される
```

## ベクトル検索

```javascript
import init, { Index, Schema } from 'laurus-wasm';

await init();

const schema = new Schema();
schema.addTextField("title");
schema.addHnswField("embedding", 3); // 3次元ベクトル

const index = await Index.create(schema);

await index.putDocument("doc1", {
  title: "Rust",
  embedding: [1.0, 0.0, 0.0]
});
await index.putDocument("doc2", {
  title: "Python",
  embedding: [0.0, 1.0, 0.0]
});
await index.commit();

// ベクトル類似度で検索
const results = await index.searchVector("embedding", [0.9, 0.1, 0.0]);
console.log(results[0].document.title); // "Rust"
```

## バンドラーでの利用

### Vite

```javascript
// vite.config.js
import wasm from 'vite-plugin-wasm';

export default {
  plugins: [wasm()]
};
```

### Webpack 5

Webpack 5 は `asyncWebAssembly` で WASM をネイティブサポートしています:

```javascript
// webpack.config.js
module.exports = {
  experiments: {
    asyncWebAssembly: true
  }
};
```
