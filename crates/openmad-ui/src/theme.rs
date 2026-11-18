//! Dark Mode design system, static Inter typography, Lucide icon font, and dynamic keyboard accent.

use iced::font::Weight;
use iced::widget::{button, container, pick_list, slider, text_input, toggler};
use iced::{Background, Border, Color, Font, Shadow, Vector};

pub const INTER_REGULAR_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
pub const INTER_MEDIUM_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-Medium.ttf");
pub const INTER_SEMIBOLD_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
pub const LUCIDE_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/lucide.ttf");

pub const FONT_INTER: Font = Font::with_name("Inter");
pub const FONT_MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..Font::with_name("Inter")
};
pub const FONT_SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..Font::with_name("Inter")
};
pub const FONT_LUCIDE: Font = Font::with_name("lucide");

pub mod type_scale {
    pub const DISPLAY: u16 = 28;
    pub const TITLE: u16 = 18;
    pub const HEADING: u16 = 14;
    pub const BODY: u16 = 13;
    pub const CAPTION: u16 = 11;
}

pub const SP_XS: u16 = 4;
pub const SP_SM: u16 = 8;
pub const SP_MD: u16 = 12;
pub const SP_LG: u16 = 16;
pub const SP_XL: u16 = 24;

pub const R_SM: f32 = 6.0;
pub const R_MD: f32 = 10.0;
pub const R_LG: f32 = 14.0;

pub const BG_BASE: Color = Color::from_rgb(0.055, 0.055, 0.071); // #0E0E12
pub const BG_SIDEBAR: Color = Color::from_rgb(0.075, 0.075, 0.094); // #131318
pub const BG_HEADER: Color = Color::from_rgb(0.067, 0.067, 0.086); // #111116
pub const BG_SURFACE: Color = Color::from_rgb(0.094, 0.094, 0.118); // #18181E
pub const BG_ELEVATED: Color = Color::from_rgb(0.133, 0.133, 0.165); // #22222A
pub const BG_HOVER: Color = Color::from_rgb(0.173, 0.173, 0.212); // #2C2C36
pub const BG_SEGMENT: Color = Color::from_rgb(0.071, 0.071, 0.090); // #121217

pub const BORDER_SUBTLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.07);
pub const BORDER_GLASS_TOP: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.11);
pub const BORDER_STRONG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.17);

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.961, 0.961, 0.969); // #F5F5F7
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.675, 0.675, 0.710); // #ACAEB5
pub const TEXT_MUTED: Color = Color::from_rgb(0.467, 0.467, 0.506); // #777781

pub const ACCENT_BLUE: Color = Color::from_rgb(0.039, 0.518, 1.0); // #0A84FF
pub const GREEN_OK: Color = Color::from_rgb(0.188, 0.820, 0.345); // #30D158
pub const AMBER_WARN: Color = Color::from_rgb(1.0, 0.624, 0.039); // #FF9F0A
pub const RED_DANGER: Color = Color::from_rgb(1.0, 0.271, 0.227); // #FF453A

/// Derives a legible dark-mode UI accent color from the keyboard's active RGB color.
pub fn dynamic_accent(rgb: [u8; 3]) -> Color {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    // Fallback to Apple Blue if keyboard is off/black or pure dark gray
    if max < 0.08 {
        return ACCENT_BLUE;
    }

    // Near-white or very unsaturated: use a crisp cool-white/ice tint so it stays distinct from borders
    if max - min < 0.08 && lum > 0.75 {
        return Color::from_rgb(0.38, 0.68, 1.0);
    }

    // Lift very dark colors so they always read clearly against #18181E
    let boost = if lum < 0.30 {
        (0.42 / lum.max(0.08)).min(2.2)
    } else {
        1.0
    };
    Color::from_rgb(
        (r * boost).clamp(0.12, 1.0),
        (g * boost).clamp(0.12, 1.0),
        (b * boost).clamp(0.12, 1.0),
    )
}

