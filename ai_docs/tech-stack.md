# 技術スタック

## 言語・エディション

| 項目 | 内容 |
|------|------|
| 言語 | Rust 2024 edition |
| ターゲット | x86_64-apple-darwin, x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu |

---

## 主要クレート

### GUI フレームワーク

| クレート | バージョン | 用途 |
|---------|-----------|------|
| `eframe` | 0.31 | ネイティブウィンドウ・イベントループ（egui のホスト） |
| `egui` | 0.31 | 即時モード GUI ウィジェット・レンダリング |

### ファイル操作

| クレート | バージョン | 用途 |
|---------|-----------|------|
| `rfd` | 0.15 | ネイティブファイルダイアログ（Open / Save） |
| `image` | 0.25 | PNG → RGBA 変換（アイコン読み込み） |
| `png` | 0.17 | PNG デコード（CHR インポート用低レベルアクセス） |

### macOS 専用（`[target.'cfg(target_os = "macos")'.dependencies]`）

| クレート | バージョン | 用途 |
|---------|-----------|------|
| `muda` | 0.15 | ネイティブ NSMenu 構築・イベント受信 |
| `objc2-app-kit` | 0.2 | `NSAppearance` / `NSApplication` Binding |
| `objc2-foundation` | 0.2 | `MainThreadMarker` / Objective-C 基盤型 |

---

## NES ドメイン知識

### CHR 2BPP 形式

- 1 タイル = 16 バイト（8 行 × 2 プレーン）
- プレーン 0（バイト 0〜7）: 各ピクセルの LSB
- プレーン 1（バイト 8〜15）: 各ピクセルの MSB
- 各行の左端ピクセルがビット 7、右端がビット 0
- カラーインデックス = (plane1_bit << 1) | plane0_bit（0〜3）

### iNES ファイル形式

```
[0x00-0x0F] ヘッダ 16 バイト
  [0x00-0x03] マジック "NES\x1A"
  [0x04]      PRG-ROM バンク数（16KB 単位）
  [0x05]      CHR-ROM バンク数（8KB 単位）、0 = CHR-RAM
  [0x06]      フラグ6（マッパー下位 / ミラーリング / バッテリー / トレーナー）
  [0x07]      フラグ7（マッパー上位）
[トレーナー]  512 バイト（フラグ6 bit2 が立つ場合のみ）
[PRG-ROM]   prg_banks × 16KB
[CHR-ROM]   chr_banks × 8KB
```

### パレット

- NES マスターパレット: 64 色（各 RGB 3 バイト = 192 バイト、`.pal` 形式）
- DAT パレット: 4 セット × 4 色（各 1 バイト = マスターパレットインデックス、`.dat` 形式）

---

## ビルド・実行

```sh
cargo run                   # 開発実行
cargo build --release       # リリースビルド
```

### 埋め込みアセット（`include_bytes!`）

| ファイル | 用途 |
|---------|------|
| `assets/icon.png` | ウィンドウアイコン |
| `assets/rchr.bin` | 起動時デフォルト CHR データ（R-CHR ロゴ） |
| `assets/rchr.pal` | デフォルトマスターパレット |
| `assets/rchr.dat` | デフォルト DAT パレット |
| `assets/nes.pal` | NES 標準 64 色パレット（リセット用） |