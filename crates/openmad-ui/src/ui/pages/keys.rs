//! Keymap remapping page with layer selector, profile keymap sync, search bar, and categorized keycode grid.

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, input_style, primary_button_style, secondary_button_style, type_scale,
    AMBER_WARN, FONT_MEDIUM, GREEN_OK, SP_MD, SP_SM, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, keycode_picker, segmented};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, row, text, text_input, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::keycodes;
use openmad_core::paths::profile_dir;
use openmad_core::profile::{profile_path, Profile};

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
                format!("{}: ({r},{c}) → {lbl}", l.t("Atual", "Current")),
                accent,
            )
        } else {
            badge(
                l.t("Selecione uma tecla acima", "Select a key above"),
                TEXT_MUTED,
            )
        };

        // Check selected profile's keymap status against live keyboard keymap
        let mut prof_km_diff: Option<usize> = None;
        let mut has_prof_km = false;
        if let (Some(ref prof_name), Some(ref cfg)) = (&self.selected_profile, &self.snap.config) {
            if let Ok(p) = profile_path(&profile_dir(), prof_name) {
                if let Ok(prof) = Profile::load(&p) {
                    if prof.keymap.is_some() {
                        has_prof_km = true;
                        prof_km_diff = prof.keymap_diff_against(
                            &cfg.keymap,
                            self.snap.board.layers,
                            self.snap.board.rows,
                            self.snap.board.cols,
                        );
                    }
                }
            }
        }

        let km_status_badge = if !has_prof_km {
            badge(
                l.t("Perfil sem keymap salvo", "No keymap in profile"),
                TEXT_MUTED,
            )
        } else if let Some(0) = prof_km_diff {
            badge(l.t("Keymap sincronizado", "Keymap in sync"), GREEN_OK)
        } else if let Some(n) = prof_km_diff {
            badge(
                format!(
                    "{n} {}",
                    l.t("tecla(s) diferem do perfil", "key(s) differ from profile")
                ),
                AMBER_WARN,
            )
        } else {
            badge(
                l.t("Layout incompatível", "Incompatible layout"),
                AMBER_WARN,
            )
        };

        let header_actions = row![
            selected_badge,
            km_status_badge,
            button(
                row![
                    icon(Icon::Download, 13, crate::theme::TEXT_PRIMARY),
                    text(l.t("Capturar p/ Perfil", "Capture to Profile"))
                        .size(type_scale::CAPTION + 1),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 11])
            .style(secondary_button_style)
            .on_press(Message::CaptureProfileKeymap),
            button(
                row![
                    icon(Icon::HardDrive, 13, contrast_on(accent)),
                    text(l.t("Aplicar do Perfil (EEPROM)", "Apply Profile Keymap"))
                        .size(type_scale::CAPTION + 1)
                        .font(FONT_MEDIUM),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(primary_button_style(accent))
            .on_press(Message::ApplyProfileKeymap),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let custom_hex_row = row![
            text(l.t("Código HEX personalizado:", "Custom HEX keycode:"))
                .size(type_scale::CAPTION + 1)
                .color(TEXT_SECONDARY),
            text_input("0x0004", &self.custom_hex_input)
                .on_input(Message::CustomHexInputChanged)
                .on_submit(Message::AssignCustomHexKeycode)
                .width(Length::Fixed(105.0))
                .padding([5, 9])
                .size(type_scale::CAPTION + 1)
                .style(input_style(accent)),
            button(
                text(l.t("Atribuir HEX", "Assign HEX"))
                    .size(type_scale::CAPTION + 1)
                    .font(FONT_MEDIUM),
            )
            .padding([6, 12])
            .style(secondary_button_style)
            .on_press(Message::AssignCustomHexKeycode),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let picker = keycode_picker(
            l,
            self.pick_tab,
            &self.keycode_search,
            accent,
            Message::SelectKeyPickerTab,
            Message::KeycodeSearchChanged,
            Message::KeycodePicked,
        );

        let body = column![
            row![
                text(l.t("Camada:", "Layer:"))
                    .size(type_scale::CAPTION + 1)
                    .font(FONT_MEDIUM)
                    .color(TEXT_SECONDARY),
                layer_seg,
                Space::with_width(Length::Fill),
                custom_hex_row,
            ]
            .spacing(SP_SM)
            .align_y(Alignment::Center),
            picker,
        ]
        .spacing(SP_MD);

        card(
            l.t("Remapear Teclas", "Keymap Assignment"),
            Some(
                l.t(
                    "Clique em uma ou mais teclas acima e escolha a nova função abaixo (grava na EEPROM). Use Capturar/Aplicar para salvar keymaps por perfil.",
                    "Select key(s) on the keyboard above and pick a new keycode below (writes to EEPROM). Use Capture/Apply to manage per-profile keymaps.",
                )
                .to_string(),
            ),
            Some(header_actions.into()),
            body.into(),
        )
    }
}
