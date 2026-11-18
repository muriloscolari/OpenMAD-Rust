//! Lucide icon font helpers.

use crate::theme::FONT_LUCIDE;
use iced::widget::text;
use iced::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Keyboard,
    Lightbulb,
    Zap,
    Layers,
    CircleDot,
    AppWindow,
    HardDrive,
    Settings,
    Save,
    Upload,
    Download,
    Plus,
    Minus,
    Trash,
    Copy,
    Pencil,
    Search,
    ChevronDown,
    ChevronUp,
    MoreHorizontal,
    Minimize,
    Refresh,
    Check,
    X,
    AlertTriangle,
    ExternalLink,
    Palette,
    Flame,
    Waves,
    Wind,
    Rainbow,
    RotateCcw,
    Power,
    Usb,
    Sparkles,
    Sun,
}

impl Icon {
    pub const fn codepoint(self) -> char {
        match self {
            Self::Keyboard => '\u{e284}',
            Self::Lightbulb => '\u{e1c2}',
            Self::Zap => '\u{e1b4}',
            Self::Layers => '\u{e529}',
            Self::CircleDot => '\u{e345}',
            Self::AppWindow => '\u{e426}',
            Self::HardDrive => '\u{e0ed}',
            Self::Settings => '\u{e154}',
            Self::Save => '\u{e14d}',
            Self::Upload => '\u{e19e}',
            Self::Download => '\u{e0b2}',
            Self::Plus => '\u{e13d}',
            Self::Minus => '\u{e11c}',
            Self::Trash => '\u{e18e}',
            Self::Copy => '\u{e09e}',
            Self::Pencil => '\u{e1f9}',
            Self::Search => '\u{e151}',
            Self::ChevronDown => '\u{e06d}',
            Self::ChevronUp => '\u{e070}',
            Self::MoreHorizontal => '\u{e0b6}',
            Self::Minimize => '\u{e11b}',
            Self::Refresh => '\u{e145}',
            Self::Check => '\u{e06c}',
            Self::X => '\u{e1b2}',
            Self::AlertTriangle => '\u{e193}',
            Self::ExternalLink => '\u{e0b9}',
            Self::Palette => '\u{e1dd}',
            Self::Flame => '\u{e0d2}',
            Self::Waves => '\u{e283}',
            Self::Wind => '\u{e1b0}',
            Self::Rainbow => '\u{e4c2}',
            Self::RotateCcw => '\u{e148}',
            Self::Power => '\u{e140}',
            Self::Usb => '\u{e356}',
            Self::Sparkles => '\u{e412}',
            Self::Sun => '\u{e178}',
        }
    }
}

pub fn icon<'a>(ic: Icon, size: u16, color: Color) -> iced::widget::Text<'a> {
    let mut buf = [0u8; 4];
    let s = ic.codepoint().encode_utf8(&mut buf).to_string();
    text(s).font(FONT_LUCIDE).size(size).color(color)
}
