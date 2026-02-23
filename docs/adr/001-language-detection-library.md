# ADR-001: 言語判定ライブラリの選定

- Status: Accepted
- Date: 2026-02-23

## Context

`task lang` コマンドで設定された言語と、タスクの title/description の言語が一致するかを検証する機能を追加するにあたり、Rust の言語判定ライブラリを選定する必要がある。

要件:

- **軽量**: CLI ツールのバイナリサイズ・コンパイル時間を大きく増やさない
- **信頼度スコア**: 短文での誤判定を避けるため、しきい値ベースのフィルタリングが必要
- **日本語・英語**: 最低限この2言語を高精度で判定できること

## 候補

### 1. whatlang v0.16

- GitHub: https://github.com/greyblake/whatlang-rs
- crates.io: https://crates.io/crates/whatlang
- ライセンス: MIT
- 対応言語数: 70
- アルゴリズム: trigram (n=3) ベース（Cavnar & Trenkle, 1994）
- 信頼度スコア: あり（0.0〜1.0 の `confidence()` + `is_reliable()` メソッド）
- crate サイズ: ~81 KB
- ランタイム依存: `hashbrown` のみ
- スループット: 短文 1.66 MiB/s、長文 9.42 MiB/s

### 2. lingua-rs v1.7

- GitHub: https://github.com/pemistahl/lingua-rs
- crates.io: https://crates.io/crates/lingua
- ライセンス: Apache-2.0
- 対応言語数: 75
- アルゴリズム: n-gram (n=1〜5) + ルールベース。言語ごとに個別のモデル crate を持つ
- 信頼度スコア: あり（各言語の確率分布を返す `compute_language_confidence_values()`）
- crate サイズ: ~3.3 MB（メイン crate のみ）
- ランタイム依存: ~15 crate（ahash, brotli, dashmap, rayon 等）
- バイナリへの影響: 全言語モデル込みで約 110 MB
- 短文精度: 候補中最高（単語レベルで 73.9%、文レベルで 99.7% — README のドイツ語例）

### 3. whichlang v0.1

- GitHub: https://github.com/quickwit-oss/whichlang
- crates.io: https://crates.io/crates/whichlang
- ライセンス: MIT
- 対応言語数: 16
- アルゴリズム: multiclass logistic regression + character n-gram (n=2,3,4)、特徴量ハッシュ（4,096 次元）
- 信頼度スコア: **なし**（`detect_language()` は `Lang` を直接返す、`Option` なし）
- crate サイズ: ~279 KB
- ランタイム依存: なし（ゼロ依存）
- スループット: 短文 105.69 MiB/s、長文 112.31 MiB/s

## 比較

以下のベンチマークデータは lingua-rs README（iMac 3.6 GHz 8-Core Intel Core i9、2,000 文 × 16 言語）からの引用。

| | whatlang | lingua-rs (high) | whichlang |
|---|---|---|---|
| 対応言語数 | 70 | 75 | 16 |
| 信頼度スコア | あり | あり | **なし** |
| crate サイズ | 81 KB | 3.3 MB + モデル | 279 KB |
| ランタイム依存数 | 1 | ~15 | 0 |
| バイナリサイズ影響 | 極小 | ~110 MB | 極小 |
| 処理速度 (single-thread) | 47.41 ms | 361.54 ms | 2.68 ms |
| 短文精度 (≤20 chars) | 78.74% | 最高（公称） | 92.10% |
| 平均精度 (16 言語共通) | 91.69% | — | 97.03% |

精度・速度の出典:
- whichlang vs whatlang の精度比較: [whichlang README](https://github.com/quickwit-oss/whichlang)
- 処理速度ベンチマーク: [lingua-rs README](https://github.com/pemistahl/lingua-rs)
- whichlang の設計詳細: [Quickwit Blog "Whichlang: A fast language detection library for Rust"](https://quickwit.io/blog/whichlang-language-detection-library)

## Decision

**whatlang** を採用する。

### 採用理由

1. **信頼度スコアが必須**: タスク title は最大 50 文字と短い。短文では言語判定の精度が落ちるため、信頼度が低い場合にバリデーションをスキップするしきい値制御が不可欠。whichlang は信頼度スコアを返さないため、この要件を満たせない

2. **バイナリサイズの制約**: lingua-rs は全言語モデル込みで約 110 MB。CLI ツールとしては許容できない。whatlang は crate サイズ 81 KB、ランタイム依存 1 個で、バイナリサイズへの影響が最小

3. **十分な精度**: whichlang（97.03%）より平均精度は劣る（91.69%）が、信頼度しきい値と最小文字数の組み合わせで誤判定を制御できる。日本語・英語のように文字体系が異なる言語ペアでは判定精度は十分に高い

### 不採用理由

- **lingua-rs**: 精度は最高だが、バイナリサイズ ~110 MB は CLI ツールとして過剰。依存も ~15 crate と重い
- **whichlang**: 速度・精度ともに優秀だが、信頼度スコアがないため短文での誤判定を制御できない。16 言語のみという制約もある

## 実装の詳細

- ライブラリ: `whatlang = "0.16"`
- 信頼度しきい値: `0.5`（これ未満はバリデーションをスキップ）
- 最小文字数: `8`（これ未満はバリデーションをスキップ）
- 言語コード: ISO 639-1（ja, en 等）と ISO 639-3（jpn, eng 等）の両方を受け付ける

## References

- whatlang: https://github.com/greyblake/whatlang-rs
- lingua-rs: https://github.com/pemistahl/lingua-rs
- whichlang: https://github.com/quickwit-oss/whichlang
- Quickwit Blog - Whichlang: https://quickwit.io/blog/whichlang-language-detection-library
- lingua-rs ベンチマーク: https://github.com/pemistahl/lingua-rs#library-comparison
- Cavnar, W. B., & Trenkle, J. M. (1994). "N-Gram-Based Text Categorization"
