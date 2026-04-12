use eframe::egui;

/// NES ハードウェアの標準 64色マスターパレット（RGB各 8bit）
/// 参考: https://www.nesdev.org/wiki/PPU_palettes
pub const NES_PALETTE: [[u8; 3]; 64] = [
    [84,  84,  84 ], // 0x00
    [0,   30,  116], // 0x01
    [8,   16,  144], // 0x02
    [48,  0,   136], // 0x03
    [68,  0,   100], // 0x04
    [92,  0,   48 ], // 0x05
    [84,  4,   0  ], // 0x06
    [60,  24,  0  ], // 0x07
    [32,  42,  0  ], // 0x08
    [8,   58,  0  ], // 0x09
    [0,   64,  0  ], // 0x0A
    [0,   60,  0  ], // 0x0B
    [0,   50,  60 ], // 0x0C
    [0,   0,   0  ], // 0x0D
    [0,   0,   0  ], // 0x0E
    [0,   0,   0  ], // 0x0F
    [152, 150, 152], // 0x10
    [8,   76,  196], // 0x11
    [48,  50,  236], // 0x12
    [92,  30,  228], // 0x13
    [136, 20,  176], // 0x14
    [160, 20,  100], // 0x15
    [152, 34,  32 ], // 0x16
    [120, 60,  0  ], // 0x17
    [84,  90,  0  ], // 0x18
    [40,  114, 0  ], // 0x19
    [8,   124, 0  ], // 0x1A
    [0,   118, 40 ], // 0x1B
    [0,   102, 120], // 0x1C
    [0,   0,   0  ], // 0x1D
    [0,   0,   0  ], // 0x1E
    [0,   0,   0  ], // 0x1F
    [236, 238, 236], // 0x20
    [76,  154, 236], // 0x21
    [120, 124, 236], // 0x22
    [176, 98,  236], // 0x23
    [228, 84,  236], // 0x24
    [236, 88,  180], // 0x25
    [236, 106, 100], // 0x26
    [212, 136, 32 ], // 0x27
    [160, 170, 0  ], // 0x28
    [116, 196, 0  ], // 0x29
    [76,  208, 32 ], // 0x2A
    [56,  204, 108], // 0x2B
    [56,  180, 204], // 0x2C
    [60,  60,  60 ], // 0x2D
    [0,   0,   0  ], // 0x2E
    [0,   0,   0  ], // 0x2F
    [236, 238, 236], // 0x30
    [168, 204, 236], // 0x31
    [188, 188, 236], // 0x32
    [212, 178, 236], // 0x33
    [236, 174, 236], // 0x34
    [236, 174, 212], // 0x35
    [236, 180, 176], // 0x36
    [228, 196, 144], // 0x37
    [204, 210, 120], // 0x38
    [180, 222, 120], // 0x39
    [168, 226, 144], // 0x3A
    [152, 226, 180], // 0x3B
    [160, 214, 228], // 0x3C
    [160, 162, 160], // 0x3D
    [0,   0,   0  ], // 0x3E
    [0,   0,   0  ], // 0x3F
];

/// NES マスターパレット（64色 RGB）
///
/// デフォルトは NES_PALETTE 定数。
/// PAL ファイルを読み込むことで任意の RGB 値に差し替えられる。
#[derive(Clone)]
pub struct MasterPalette {
    pub colors: [[u8; 3]; 64],
}

impl Default for MasterPalette {
    fn default() -> Self {
        Self { colors: NES_PALETTE }
    }
}

impl MasterPalette {
    /// 192 バイトの .pal データ（64色 × RGB 3バイト）からパース
    pub fn from_pal_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 192 {
            return None;
        }
        let mut colors = [[0u8; 3]; 64];
        for i in 0..64 {
            colors[i] = [data[i * 3], data[i * 3 + 1], data[i * 3 + 2]];
        }
        Some(Self { colors })
    }
}

/// DAT パレット: 4セット × 4色 の NES パレットインデックス
///
/// NES の PPU パレットメモリに相当する。
/// 各値は MasterPalette のインデックス（0x00〜0x3F）。
#[derive(Clone)]
pub struct DatPalette {
    /// sets[set_index][color_index] = NES パレットインデックス
    pub sets: [[u8; 4]; 4],
}

impl Default for DatPalette {
    fn default() -> Self {
        Self {
            sets: [
                [0x0F, 0x00, 0x10, 0x30], // セット 0: 黒〜白（グレースケール系）
                [0x0F, 0x16, 0x26, 0x36], // セット 1: 赤系
                [0x0F, 0x19, 0x29, 0x39], // セット 2: 緑系
                [0x0F, 0x11, 0x21, 0x31], // セット 3: 青系
            ],
        }
    }
}

impl DatPalette {
    /// 指定セット・カラーインデックスの RGB を返す
    pub fn color_rgb(&self, set: usize, color_idx: usize, master: &MasterPalette) -> [u8; 3] {
        let nes_idx = self.sets[set][color_idx] as usize & 0x3F;
        master.colors[nes_idx]
    }

    /// egui の Color32 で返す
    pub fn color32(&self, set: usize, color_idx: usize, master: &MasterPalette) -> egui::Color32 {
        let [r, g, b] = self.color_rgb(set, color_idx, master);
        egui::Color32::from_rgb(r, g, b)
    }

    /// 16バイト以上の .dat データからパース（4セット × 4色）
    ///
    /// YY-CHR 互換フォーマット: 各バイトが NES パレットインデックス（0x00〜0x3F）
    pub fn from_dat_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }
        let mut sets = [[0u8; 4]; 4];
        for s in 0..4 {
            for c in 0..4 {
                sets[s][c] = data[s * 4 + c] & 0x3F;
            }
        }
        Some(Self { sets })
    }

    /// 現在のパレットを 16バイトの .dat 形式で返す
    pub fn to_dat_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        for s in 0..4 {
            for c in 0..4 {
                out[s * 4 + c] = self.sets[s][c];
            }
        }
        out
    }
}
