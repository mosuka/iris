# インストール

## PyPI からインストール

```bash
pip install laurus
```

## ソースからビルド

ソースからビルドするには Rust ツールチェーン（1.75 以降）と [Maturin](https://github.com/PyO3/maturin) が必要です。

```bash
# Maturin をインストール
pip install maturin

# リポジトリをクローン
git clone https://github.com/mosuka/laurus.git
cd laurus/laurus-python

# 開発モードでビルドとインストール
maturin develop

# またはリリースホイールをビルド
maturin build --release
pip install target/wheels/laurus-*.whl
```

## 動作確認

```python
import laurus
index = laurus.Index()
print(index)  # Index()
```

## 動作要件

- Python 3.8 以降
- コンパイル済みネイティブ拡張以外のランタイム依存関係なし
