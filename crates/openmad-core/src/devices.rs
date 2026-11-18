//! Registry of keyboards on the DuckBread controller.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub const VENDOR_ID: u16 = 0x373B;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFactor {
    Macropad,
    Layout60,
    Layout65,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardSpec {
    pub product_id: u16,
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub layers: usize,
    pub bootloader_id: u16,
    pub verified: bool,
}

impl BoardSpec {
    pub const fn new_static(
        product_id: u16,
        name: &'static str,
        rows: usize,
        cols: usize,
        layers: usize,
        bootloader_id: u16,
        verified: bool,
    ) -> Self {
        let _ = (product_id, name, rows, cols, layers, bootloader_id, verified);
        unreachable!()
    }

    pub fn total_keys(&self) -> usize {
        self.rows * self.cols
    }

    pub fn form_factor(&self) -> FormFactor {
        if self.cols <= 8 {
            FormFactor::Macropad
        } else if self.cols <= 14 {
            FormFactor::Layout60
        } else {
            FormFactor::Layout65
        }
    }
}

pub struct StaticBoard {
    pub product_id: u16,
    pub name: &'static str,
    pub rows: usize,
    pub cols: usize,
    pub layers: usize,
    pub bootloader_id: u16,
    pub verified: bool,
}

impl StaticBoard {
    pub fn to_spec(&self) -> BoardSpec {
        BoardSpec {
            product_id: self.product_id,
            name: self.name.to_string(),
            rows: self.rows,
            cols: self.cols,
            layers: self.layers,
            bootloader_id: self.bootloader_id,
            verified: self.verified,
        }
    }
}

pub const BOARDS: &[StaticBoard] = &[
    // -- 60% family, 5x14 --
    StaticBoard { product_id: 0x1053, name: "MAD60 HE", rows: 5, cols: 14, layers: 4, bootloader_id: 0x201F, verified: true },
    StaticBoard { product_id: 0x1054, name: "MAD60 HE RGB", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2020, verified: true },
    StaticBoard { product_id: 0x1055, name: "MAD60 HE Ultra", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2021, verified: false },
    StaticBoard { product_id: 0x1056, name: "MAD60 HE Limited", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2022, verified: false },
    StaticBoard { product_id: 0x10AD, name: "MAD60 Pro", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2053, verified: false },
    StaticBoard { product_id: 0x1120, name: "MAD60 HE V2", rows: 5, cols: 14, layers: 4, bootloader_id: 0x210B, verified: false },
    StaticBoard { product_id: 0x10F6, name: "MAD Light 60 HE", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2085, verified: false },
    StaticBoard { product_id: 0x110B, name: "MAD63 HE", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2102, verified: false },
    StaticBoard { product_id: 0x112A, name: "Z60", rows: 5, cols: 14, layers: 4, bootloader_id: 0x2115, verified: false },
    // -- 65% / 68 family, 5x15 --
    StaticBoard { product_id: 0x1057, name: "MAD68 HE", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2023, verified: false },
    StaticBoard { product_id: 0x1058, name: "MAD68 HE RGB", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2024, verified: true },
    StaticBoard { product_id: 0x1059, name: "MAD68 HE Ultra", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2025, verified: false },
    StaticBoard { product_id: 0x105A, name: "MAD68 HE Limited", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2026, verified: false },
    StaticBoard { product_id: 0x10A7, name: "MAD68 R", rows: 5, cols: 15, layers: 4, bootloader_id: 0x204E, verified: false },
    StaticBoard { product_id: 0x10D6, name: "MAD68 Pro", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2075, verified: false },
    StaticBoard { product_id: 0x1123, name: "MAD68 HE V2", rows: 5, cols: 15, layers: 4, bootloader_id: 0x210E, verified: false },
    StaticBoard { product_id: 0x1144, name: "MAD Light 68 HE", rows: 5, cols: 15, layers: 4, bootloader_id: 0x212D, verified: false },
    StaticBoard { product_id: 0x1136, name: "Z68", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2121, verified: false },
    StaticBoard { product_id: 0x1139, name: "MAG68", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2124, verified: false },
    StaticBoard { product_id: 0x10A3, name: "FIRE68", rows: 5, cols: 15, layers: 4, bootloader_id: 0x204A, verified: false },
    StaticBoard { product_id: 0x10A9, name: "FIRE68", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2050, verified: false },
    StaticBoard { product_id: 0x10AA, name: "FIRE68 Pro", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2051, verified: false },
    StaticBoard { product_id: 0x10A4, name: "FIRE68 Ultra", rows: 5, cols: 15, layers: 4, bootloader_id: 0x204B, verified: false },
    StaticBoard { product_id: 0x10AB, name: "FIRE68 Ultra", rows: 5, cols: 15, layers: 4, bootloader_id: 0x2052, verified: false },
    // -- macropad, 5x6 --
    StaticBoard { product_id: 0x107D, name: "MAD Smart", rows: 5, cols: 6, layers: 4, bootloader_id: 0x2035, verified: false },
];

pub fn default_board() -> BoardSpec {
    lookup(0x1058).expect("MAD68 HE RGB is always in BOARDS")
}

pub fn lookup(product_id: u16) -> Option<BoardSpec> {
    BOARDS
        .iter()
        .find(|b| b.product_id == product_id)
        .map(StaticBoard::to_spec)
}

pub fn describe(product_id: u16) -> BoardSpec {
    lookup(product_id).unwrap_or_else(|| {
        let def = default_board();
        BoardSpec {
            product_id,
            name: format!("Unrecognised board ({product_id:#06x})"),
            rows: def.rows,
            cols: def.cols,
            layers: def.layers,
            bootloader_id: 0,
            verified: false,
        }
    })
}

pub fn is_bootloader_pid(product_id: u16) -> bool {
    BOARDS.iter().any(|b| b.bootloader_id == product_id)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct UnlockedFile {
    #[serde(default)]
    unlocked: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UnlockedBoards {
    pids: BTreeSet<u16>,
}

impl UnlockedBoards {
    pub fn load(path: &Path) -> Self {
        let mut set = BTreeSet::new();
        if let Ok(text) = std::fs::read_to_string(path) {
            let trimmed = text.trim_start_matches('\u{feff}');
            if let Ok(raw) = serde_json::from_str::<UnlockedFile>(trimmed) {
                for s in raw.unlocked {
                    if let Some(pid) = parse_pid(&s) {
                        set.insert(pid);
                    }
                }
            }
        }
        Self { pids: set }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = UnlockedFile {
            unlocked: self.pids.iter().map(|p| format!("{p:#06x}")).collect(),
        };
        let json = serde_json::to_string_pretty(&file).unwrap_or_default();
        std::fs::write(path, json)
    }

    pub fn is_unlocked(&self, pid: u16) -> bool {
        self.pids.contains(&pid)
    }

    pub fn set_unlocked(&mut self, pid: u16, on: bool) {
        if on {
            self.pids.insert(pid);
        } else {
            self.pids.remove(&pid);
        }
    }

    pub fn list(&self) -> Vec<u16> {
        self.pids.iter().copied().collect()
    }
}

fn parse_pid(s: &str) -> Option<u16> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u16::from_str_radix(hex, 16).ok()
    } else {
        t.parse::<u16>().ok()
    }
}