pub fn accent_hover(c: Color) -> Color {
    Color::from_rgb(
        (c.r + (1.0 - c.r) * 0.18).min(1.0),
        (c.g + (1.0 - c.g) * 0.18).min(1.0),
        (c.b + (1.0 - c.b) * 0.18).min(1.0),
    )
}

/// Returns dark or white text for maximum readability on a solid accent button.
pub fn contrast_on(c: Color) -> Color {
    let lum = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    if lum > 0.62 {
        Color::from_rgb(0.06, 0.06, 0.08)
    } else {
        Color::WHITE
    }
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color::from_rgba(c.r, c.g, c.b, a)
}

pub fn app_theme() -> iced::Theme {
    iced::Theme::custom(
        "OpenMAD Dark".to_string(),
        iced::theme::Palette {
            background: BG_BASE,
            text: TEXT_PRIMARY,
            primary: ACCENT_BLUE,
            success: GREEN_OK,
            danger: RED_DANGER,
        },
    )
}

pub fn sidebar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SIDEBAR)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.30),
            offset: Vector::new(4.0, 0.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    }
}

pub fn header_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_HEADER)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SURFACE)),
        border: Border {
            color: BORDER_GLASS_TOP,
            width: 1.0,
            radius: R_LG.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub fn elevated_card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_ELEVATED)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: R_MD.into(),
        },
        ..Default::default()
    }
}

pub fn pill_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_BASE)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: R_SM.into(),
        },
        ..Default::default()
    }
}

pub fn segmented_bar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SEGMENT)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: R_MD.into(),
        },
        ..Default::default()
    }
}

pub fn badge_style(color: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(with_alpha(color, 0.14))),
        border: Border {
            color: with_alpha(color, 0.38),
            width: 1.0,
            radius: 20.0.into(),
        },
        ..Default::default()
    }
}

pub fn warn_banner_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 0.624, 0.039, 0.12))),
        border: Border {
            color: Color::from_rgba(1.0, 0.624, 0.039, 0.35),
            width: 1.0,
            radius: R_MD.into(),
        },
        ..Default::default()
    }
}

pub fn toast_style(is_err: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| {
        let accent = if is_err { RED_DANGER } else { GREEN_OK };
        container::Style {
            background: Some(Background::Color(BG_ELEVATED)),
            border: Border {
                color: with_alpha(accent, 0.75),
                width: 1.0,
                radius: R_MD.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
                offset: Vector::new(0.0, 8.0),
                blur_radius: 22.0,
            },
            ..Default::default()
        }
    }
}

pub fn nav_button_style(
    active: bool,
    accent: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            with_alpha(accent, 0.15)
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            BG_SURFACE
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { TEXT_PRIMARY } else { TEXT_SECONDARY },
            border: Border {
                color: if active {
                    with_alpha(accent, 0.40)
                } else {
                    Color::TRANSPARENT
                },
                width: 1.0,
                radius: R_MD.into(),
            },
            ..Default::default()
        }
    }
}

