//! Apple-inspired Dark Mode theme and Inter typography for iced.

use iced::widget::{button, container, pick_list, slider, text_input};
use iced::{Background, Border, Color, Font, Shadow, Vector};

pub const INTER_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/Inter-Variable.ttf");
pub const FONT_INTER: Font = Font::with_name("Inter");

pub const BG_BASE: Color = Color::from_rgb(0.043, 0.043, 0.051); // #0B0B0D
pub const BG_SIDEBAR: Color = Color::from_rgb(0.067, 0.067, 0.082); // #111115
pub const BG_SURFACE: Color = Color::from_rgb(0.102, 0.102, 0.125); // #1A1A20
pub const BG_ELEVATED: Color = Color::from_rgb(0.145, 0.145, 0.176); // #25252D
pub const BG_HOVER: Color = Color::from_rgb(0.188, 0.188, 0.227); // #30303A
pub const BG_SEGMENT: Color = Color::from_rgb(0.082, 0.082, 0.102); // #15151A

pub const BORDER_SUBTLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.07);
pub const BORDER_STRONG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);

pub const TEXT_PRIMARY: Color = Color::from_rgb(0.961, 0.961, 0.969); // #F5F5F7
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.651, 0.651, 0.682); // #A6A6AE
pub const TEXT_MUTED: Color = Color::from_rgb(0.459, 0.459, 0.490); // #75757D

pub const ACCENT_BLUE: Color = Color::from_rgb(0.039, 0.518, 1.0); // #0A84FF
pub const ACCENT_BLUE_HOVER: Color = Color::from_rgb(0.200, 0.588, 1.0);
pub const GREEN_OK: Color = Color::from_rgb(0.188, 0.820, 0.345); // #30D158
pub const AMBER_WARN: Color = Color::from_rgb(1.0, 0.624, 0.039); // #FF9F0A
pub const RED_DANGER: Color = Color::from_rgb(1.0, 0.271, 0.227); // #FF453A

pub fn app_theme() -> iced::Theme {
    iced::Theme::custom(
        "OpenMAD Apple Dark".to_string(),
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
        ..Default::default()
    }
}

pub fn card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SURFACE)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.32),
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
            radius: 10.0.into(),
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
            radius: 11.0.into(),
        },
        ..Default::default()
    }
}

pub fn badge_style(color: Color) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(
            color.r, color.g, color.b, 0.14,
        ))),
        border: Border {
            color: Color::from_rgba(color.r, color.g, color.b, 0.35),
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
            radius: 10.0.into(),
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
                color: accent,
                width: 1.0,
                radius: 10.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            },
            ..Default::default()
        }
    }
}

pub fn nav_button_style(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            Color::from_rgba(0.039, 0.518, 1.0, 0.16)
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            BG_SURFACE
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { Color::WHITE } else { TEXT_SECONDARY },
            border: Border {
                color: if active {
                    Color::from_rgba(0.039, 0.518, 1.0, 0.45)
                } else {
                    Color::TRANSPARENT
                },
                width: 1.0,
                radius: 9.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn tab_button_style(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if active {
            ACCENT_BLUE
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            BG_ELEVATED
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { Color::WHITE } else { TEXT_SECONDARY },
            border: Border {
                color: if active { ACCENT_BLUE } else { Color::TRANSPARENT },
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    }
}

pub fn swatch_button_style(color: Color) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(color)),
        text_color: Color::WHITE,
        border: Border {
            color: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                Color::WHITE
            } else {
                BORDER_STRONG
            },
            width: 1.5,
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

pub fn primary_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT_BLUE_HOVER,
        button::Status::Disabled => Color::from_rgba(0.039, 0.518, 1.0, 0.35),
        _ => ACCENT_BLUE,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn secondary_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => BG_HOVER,
        _ => BG_ELEVATED,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub fn danger_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => Color::from_rgba(1.0, 0.271, 0.227, 0.28),
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

pub fn input_style(_theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused => ACCENT_BLUE,
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
        selection: Color::from_rgba(0.039, 0.518, 1.0, 0.35),
    }
}

pub fn slider_style(_theme: &iced::Theme, _status: slider::Status) -> slider::Style {
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(ACCENT_BLUE),
                Background::Color(BG_ELEVATED),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.5 },
            background: Background::Color(Color::WHITE),
            border_width: 1.0,
            border_color: BORDER_STRONG,
        },
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
