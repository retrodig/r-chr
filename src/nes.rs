/// iNES ヘッダ（16バイト）のパース結果
#[derive(Debug, Clone)]
pub struct NesHeader {
    /// PRG-ROM のサイズ（16KB 単位）
    pub prg_rom_banks: u8,
    /// CHR-ROM のサイズ（8KB 単位）。0 の場合は CHR-RAM
    pub chr_rom_banks: u8,
    /// マッパー番号
    pub mapper: u8,
    /// 縦スクロール(true) / 横スクロール(false)
    pub vertical_mirroring: bool,
    /// バッテリーバックアップ SRAM あり
    pub has_battery: bool,
}

impl NesHeader {
    pub fn prg_rom_size(&self) -> usize {
        self.prg_rom_banks as usize * 16 * 1024
    }

    pub fn chr_rom_size(&self) -> usize {
        self.chr_rom_banks as usize * 8 * 1024
    }
}

/// NES ROM ファイルのパース結果
#[derive(Debug, Clone)]
pub struct NesRom {
    pub header: NesHeader,
    /// PRG-ROM バイト列
    pub prg_rom: Vec<u8>,
    /// CHR-ROM バイト列（CHR-RAM の場合は空）
    pub chr_rom: Vec<u8>,
}

#[derive(Debug)]
pub enum ParseError {
    TooShort,
    InvalidMagic,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::TooShort => write!(f, "ファイルが短すぎます"),
            ParseError::InvalidMagic => write!(f, "NES ファイルではありません（マジックナンバー不一致）"),
        }
    }
}

/// バイト列を iNES フォーマットとしてパースする
pub fn parse_nes(data: &[u8]) -> Result<NesRom, ParseError> {
    if data.len() < 16 {
        return Err(ParseError::TooShort);
    }

    // マジックナンバー: "NES" + 0x1A
    if &data[0..4] != b"NES\x1a" {
        return Err(ParseError::InvalidMagic);
    }

    let prg_rom_banks = data[4];
    let chr_rom_banks = data[5];
    let flags6 = data[6];
    let flags7 = data[7];

    let vertical_mirroring = flags6 & 0x01 != 0;
    let has_battery = flags6 & 0x02 != 0;
    let has_trainer = flags6 & 0x04 != 0;
    let mapper_lo = flags6 >> 4;
    let mapper_hi = flags7 & 0xF0;
    let mapper = mapper_hi | mapper_lo;

    let header = NesHeader {
        prg_rom_banks,
        chr_rom_banks,
        mapper,
        vertical_mirroring,
        has_battery,
    };

    // トレーナー（512バイト）がある場合はスキップ
    let mut offset = 16;
    if has_trainer {
        offset += 512;
    }

    let prg_size = header.prg_rom_size();
    let chr_size = header.chr_rom_size();

    if data.len() < offset + prg_size + chr_size {
        return Err(ParseError::TooShort);
    }

    let prg_rom = data[offset..offset + prg_size].to_vec();
    let chr_rom = data[offset + prg_size..offset + prg_size + chr_size].to_vec();

    Ok(NesRom { header, prg_rom, chr_rom })
}