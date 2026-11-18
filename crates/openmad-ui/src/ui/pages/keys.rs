//! Keymap remapping page with layer selector, search bar, and categorized keycode grid.

use crate::app::{KeyPickerTab, Message, OpenMadApp};
use crate::theme::{
    contrast_on, input_style, primary_button_style, secondary_button_style, type_scale,
    FONT_MEDIUM, SP_MD, SP_SM, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, segmented};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, row, text, text_input, Column, Row, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::keycodes::{
    self, KEYBOARD_PICKER_CODES, LAYER_PICKER_CODES, SYSTEM_PICKER_CODES,
};

impl OpenMadApp {
    pub fn view_keys_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        let layer_items = [(0, "L0"), (1, "L1"), (2, "L2"), (3, "L3")];
        let layer_seg = segmented(&layer_items, self.layer, accent, Message::SelectLayer);

        // Selected key badge
        let selected_badge = if let Some(&first_idx) = self.selected_keys.iter().next() {
            let cols = self.snap.board.cols.max(1);
            let r = first_idx / cols;
            let c = first_idx % cols;
            let cur_kc = self
                .snap
                .config
                .as_ref()
                .and_then(|cfg| {
                    let dec = keycodes::decode_keymap(
                        &cfg.keymap,
                        self.layer,
                        self.snap.board.rows,
                        cols,
                    );
                    dec.get(r).and_then(|row| row.get(c)).copied()
                })
                .unwrap_or(0);
            let lbl = keycodes::label(cur_kc);
            badge(
                format!(
                    "{}: ({r},{c}) → {lbl}",
                    l.t("Atual", "Current")
                ),
                accent,
            )
        } else {
            badge(
                l.t("Selecione uma tecla acima", "Select a key above"),
                TEXT_MUTED,
            )
        };

        let pick_items = [
            (KeyPickerTab::Keyboard, l.t("Teclas Padrão", "Keyboard")),
            (KeyPickerTab::System, l.t("Sistema / Mídia", "System / Media")),
            (KeyPickerTab::Macro, "Macros"),
            (KeyPickerTab::Layer, l.t("Camadas (Fn)", "Layers (Fn)")),
        ];
        let pick_seg = segmented(&pick_items, self.pick_tab, accent, Message::SelectKeyPickerTab);

        let search_box = row![
            icon(Icon::Search, 14, TEXT_MUTED),
            text_input(
                l.t("Buscar tecla (ex: Enter, Vol, F12)…", "Search keycode (e.g. Enter, Vol, F12)…"),
                &self.keycode_search,
            )
            .on_input(Message::KeycodeSearchChanged)
            .padding([6, 10])
            .size(type_scale::CAPTION + 1)
            .style(input_style(accent)),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fixed(270.0));

        let q = self.keycode_search.trim().to_lowercase();
        let all_codes: Vec<(u16, String)> = match self.pick_tab {
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
                text(l.t(
                    "Nenhuma tecla encontrada para esta busca.",
                    "No keycodes match your search.",
                ))
                .size(type_scale::BODY)
                .color(TEXT_MUTED),
            );
        } else {
            for chunk in codes.chunks(10) {
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
                        .on_press(Message::AssignKeycode(*kc)),
                    );
                }
                for _ in chunk.len()..10 {
                    r = r.push(Space::with_width(Length::Fill));
                }
                grid = grid.push(r);
            }
        }

        let header_actions = row![
            selected_badge,
            button(
                row![
                    icon(Icon::HardDrive, 13, contrast_on(accent)),
                    text(l.t("Aplicar Keymap (EEPROM)", "Apply Keymap (EEPROM)"))
                        .size(type_scale::CAPTION + 1)
                        .font(FONT_MEDIUM),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(primary_button_style(accent))
            .on_press(Message::ApplyFullKeymap),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let body = column![
            row![
                text(l.t("Camada:", "Layer:"))
                    .size(type_scale::CAPTION + 1)
                    .font(FONT_MEDIUM)
                    .color(TEXT_SECONDARY),
                layer_seg,
                Space::with_width(Length::Fill),
                search_box,
            ]
            .spacing(SP_SM)
            .align_y(Alignment::Center),
            pick_seg,
            grid,
        ]
        .spacing(SP_MD);

        card(
            l.t("Remapear Teclas", "Keymap Assignment"),
            Some(
                l.t(
                    "Clique em uma tecla no teclado acima e escolha a nova função abaixo. Gravado diretamente na EEPROM.",
                    "Select a key on the keyboard above, then click its new keycode below (writes to EEPROM).",
                )
                .to_string(),
            ),
            Some(header_actions.into()),
            body.into(),
        )
    }
}
