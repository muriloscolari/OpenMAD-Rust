//! Localization helpers (Português BR & English).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Pt,
    En,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        if code.trim().eq_ignore_ascii_case("en") {
            Self::En
        } else {
            Self::Pt
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Pt => "pt",
            Self::En => "en",
        }
    }

    pub fn is_pt(self) -> bool {
        matches!(self, Self::Pt)
    }

    pub fn t(self, pt: &'static str, en: &'static str) -> &'static str {
        match self {
            Self::Pt => pt,
            Self::En => en,
        }
    }
}
