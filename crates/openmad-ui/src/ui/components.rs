//! Reusable UI components for OpenMAD-Rust.

use crate::app::{KeyPickerTab, Message};
use crate::i18n::Lang;
use crate::theme::{
    badge_style, card_style, input_style, pill_style, secondary_button_style, segmented_bar_style,
    slider_style, tab_button_style, toggler_style, type_scale, with_alpha, FONT_MEDIUM,
    FONT_SEMIBOLD, SP_LG, SP_MD, SP_SM, SP_XL, SP_XS, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, row, slider, text, text_input, toggler, Column, Row, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::keycodes::{KEYBOARD_PICKER_CODES, LAYER_PICKER_CODES, SYSTEM_PICKER_CODES};

pub fn card<'a>(
    title: impl Into<String>,
    desc: Option<String>,
    actions: Option<Element<'a, Message>>,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let title_str = title.into();
    let mut header_left = column![
        text(title_str)
            .size(type_scale::HEADING)
            .font(FONT_SEMIBOLD)
            .color(TEXT_PRIMARY),
    ]
    .spacing(3);
    if let Some(d) = desc {
        header_left = header_left.push(
            text(d)
                .size(type_scale::CAPTION + 1)
                .color(TEXT_MUTED),
        );
    }

    let header_left = header_left.width(Length::Fill);
    let mut header_row = row![header_left]
        .spacing(SP_MD)
        .align_y(Alignment::Center);
    if let Some(act) = actions {
        header_row = header_row.push(act);
    }

    container(
        column![header_row, body]
            .spacing(SP_MD)
            .padding(SP_LG),
    )
    .width(Length::Fill)
    .style(card_style)
    .into()
}

pub fn slider_row<'a>(
    label: impl Into<String>,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    step: f32,
    value_label: impl Into<String>,
    accent: Color,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    let min = *range.start();
    let max = *range.end();
    let dec_val = (value - step).max(min);
    let inc_val = (value + step).min(max);

    let dec_msg = on_change(dec_val);
    let inc_msg = on_change(inc_val);

    row![
        text(label.into())
            .size(type_scale::BODY)
            .font(FONT_MEDIUM)
            .color(TEXT_SECONDARY)
            .width(Length::Fixed(165.0)),
        slider(range, value, on_change)
            .step(step)
            .style(slider_style(accent))
            .width(Length::Fill),
        container(
            text(value_label.into())
                .size(type_scale::CAPTION + 1)
                .font(FONT_MEDIUM)
                .color(TEXT_PRIMARY),
        )
        .padding([4, 9])
        .width(Length::Fixed(74.0))
        .align_x(Alignment::Center)
        .style(pill_style),
        button(icon(Icon::Minus, 12, TEXT_SECONDARY))
            .padding([5, 7])
            .style(secondary_button_style)
            .on_press(dec_msg),
        button(icon(Icon::Plus, 12, TEXT_SECONDARY))
            .padding([5, 7])
            .style(secondary_button_style)
            .on_press(inc_msg),
    ]
    .spacing(SP_SM)
    .align_y(Alignment::Center)
    .into()
}

pub fn toggle_row<'a>(
    label: impl Into<String>,
    desc: Option<String>,
    value: bool,
    accent: Color,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let mut left = column![
        text(label.into())
            .size(type_scale::BODY)
            .font(FONT_MEDIUM)
            .color(TEXT_PRIMARY),
    ]
    .spacing(2)
    .width(Length::Fill);

    if let Some(d) = desc {
        left = left.push(
            text(d)
                .size(type_scale::CAPTION)
                .color(TEXT_MUTED),
        );
    }

    row![
        left,
        toggler(value)
            .style(toggler_style(accent))
            .on_toggle(on_toggle),
    ]
    .spacing(SP_MD)
    .align_y(Alignment::Center)
    .into()
}