pub fn tab_button_style(
    active: bool,
    accent: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            accent
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            BG_ELEVATED
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active {
                contrast_on(accent)
            } else {
                TEXT_SECONDARY
            },
            border: Border {
                color: if active { accent } else { Color::TRANSPARENT },
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn swatch_button_style(
    color: Color,
    selected: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(color)),
        text_color: Color::WHITE,
        border: Border {
            color: if selected || matches!(status, button::Status::Hovered | button::Status::Pressed)
            {
                Color::WHITE
            } else {
                BORDER_STRONG
            },
            width: if selected { 2.2 } else { 1.2 },
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

pub fn primary_button_style(
    accent: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => accent_hover(accent),
            button::Status::Disabled => with_alpha(accent, 0.35),
            _ => accent,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: contrast_on(bg),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn toggle_primary_style(
    active: bool,
    accent: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        if active {
            primary_button_style(accent)(theme, status)
        } else {
            secondary_button_style(theme, status)
        }
    }
}

pub fn confirm_button_style(
    danger: bool,
    accent: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        if danger {
            danger_button_style(theme, status)
        } else {
            primary_button_style(accent)(theme, status)
        }
    }
}

pub fn secondary_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let (bg, txt) = match status {
        button::Status::Hovered => (BG_HOVER, TEXT_PRIMARY),
        button::Status::Pressed => (BG_SURFACE, TEXT_PRIMARY),
        button::Status::Disabled => (with_alpha(BG_ELEVATED, 0.5), TEXT_MUTED),
        _ => (BG_ELEVATED, TEXT_PRIMARY),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: txt,
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn ghost_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => BG_ELEVATED,
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_SECONDARY,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn danger_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Color::from_rgba(1.0, 0.271, 0.227, 0.28)
        }
        button::Status::Disabled => Color::from_rgba(1.0, 0.271, 0.227, 0.06),
        _ => Color::from_rgba(1.0, 0.271, 0.227, 0.14),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: RED_DANGER,
        border: Border {
            color: Color::from_rgba(1.0, 0.271, 0.227, 0.40),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn effect_card_style(
    active: bool,
    accent: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            with_alpha(accent, 0.16)
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            BG_HOVER
        } else {
            BG_ELEVATED
        };
        let border_color = if active {
            accent
        } else if matches!(status, button::Status::Hovered) {
            BORDER_STRONG
        } else {
            BORDER_SUBTLE
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: TEXT_PRIMARY,
            border: Border {
                color: border_color,
                width: if active { 1.5 } else { 1.0 },
                radius: R_MD.into(),
            },
            ..Default::default()
        }
    }
}

pub fn input_style(accent: Color) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let border_color = match status {
            text_input::Status::Focused => accent,
            text_input::Status::Hovered => BORDER_STRONG,
            _ => BORDER_SUBTLE,
        };
        text_input::Style {
            background: Background::Color(BG_BASE),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 8.0.into(),
            },
            icon: TEXT_SECONDARY,
            placeholder: TEXT_MUTED,
            value: TEXT_PRIMARY,
            selection: with_alpha(accent, 0.35),
        }
    }
}

pub fn slider_style(accent: Color) -> impl Fn(&iced::Theme, slider::Status) -> slider::Style {
    move |_, status| {
        let active = matches!(status, slider::Status::Hovered | slider::Status::Dragged);
        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(if active { accent_hover(accent) } else { accent }),
                    Background::Color(BG_ELEVATED),
                ),
                width: 6.0,
                border: Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle {
                    radius: if active { 9.0 } else { 8.0 },
                },
                background: Background::Color(Color::WHITE),
                border_width: if active { 2.0 } else { 1.0 },
                border_color: if active { accent } else { BORDER_STRONG },
            },
        }
    }
}

pub fn toggler_style(accent: Color) -> impl Fn(&iced::Theme, toggler::Status) -> toggler::Style {
    move |_, status| {
        let is_toggled = matches!(
            status,
            toggler::Status::Active { is_toggled: true }
                | toggler::Status::Hovered { is_toggled: true }
        );
        let hovered = matches!(status, toggler::Status::Hovered { .. });
        toggler::Style {
            background: if is_toggled {
                if hovered { accent_hover(accent) } else { accent }
            } else if hovered {
                BG_HOVER
            } else {
                BG_ELEVATED
            },
            background_border_width: 1.0,
            background_border_color: if is_toggled { accent } else { BORDER_STRONG },
            foreground: if is_toggled { contrast_on(accent) } else { TEXT_PRIMARY },
            foreground_border_width: 0.0,
            foreground_border_color: Color::TRANSPARENT,
        }
    }
}

pub fn pick_list_style(_theme: &iced::Theme, status: pick_list::Status) -> pick_list::Style {
    let border = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened => BORDER_STRONG,
        _ => BORDER_SUBTLE,
    };
    pick_list::Style {
        text_color: TEXT_PRIMARY,
        placeholder_color: TEXT_MUTED,
        handle_color: TEXT_SECONDARY,
        background: Background::Color(BG_ELEVATED),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
    }
}
