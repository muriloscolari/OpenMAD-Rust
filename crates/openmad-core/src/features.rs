//! Structured types for the keyboard's Hall-effect, lighting, and macro features.

use serde::{Deserialize, Serialize};

use crate::protocol::{AdvancedKeyMode, MacroAction};

pub const DKS_ACTION_COUNT: usize = 4;
pub const DKS_ACTION_SIZE: usize = 4;
pub const DKS_STATUS_BITS: usize = 10;

pub const MACRO_ESCAPE: u8 = 0x01;
pub const MACRO_TERMINATOR: u8 = 0x00;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DksAction {
    pub status: u16,
    pub key_class: u8,
    pub key_id: u8,
}

impl DksAction {
    pub fn keycode(self) -> u16 {
        ((self.key_class as u16) << 8) | (self.key_id as u16)
    }

    pub fn from_keycode(keycode: u16, status: u16) -> Self {
        Self {
            status,
            key_class: ((keycode >> 8) & 0xFF) as u8,
            key_id: (keycode & 0xFF) as u8,
        }
    }

    pub fn bits(self) -> [bool; DKS_STATUS_BITS] {
        let mut out = [false; DKS_STATUS_BITS];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = ((self.status >> i) & 1) != 0;
        }
        out
    }

    pub fn is_empty(self) -> bool {
        self.status == 0 && self.keycode() == 0
    }

    pub fn to_bytes(self) -> [u8; 4] {
        let s = self.status.to_be_bytes();
        [s[0], s[1], self.key_class, self.key_id]
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            status: u16::from_be_bytes([data[0], data[1]]),
            key_class: data[2],
            key_id: data[3],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dks {
    pub index: usize,
    pub actions: [DksAction; DKS_ACTION_COUNT],
}

impl Dks {
    pub fn empty(index: usize) -> Self {
        Self {
            index,
            actions: [DksAction::default(); DKS_ACTION_COUNT],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.actions.iter().all(|a| a.is_empty())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(DKS_ACTION_COUNT * DKS_ACTION_SIZE);
        for a in &self.actions {
            out.extend_from_slice(&a.to_bytes());
        }
        out
    }

    pub fn from_bytes(index: usize, data: &[u8]) -> Self {
        let mut actions = [DksAction::default(); DKS_ACTION_COUNT];
        for (i, slot) in actions.iter_mut().enumerate() {
            let off = i * DKS_ACTION_SIZE;
            if off + DKS_ACTION_SIZE <= data.len() {
                *slot = DksAction::from_bytes(&data[off..off + DKS_ACTION_SIZE]);
            }
        }
        Self { index, actions }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancedKey {
    pub index: usize,
    pub mode: AdvancedKeyMode,
    pub id: u8,
    pub rs_apc_lv: u16,
    pub gapc_sw: u8,
    pub rt_sw: u8,
    pub key1_row: u8,
    pub key1_col: u8,
    pub key2_row: u8,
    pub key2_col: u8,
    pub layer: u8,
}

impl AdvancedKey {
    pub fn empty(index: usize) -> Self {
        Self {
            index,
            mode: AdvancedKeyMode::None,
            id: 0,
            rs_apc_lv: 0,
            gapc_sw: 0,
            rt_sw: 0,
            key1_row: 0,
            key1_col: 0,
            key2_row: 0,
            key2_col: 0,
            layer: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.mode != AdvancedKeyMode::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LightInfo {
    pub effect: u8,
    pub speed: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub brightness: u8,
}

impl LightInfo {
    pub fn color_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BoxLight {
    pub mode: u8,
    pub colorful: u8,
    pub brightness: u8,
    pub speed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TapDance {
    pub index: usize,
    pub tap: u16,
    pub hold: u16,
    pub timer_ms: u16,
}

impl TapDance {
    pub fn empty(index: usize) -> Self {
        Self {
            index,
            tap: 0,
            hold: 0,
            timer_ms: 200,
        }
    }

    pub fn is_active(self) -> bool {
        self.tap != 0 || self.hold != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct DeadBand {
    pub top_mm: f32,
    pub bottom_mm: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeyboardFeature {
    pub rgb_area: u8,
    pub wasd_switch: u8,
    pub mac_switch: u8,
    pub win_lock: u8,
    pub nkro_switch: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RapidTrigger {
    pub row: u8,
    pub col: u8,
    pub enabled: bool,
    pub press_mm: f32,
    pub release_mm: f32,
}

/// All 21 firmware lighting effects supported by DuckBread (exact vendor table).
#[derive(Debug, Clone, Copy)]
pub struct LightEffectMeta {
    pub id: u8,
    pub name_en: &'static str,
    pub name_pt: &'static str,
    pub has_speed: bool,
    pub has_color: bool,
}

impl LightEffectMeta {
    pub fn label(self, pt: bool) -> &'static str {
        if pt { self.name_pt } else { self.name_en }
    }
}

pub const LIGHT_EFFECTS: &[LightEffectMeta] = &[
    LightEffectMeta { id: 0,  name_en: "Off",                             name_pt: "Desligado",                      has_speed: false, has_color: false },
    LightEffectMeta { id: 1,  name_en: "Customization (Per-Key)",         name_pt: "Personalizado (Por Tecla)",      has_speed: false, has_color: true  },
    LightEffectMeta { id: 2,  name_en: "Monochromatic Constant",          name_pt: "Cor Fixa",                       has_speed: false, has_color: true  },
    LightEffectMeta { id: 3,  name_en: "Focus",                           name_pt: "Foco",                           has_speed: false, has_color: true  },
    LightEffectMeta { id: 4,  name_en: "Rainbow Static (Vertical)",       name_pt: "Arco-Íris Estático (Vertical)",  has_speed: false, has_color: true  },
    LightEffectMeta { id: 5,  name_en: "Rainbow Static (Horizontal)",     name_pt: "Arco-Íris Estático (Horizontal)",has_speed: false, has_color: true  },
    LightEffectMeta { id: 6,  name_en: "Monochromatic Breathing",         name_pt: "Respiração Monocromática",       has_speed: true,  has_color: true  },
    LightEffectMeta { id: 7,  name_en: "Ribbon Fluttering 1",             name_pt: "Fita Ondulante 1",               has_speed: true,  has_color: true  },
    LightEffectMeta { id: 8,  name_en: "Ribbon Fluttering 2",             name_pt: "Fita Ondulante 2",               has_speed: true,  has_color: true  },
    LightEffectMeta { id: 9,  name_en: "Rotating Belt 1",                 name_pt: "Faixa Rotativa 1",               has_speed: true,  has_color: true  },
    LightEffectMeta { id: 11, name_en: "Rotating Belt 2",                 name_pt: "Faixa Rotativa 2",               has_speed: true,  has_color: true  },
    LightEffectMeta { id: 13, name_en: "Spectrum Cycle (RGB)",            name_pt: "Ciclo de Espectro (RGB)",        has_speed: true,  has_color: false },
    LightEffectMeta { id: 19, name_en: "Colorful Swirls 1",               name_pt: "Espiral Colorida 1",             has_speed: true,  has_color: false },
    LightEffectMeta { id: 23, name_en: "Colorful Swirls 2",               name_pt: "Espiral Colorida 2",             has_speed: true,  has_color: false },
    LightEffectMeta { id: 25, name_en: "Christmas",                       name_pt: "Natal",                          has_speed: false, has_color: false },
    LightEffectMeta { id: 26, name_en: "Spectrum Breathing",              name_pt: "Respiração Espectral",           has_speed: true,  has_color: true  },
    LightEffectMeta { id: 28, name_en: "Spectrum Floating",               name_pt: "Espectro Flutuante",             has_speed: true,  has_color: true  },
    LightEffectMeta { id: 30, name_en: "Digital Rain",                    name_pt: "Chuva Digital",                  has_speed: false, has_color: false },
    LightEffectMeta { id: 31, name_en: "Monochromatic Lighting Up",       name_pt: "Acendimento Monocromático",      has_speed: true,  has_color: true  },
    LightEffectMeta { id: 36, name_en: "Crucifix",                        name_pt: "Cruzamento",                     has_speed: true,  has_color: true  },
    LightEffectMeta { id: 39, name_en: "Fingertip Rainbow",               name_pt: "Arco-Íris ao Digitar",           has_speed: true,  has_color: false },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MacroStep {
    Text { text: String },
    Tap { keycode: u16 },
    Down { keycode: u16 },
    Up { keycode: u16 },
    Delay { delay_ms: u32 },
}

fn decode_delay(b1: u8, b2: u8) -> u32 {
    (b1.saturating_sub(1) as u32) + (b2.saturating_sub(1) as u32) * 255
}

fn encode_delay(ms: u32) -> [u8; 2] {
    let clamped = ms.min(255 * 255 + 254);
    [((clamped % 255) + 1) as u8, ((clamped / 255) + 1) as u8]
}

pub fn decode_macro(data: &[u8]) -> Vec<MacroStep> {
    let mut steps = Vec::new();
    let mut text = String::new();
    let mut i = 0;

    while i < data.len() {
        let b = data[i];
        if b != MACRO_ESCAPE {
            text.push(b as char);
            i += 1;
            continue;
        }
        if !text.is_empty() {
            steps.push(MacroStep::Text {
                text: std::mem::take(&mut text),
            });
        }
        if i + 1 >= data.len() {
            break;
        }
        let op = data[i + 1];
        let Some(action) = MacroAction::from_u8(op) else {
            i += 2;
            continue;
        };
        match action {
            MacroAction::SsTapCode if i + 2 < data.len() => {
                steps.push(MacroStep::Tap { keycode: data[i + 2] as u16 });
                i += 3;
            }
            MacroAction::SsDownCode if i + 2 < data.len() => {
                steps.push(MacroStep::Down { keycode: data[i + 2] as u16 });
                i += 3;
            }
            MacroAction::SsUpCode if i + 2 < data.len() => {
                steps.push(MacroStep::Up { keycode: data[i + 2] as u16 });
                i += 3;
            }
            MacroAction::VialMacroExtTap if i + 3 < data.len() => {
                let kc = (data[i + 2] as u16) | ((data[i + 3] as u16) << 8);
                steps.push(MacroStep::Tap { keycode: kc });
                i += 4;
            }
            MacroAction::VialMacroExtDown if i + 3 < data.len() => {
                let kc = (data[i + 2] as u16) | ((data[i + 3] as u16) << 8);
                steps.push(MacroStep::Down { keycode: kc });
                i += 4;
            }
            MacroAction::VialMacroExtUp if i + 3 < data.len() => {
                let kc = (data[i + 2] as u16) | ((data[i + 3] as u16) << 8);
                steps.push(MacroStep::Up { keycode: kc });
                i += 4;
            }
            MacroAction::SsDelayCode if i + 3 < data.len() => {
                steps.push(MacroStep::Delay {
                    delay_ms: decode_delay(data[i + 2], data[i + 3]),
                });
                i += 4;
            }
            _ => i += 2,
        }
    }
    if !text.is_empty() {
        steps.push(MacroStep::Text { text });
    }
    steps
}

pub fn encode_macro(steps: &[MacroStep]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for s in steps {
        match s {
            MacroStep::Text { text } => {
                for ch in text.chars() {
                    let code = ch as u32;
                    if code == MACRO_ESCAPE as u32 || code == MACRO_TERMINATOR as u32 || code > 0x7F {
                        return Err(format!(
                            "macro text cannot contain byte {code:#04x}; use a tap step"
                        ));
                    }
                    out.push(code as u8);
                }
            }
            MacroStep::Delay { delay_ms } => {
                out.push(MACRO_ESCAPE);
                out.push(MacroAction::SsDelayCode as u8);
                out.extend_from_slice(&encode_delay(*delay_ms));
            }
            MacroStep::Tap { keycode } => push_key_step(&mut out, *keycode, MacroAction::SsTapCode, MacroAction::VialMacroExtTap),
            MacroStep::Down { keycode } => push_key_step(&mut out, *keycode, MacroAction::SsDownCode, MacroAction::VialMacroExtDown),
            MacroStep::Up { keycode } => push_key_step(&mut out, *keycode, MacroAction::SsUpCode, MacroAction::VialMacroExtUp),
        }
    }
    Ok(out)
}

fn push_key_step(out: &mut Vec<u8>, kc: u16, one: MacroAction, two: MacroAction) {
    out.push(MACRO_ESCAPE);
    if kc > 0xFF {
        out.push(two as u8);
        out.push((kc & 0xFF) as u8);
        out.push(((kc >> 8) & 0xFF) as u8);
    } else {
        out.push(one as u8);
        out.push((kc & 0xFF) as u8);
    }
}

pub fn split_macro_buffer(buffer: &[u8], count: usize) -> Vec<Vec<u8>> {
    let mut macros = Vec::with_capacity(count);
    let mut start = 0;
    for _ in 0..count {
        if start >= buffer.len() {
            macros.push(Vec::new());
            continue;
        }
        if let Some(rel_end) = buffer[start..].iter().position(|&b| b == MACRO_TERMINATOR) {
            macros.push(buffer[start..start + rel_end].to_vec());
            start += rel_end + 1;
        } else {
            macros.push(buffer[start..].to_vec());
            start = buffer.len();
        }
    }
    macros
}

pub fn join_macro_buffer(macros: &[Vec<u8>], size: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for m in macros {
        out.extend_from_slice(m);
        out.push(MACRO_TERMINATOR);
    }
    if out.len() > size {
        return Err(format!("macros need {} bytes, buffer is {}", out.len(), size));
    }
    out.resize(size, 0);
    Ok(out)
}
