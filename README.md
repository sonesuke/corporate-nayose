# corporate-nayose

日本の法人データの名寄せ(照合・統合)を行う Rust パイプライン。

## 使い方

```sh
mise run build    # Nix で dev container イメージをビルド
mise run up       # dev container 起動
mise run setup    # コンテナ内環境のセットアップ
```

コンテナ内での主なタスク:

| タスク | 内容 |
|---|---|
| `mise tables` | `data/` の原始 CSV から parquet テーブルを生成(release モード) |
| `mise verify-tables` | 生成済みテーブルを実データの期待値と照合 |
| `mise test` / `mise test-e2e` | 単体テスト / 結合テスト |
| `mise fmt` / `mise clippy` | フォーマット / リント |
| `mise pre-commit` | fmt + clippy + test |

## テーブル

`mise tables` が `data/parquet/` に生成する:

- **`companies.parquet`(企業)** — `Kihonjoho_UTF-8.csv`(96列)から名寄せに必要な 26 列を抜粋。法人番号で重複排除(keep-first)。`status` は現存 = null / 閉鎖 = `閉鎖`。閉鎖法人も保持する
- **`establishments.parquet`(事業所)** — `Jigyousyojoho_UTF-8.csv`(15列)から 9 列を抜粋。1 法人に複数事業所があるため重複排除しない

空欄の数値・日付は null に正規化される。郵便番号・市区町村コード等の先頭ゼロは String として保持。

CI は実データなしで全テストを実行する(フィクスチャベース)。実データ検証は `mise verify-tables` で手動実行する。
