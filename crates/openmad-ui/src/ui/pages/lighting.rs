//! Lighting page with effect preview gallery, 2D HSV picker, collapsible RGB sliders, and friendly side lighting.

use crate::app::{Message, OpenMadApp, COLOR_PRESETS};
use crate::theme::{
    contrast_on, effect_card_style, elevated_card_style, ghost_button_style, input_style,
    pick_list_style, primary_button_style, secondary_button_style, swatch_button_style,
    type_scale, FONT_MEDIUM, FONT_SEMIBOLD, SP_LG, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::ui::components::{card, slider_row, toggle_row};
use crate::ui::icons::{icon, Icon};
use crate::widgets::color_picker::ColorPickerCanvas;
use iced::widget::{button, column, container, pick_list, row, text, text_input, Column, Row, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::features::LIGHT_EFFECTS;
use openmad_core::protocol::BRIGHTNESS_MAX;

fn effect_icon(id: u8) -> Icon {
    match id {
        0 => Icon::Power,
        1 => Icon::Palette,
        2 | 3 => Icon::Sun,
        4 | 5 | 13 | 26 | 28 | 39 => Icon::Rainbow,
        6 => Icon::Wind,
        7 | 8 | 9 | 11 => Icon::Waves,
        19 | 23 => Icon::Sparkles,
        25 | 30 | 31 | 36 => Icon::Flame,
        _ => Icon::Lightbulb,
    }
}

fn effect_preview_bar<'a>(id: u8, active_rgb: [u8; 3]) -> Element<'a, Message> {
    let [r, g, b] = active_rgb;
    let c1 = Color::from_rgb8(r, g, b);
    let swatches: Vec<Color> = match id {
        0 => vec![Color::from_rgb(0.16, 0.16, 0.20); 4],
        1 => vec![
            Color::from_rgb8(255, 59, 48),
            Color::from_rgb8(48, 209, 88),
            Color::from_rgb8(10, 132, 255),
            Color::from_rgb8(191, 90, 242),
        ],
        4 | 5 | 13 | 19 | 23 | 26 | 28 | 39 => vec![
            Color::from_rgb8(255, 69, 58),
            Color::from_rgb8(255, 214, 10),
            Color::from_rgb8(48, 209, 88),
            Color::from_rgb8(10, 132, 255),
        ],
        25 => vec![
            Color::from_rgb8(255, 59, 48),
            Color::from_rgb8(48, 209, 88),
            Color::from_rgb8(255, 59, 48),
            Color::from_rgb8(48, 209, 88),
        ],
        30 => vec![
            Color::from_rgb8(18, 120, 52),
            Color::from_rgb8(48, 209, 88),
            Color::from_rgb8(100, 240, 140),
            Color::from_rgb8(18, 120, 52),
        ],
        6 => vec![
            Color::from_rgba(c1.r, c1.g, c1.b, 0.25),
            Color::from_rgba(c1.r, c1.g, c1.b, 0.65),
            c1,
            Color::from_rgba(c1.r, c1.g, c1.b, 0.45),
        ],
        _ => vec![c1, c1, c1, c1],
    };

    let mut r_row = Row::new().spacing(2);
    for col in swatches {
        r_row = r_row.push(
            container(Space::with_width(Length::Fill).height(Length::Fixed(3.0)))
                .width(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(col)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }
    r_row.width(Length::Fill).into()
}

impl OpenMadApp {
    pub fn view_lighting_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let pt = l.is_pt();
        let cur_meta = LIGHT_EFFECTS.iter().find(|e| e.id == self.light_draft.effect);
        let has_color = cur_meta.map_or(true, |e| e.has_color);
        let has_speed = cur_meta.map_or(true, |e| e.has_speed);

        let effect_hint = if !has_color {
            l.t(
                "Efeito com cores automáticas — clicar em qualquer cor muda para Cor Fixa.",
                "Auto-spectrum effect — clicking any color switches to Constant Color.",
            )
        } else if self.light_draft.effect == 1 {
            l.t(
                "Modo Personalizado: clicar em uma cor pinta imediatamente as teclas selecionadas.",
                "Per-Key mode: clicking a color immediately paints selected keys.",
            )
        } else {
            l.t(
                "Clique ou arraste no seletor ao lado para aplicar ao vivo no teclado.",
                "Click or drag the color picker to apply live on the keyboard.",
            )
        };

        // Effect preview card gallery (3 columns)
        let mut gallery = Column::new().spacing(6);
        for chunk in LIGHT_EFFECTS.chunks(3) {
            let mut r = Row::new().spacing(6);
            for eff in chunk {
                let active = self.light_draft.effect == eff.id;
                let ic_col = if active { accent } else { TEXT_SECONDARY };
                let card_btn = button(
                    column![
                        row![
                            icon(effect_icon(eff.id), 13, ic_col),
                            text(eff.label(pt))
                                .size(type_scale::CAPTION)
                                .font(if active { FONT_SEMIBOLD } else { FONT_MEDIUM })
                                .color(if active { TEXT_PRIMARY } else { TEXT_SECONDARY }),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                        effect_preview_bar(
                            eff.id,
                            [self.light_draft.r, self.light_draft.g, self.light_draft.b],
                        ),
                    ]
                    .spacing(6),
                )
                .width(Length::Fill)
                .height(Length::Fixed(48.0))
                .padding([7, 9])
                .style(effect_card_style(active, accent))
                .on_press(Message::LightEffectSelected(eff.id));
                r = r.push(card_btn);
            }
            for _ in chunk.len()..3 {
                r = r.push(Space::with_width(Length::Fill));
            }
            gallery = gallery.push(r);
        }

        let mut left_col = column![
            text(l.t("Galeria de Efeitos", "Effect Gallery"))
                .size(type_scale::BODY)
                .font(FONT_SEMIBOLD)
                .color(TEXT_PRIMARY),
            gallery,
            Space::with_height(Length::Fixed(4.0)),
            slider_row(
                l.t("Brilho", "Brightness"),
                0.0..=(BRIGHTNESS_MAX as f32),
                self.light_draft.brightness as f32,
                1.0,
                format!("{}/{}", self.light_draft.brightness, BRIGHTNESS_MAX),
                accent,
                |v| Message::LightBrightnessChanged(v as u8),
            ),
        ]
        .spacing(SP_SM)
        .width(Length::FillPortion(3));

        if has_speed {
            left_col = left_col.push(slider_row(
                l.t("Velocidade", "Speed"),
                1.0..=255.0,
                self.light_draft.speed.max(1) as f32,
                1.0,
                format!("{}", self.light_draft.speed),
                accent,
                |v| Message::LightSpeedChanged(v as u8),
            ));
        }

        // Right column: Color Picker + HEX + Presets + Collapsible RGB
        let swatch = container(Space::with_width(Length::Fixed(34.0)).height(Length::Fixed(34.0)))
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(
                    self.light_draft.r,
                    self.light_draft.g,
                    self.light_draft.b,
                ))),
                border: iced::Border {
                    color: TEXT_SECONDARY,
                    width: 1.2,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

        let mut presets = Row::new().spacing(6).align_y(Alignment::Center);
        for &(r, g, b) in COLOR_PRESETS {
            let sel = self.light_draft.r == r && self.light_draft.g == g && self.light_draft.b == b;
            presets = presets.push(
                button(Space::with_width(Length::Fixed(20.0)).height(Length::Fixed(20.0)))
                    .padding(0)
                    .style(swatch_button_style(Color::from_rgb8(r, g, b), sel))
                    .on_press(Message::LightColorPreset(r, g, b)),
            );
        }

        let hsv_picker = ColorPickerCanvas {
            hue: self.light_hue,
            sat: self.light_sat,
            val: self.light_val,
        }
        .view(Message::LightHsvChanged);

        let mut right_col = column![
            text(l.t("Cor Ativa", "Active Color"))
                .size(type_scale::BODY)
                .font(FONT_SEMIBOLD)
                .color(TEXT_PRIMARY),
            hsv_picker,
            row![
                swatch,
                column![
                    text(l.t("Código HEX", "HEX")).size(type_scale::CAPTION).color(TEXT_MUTED),
                    text_input("#0A84FF", &self.light_hex)
                        .on_input(Message::LightHexChanged)
                        .width(Length::Fixed(96.0))
                        .padding([5, 8])
                        .size(type_scale::CAPTION + 1)
                        .style(input_style(accent)),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            column![
                text(l.t("Cores rápidas", "Quick Presets"))
                    .size(type_scale::CAPTION)
                    .color(TEXT_MUTED),
                presets,
            ]
            .spacing(4),
            button(
                row![
                    icon(
                        if self.lighting_advanced_rgb {
                            Icon::ChevronUp
                        } else {
                            Icon::ChevronDown
                        },
                        13,
                        TEXT_SECONDARY,
                    ),
                    text(l.t("Ajuste RGB fino", "Fine RGB sliders"))
                        .size(type_scale::CAPTION + 1)
                        .color(TEXT_SECONDARY),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([4, 8])
            .style(ghost_button_style)
            .on_press(Message::ToggleLightingRgb),
        ]
        .spacing(SP_SM)
        .width(Length::FillPortion(2));

        if self.lighting_advanced_rgb {
            right_col = right_col.push(
                container(
                    column![
                        slider_row(
                            l.t("Vermelho", "Red"),
                            0.0..=255.0,
                            self.light_draft.r as f32,
                            1.0,
                            format!("{}", self.light_draft.r),
                            Color::from_rgb8(255, 69, 58),
                            |v| Message::LightRChanged(v as u8),
                        ),
                        slider_row(
                            l.t("Verde", "Green"),
                            0.0..=255.0,
                            self.light_draft.g as f32,
                            1.0,
                            format!("{}", self.light_draft.g),
                            Color::from_rgb8(48, 209, 88),
                            |v| Message::LightGChanged(v as u8),
                        ),
                        slider_row(
                            l.t("Azul", "Blue"),
                            0.0..=255.0,
                            self.light_draft.b as f32,
                            1.0,
                            format!("{}", self.light_draft.b),
                            Color::from_rgb8(10, 132, 255),
                            |v| Message::LightBChanged(v as u8),
                        ),
                    ]
                    .spacing(6)
                    .padding(10),
                )
                .style(elevated_card_style),
            );
        }

        let paint_label = if self.selected_keys.is_empty() {
            l.t("Pintar Todas as Teclas", "Paint All Keys").to_string()
        } else {
            format!(
                "{} ({})",
                l.t("Pintar Seleção", "Paint Selection"),
                self.selected_keys.len()
            )
        };

        let header_actions = row![
            button(
                row![
                    icon(Icon::Palette, 13, TEXT_PRIMARY),
                    text(paint_label).size(type_scale::CAPTION + 1),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 11])
            .style(secondary_button_style)
            .on_press(Message::PaintSelectedKeys),
            button(
                row![
                    icon(Icon::HardDrive, 13, contrast_on(accent)),
                    text(l.t("Salvar na Flash", "Save to Flash"))
                        .size(type_scale::CAPTION + 1)
                        .font(FONT_MEDIUM),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(primary_button_style(accent))
            .on_press(Message::SaveLighting),
        ]
        .spacing(8);

        let main_card = card(
            l.t("Iluminação Global", "Global Lighting"),
            Some(effect_hint.to_string()),
            Some(header_actions.into()),
            row![left_col, right_col]
                .spacing(SP_LG)
                .align_y(Alignment::Start)
                .into(),
        );

        // Side / Box lighting with readable mode names
        let box_modes: Vec<String> = vec![
            format!("0 · {}", l.t("Desligado", "Off")),
            format!("1 · {}", l.t("Cor Fixa", "Constant")),
            format!("2 · {}", l.t("Respiração", "Breathing")),
            format!("3 · {}", l.t("Onda RGB", "RGB Wave")),
            format!("4 · {}", l.t("Arco-Íris", "Rainbow")),
            format!("5 · {}", l.t("Espectro", "Spectrum")),
            format!("6 · {}", l.t("Flutuante", "Floating")),
        ];
        let cur_box = box_modes
            .get(self.box_draft.mode as usize)
            .cloned()
            .unwrap_or_else(|| format!("{} · {}", self.box_draft.mode, l.t("Modo", "Mode")));

        let box_body = column![
            row![
                text(l.t("Modo da Faixa", "Strip Mode"))
                    .size(type_scale::BODY)
                    .font(FONT_MEDIUM)
                    .color(TEXT_SECONDARY)
                    .width(Length::Fixed(165.0)),
                pick_list(box_modes, Some(cur_box), |s| {
                    let id = s
                        .split('·')
                        .next()
                        .and_then(|n| n.trim().parse::<u8>().ok())
                        .unwrap_or(0);
                    Message::BoxModeChanged(id)
                })
                .style(pick_list_style)
                .padding([6, 10]),
            ]
            .spacing(SP_SM)
            .align_y(Alignment::Center),
            toggle_row(
                l.t("Modo Colorido (RGB)", "Colorful RGB"),
                Some(
                    l.t(
                        "Alterna entre cor única e ciclo multicolorido na faixa lateral.",
                        "Switches between single color and multicolor cycle on the side strip.",
                    )
                    .to_string(),
                ),
                self.box_draft.colorful != 0,
                accent,
                |on| Message::BoxColorfulChanged(if on { 1 } else { 0 }),
            ),
            slider_row(
                l.t("Brilho Lateral", "Side Brightness"),
                0.0..=255.0,
                self.box_draft.brightness as f32,
                1.0,
                format!("{}", self.box_draft.brightness),
                accent,
                |v| Message::BoxBrightnessChanged(v as u8),
            ),
            slider_row(
                l.t("Velocidade Lateral", "Side Speed"),
                0.0..=255.0,
                self.box_draft.speed as f32,
                1.0,
                format!("{}", self.box_draft.speed),
                accent,
                |v| Message::BoxSpeedChanged(v as u8),
            ),
        ]
        .spacing(SP_SM);

        let box_card = card(
            l.t("Iluminação Lateral / Caixa", "Box / Side Lighting"),
            Some(
                l.t(
                    "Controla os LEDs secundários/laterais em modelos com faixa externa.",
                    "Controls secondary side/box LEDs on models equipped with side lighting.",
                )
                .to_string(),
            ),
            None,
            box_body.into(),
        );

        column![main_card, box_card].spacing(SP_MD).into()
    }
}