pub fn segmented<'a, T: Copy + PartialEq + 'a>(
    items: &[(T, &'a str)],
    active: T,
    accent: Color,
    on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message> {
    let mut r = Row::new().spacing(SP_XS);
    for &(val, lbl) in items {
        r = r.push(
            button(
                text(lbl)
                    .size(type_scale::CAPTION + 1)
                    .font(if active == val { FONT_SEMIBOLD } else { FONT_MEDIUM }),
            )
            .padding([6, 12])
            .style(tab_button_style(active == val, accent))
            .on_press(on_select(val)),
        );
    }
    container(r).padding(SP_XS).style(segmented_bar_style).into()
}

pub fn badge<'a>(label: impl Into<String>, color: Color) -> Element<'a, Message> {
    container(
        text(label.into())
            .size(type_scale::CAPTION)
            .font(FONT_MEDIUM)
            .color(color),
    )
    .padding([4, 10])
    .style(badge_style(color))
    .into()
}

pub fn empty_state<'a>(
    ic: Icon,
    title: impl Into<String>,
    body: impl Into<String>,
    extra: Option<Element<'a, Message>>,
    cta: Option<Element<'a, Message>>,
    accent: Color,
) -> Element<'a, Message> {
    let icon_wrap = container(icon(ic, 28, accent))
        .padding(14)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(with_alpha(accent, 0.12))),
            border: iced::Border {
                color: with_alpha(accent, 0.32),
                width: 1.0,
                radius: 999.0.into(),
            },
            ..Default::default()
        });

    let mut col = column![
        icon_wrap,
        text(title.into())
            .size(type_scale::TITLE - 2)
            .font(FONT_SEMIBOLD)
            .color(TEXT_PRIMARY),
        text(body.into())
            .size(type_scale::BODY)
            .color(TEXT_SECONDARY),
    ]
    .spacing(SP_SM)
    .align_x(Alignment::Center);

    if let Some(ex) = extra {
        col = col.push(ex);
    }
    if let Some(btn) = cta {
        col = col.push(Space::with_height(Length::Fixed(4.0)));
        col = col.push(btn);
    }

    container(col)
        .width(Length::Fill)
        .padding(SP_XL)
        .center_x(Length::Fill)
        .style(card_style)
        .into()
}

pub fn keycode_picker<'a>(
    lang: Lang,
    pick_tab: KeyPickerTab,
    search: &'a str,
    accent: Color,
    on_tab: fn(KeyPickerTab) -> Message,
    on_search: fn(String) -> Message,
    on_pick: fn(u16) -> Message,
) -> Element<'a, Message> {
    let pick_items = [
        (KeyPickerTab::Keyboard, lang.t("Teclas Padrão", "Keyboard")),
        (KeyPickerTab::System, lang.t("Sistema / Mídia", "System / Media")),
        (KeyPickerTab::Macro, "Macros"),
        (KeyPickerTab::Layer, lang.t("Camadas (Fn)", "Layers (Fn)")),
    ];
    let pick_seg = segmented(&pick_items, pick_tab, accent, on_tab);

    let search_box = row![
        icon(Icon::Search, 14, TEXT_MUTED),
        text_input(
            lang.t(
                "Buscar tecla (ex: Enter, KP7, Vol, F12)…",
                "Search keycode (e.g. Enter, KP7, Vol, F12)…",
            ),
            search,
        )
        .on_input(on_search)
        .padding([6, 10])
        .size(type_scale::CAPTION + 1)
        .style(input_style(accent)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fixed(270.0));

    let q = search.trim().to_lowercase();
    let all_codes: Vec<(u16, String)> = match pick_tab {
        KeyPickerTab::Keyboard => KEYBOARD_PICKER_CODES
            .iter()
            .map(|&(k, lbl)| (k, lbl.to_string()))
            .collect(),
        KeyPickerTab::System => SYSTEM_PICKER_CODES
            .iter()
            .map(|&(k, lbl)| (k, lbl.to_string()))
            .collect(),
        KeyPickerTab::Macro => (0..16u16).map(|i| (0x7700 + i, format!("M{i}"))).collect(),
        KeyPickerTab::Layer => LAYER_PICKER_CODES
            .iter()
            .map(|&(k, lbl)| (k, lbl.to_string()))
            .collect(),
    };

    let codes: Vec<(u16, String)> = if q.is_empty() {
        all_codes
    } else {
        all_codes
            .into_iter()
            .filter(|(_, lbl)| lbl.to_lowercase().contains(&q))
            .collect()
    };

    let mut grid = Column::new().spacing(6);
    if codes.is_empty() {
        grid = grid.push(
            text(lang.t(
                "Nenhuma tecla encontrada para esta busca.",
                "No keycodes match your search.",
            ))
            .size(type_scale::BODY)
            .color(TEXT_MUTED),
        );
    } else {
        const COLS: usize = 10;
        for chunk in codes.chunks(COLS) {
            let mut r = Row::new().spacing(6);
            for (kc, label) in chunk {
                r = r.push(
                    button(
                        text(label.clone())
                            .size(type_scale::CAPTION)
                            .font(FONT_MEDIUM),
                    )
                    .width(Length::Fill)
                    .padding([7, 6])
                    .style(secondary_button_style)
                    .on_press(on_pick(*kc)),
                );
            }
            for _ in chunk.len()..COLS {
                r = r.push(Space::with_width(Length::Fill));
            }
            grid = grid.push(r);
        }
    }

    column![
        row![pick_seg, Space::with_width(Length::Fill), search_box]
            .spacing(SP_SM)
            .align_y(Alignment::Center),
        grid,
    ]
    .spacing(SP_MD)
    .into()
}
