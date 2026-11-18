//! Advanced keys page (Rappy Snappy, SOCD, Hold/Click, DKS, Toggle).

use crate::app::{AdvSubTab, Message, OpenMadApp};
use crate::theme::{
    contrast_on, danger_button_style, input_style, primary_button_style, tab_button_style,
    type_scale, FONT_MEDIUM, SP_MD, SP_SM, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::ui::components::{card, segmented, slider_row, toggle_row};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, row, text, text_input, Column, Row, Space};
use iced::{Alignment, Color, Element, Length};

impl OpenMadApp {
    pub fn view_advanced_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let sub_items = [
            (AdvSubTab::RappySnappy, "Rappy Snappy"),
            (AdvSubTab::Socd, "SOCD"),
            (AdvSubTab::HoldClick, "Hold / Click"),
            (AdvSubTab::Dks, "DKS"),
            (AdvSubTab::Toggle, "Toggle"),
        ];
        let sub_bar = segmented(&sub_items, self.adv_sub, accent, Message::SelectAdvSubTab);

        let mut slot_row = Row::new().spacing(4).align_y(Alignment::Center);
        slot_row = slot_row.push(
            text("Slot:")
                .size(type_scale::CAPTION + 1)
                .font(FONT_MEDIUM)
                .color(TEXT_SECONDARY),
        );
        for i in 0..16 {
            slot_row = slot_row.push(
                button(text(format!("{i}")).size(type_scale::CAPTION))
                    .padding([5, 9])
                    .style(tab_button_style(self.adv_slot == i, accent))
                    .on_press(Message::AdvSlotSelected(i)),
            );
        }

        let editor: Element<'_, Message> = match self.adv_sub {
            AdvSubTab::RappySnappy | AdvSubTab::Socd => {
                let (title, desc) = if self.adv_sub == AdvSubTab::Socd {
                    (
                        l.t("Editor de Par SOCD", "SOCD Pair Editor"),
                        l.t(
                            "Resolve direções opostas simultâneas (ex: A + D), priorizando a última tecla pressionada.",
                            "Resolves simultaneous opposite cardinal directions (e.g. A + D) using last-input priority.",
                        ),
                    )
                } else {
                    (
                        l.t("Editor Rappy Snappy (RS)", "Rappy Snappy (RS) Pair Editor"),
                        l.t(
                            "Compara duas teclas e ativa apenas a que estiver pressionada mais fundo no momento.",
                            "Monitors two keys and prioritizes whichever key is currently pressed deeper.",
                        ),
                    )
                };

                let actions = row![
                    button(
                        row![
                            icon(Icon::Save, 13, contrast_on(accent)),
                            text(l.t("Salvar Par", "Save Pair"))
                                .size(type_scale::CAPTION + 1)
                                .font(FONT_MEDIUM),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 13])
                    .style(primary_button_style(accent))
                    .on_press(Message::SaveAdvancedKey),
                    button(
                        row![
                            icon(Icon::Trash, 13, crate::theme::RED_DANGER),
                            text(l.t("Limpar Slot", "Clear Slot")).size(type_scale::CAPTION + 1),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 12])
                    .style(danger_button_style)
                    .on_press(Message::ClearAdvancedKey),
                ]
                .spacing(8);

                let body = column![
                    row![
                        text(l.t("Tecla 1 (linha, col):", "Key 1 (row, col):"))
                            .size(type_scale::BODY)
                            .color(TEXT_SECONDARY),
                        text_input("row", &self.adv_k1_r)
                            .on_input(Message::AdvKey1RowChanged)
                            .width(Length::Fixed(58.0))
                            .style(input_style(accent)),
                        text_input("col", &self.adv_k1_c)
                            .on_input(Message::AdvKey1ColChanged)
                            .width(Length::Fixed(58.0))
                            .style(input_style(accent)),
                        Space::with_width(Length::Fixed(16.0)),
                        text(l.t("Tecla 2 (linha, col):", "Key 2 (row, col):"))
                            .size(type_scale::BODY)
                            .color(TEXT_SECONDARY),
                        text_input("row", &self.adv_k2_r)
                            .on_input(Message::AdvKey2RowChanged)
                            .width(Length::Fixed(58.0))
                            .style(input_style(accent)),
                        text_input("col", &self.adv_k2_c)
                            .on_input(Message::AdvKey2ColChanged)
                            .width(Length::Fixed(58.0))
                            .style(input_style(accent)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    slider_row(
                        l.t("Limiar", "Threshold"),
                        0.10..=3.50,
                        self.adv_apc.min(3.50),
                        0.05,
                        format!("{:.2} mm", self.adv_apc),
                        accent,
                        Message::AdvApcChanged,
                    ),
                    toggle_row(
                        l.t("Rapid Trigger no par", "Rapid Trigger on Pair"),
                        None,
                        self.adv_rt,
                        accent,
                        Message::AdvRtToggled,
                    ),
                ]
                .spacing(SP_SM);

                card(title, Some(desc.to_string()), Some(actions.into()), body.into())
            }
            AdvSubTab::HoldClick => card(
                "Hold / Click (Tap Dance / MT)",
                Some(
                    l.t(
                        "Executa um código de tecla no toque rápido (Tap) e outro ao manter pressionado (Hold).",
                        "Fires one keycode on quick tap and a different keycode when held past the timer.",
                    )
                    .to_string(),
                ),
                Some(
                    button(
                        row![
                            icon(Icon::Save, 13, contrast_on(accent)),
                            text(l.t("Salvar Slot MT", "Save Slot"))
                                .size(type_scale::CAPTION + 1)
                                .font(FONT_MEDIUM),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 13])
                    .style(primary_button_style(accent))
                    .on_press(Message::SaveTapDance)
                    .into(),
                ),
                row![
                    text(l.t("Keycode Tap (hex):", "Tap Keycode (hex):"))
                        .size(type_scale::BODY)
                        .color(TEXT_SECONDARY),
                    text_input("0x0004", &self.mt_tap)
                        .on_input(Message::MtTapChanged)
                        .width(Length::Fixed(96.0))
                        .style(input_style(accent)),
                    text(l.t("Keycode Hold (hex):", "Hold Keycode (hex):"))
                        .size(type_scale::BODY)
                        .color(TEXT_SECONDARY),
                    text_input("0x00E1", &self.mt_hold)
                        .on_input(Message::MtHoldChanged)
                        .width(Length::Fixed(96.0))
                        .style(input_style(accent)),
                    text("Timer (ms):").size(type_scale::BODY).color(TEXT_SECONDARY),
                    text_input("200", &self.mt_timer)
                        .on_input(Message::MtTimerChanged)
                        .width(Length::Fixed(76.0))
                        .style(input_style(accent)),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            ),
            AdvSubTab::Dks => {
                let mut rows_col = Column::new().spacing(SP_SM);
                for (i, act) in self.dks_draft.actions.iter().enumerate() {
                    let kc_str = format!("{:#06x}", act.keycode());
                    let st_str = format!("{:#06x}", act.status);
                    let kc_clone = kc_str.clone();
                    let st_clone = st_str.clone();
                    rows_col = rows_col.push(
                        row![
                            text(format!("{} {i}:", l.t("Ação", "Action")))
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM)
                                .color(TEXT_SECONDARY)
                                .width(Length::Fixed(70.0)),
                            text_input("keycode", &kc_str)
                                .on_input(move |s| Message::DksActionChanged(i, s, st_clone.clone()))
                                .width(Length::Fixed(110.0))
                                .style(input_style(accent)),
                            text(l.t("Máscara:", "Trigger Mask:"))
                                .size(type_scale::BODY)
                                .color(TEXT_SECONDARY),
                            text_input("mask", &st_str)
                                .on_input(move |s| Message::DksActionChanged(i, kc_clone.clone(), s))
                                .width(Length::Fixed(110.0))
                                .style(input_style(accent)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }

                card(
                    "Dynamic Key Stroke (DKS)",
                    Some(
                        l.t(
                            "Dispara até 4 ações diferentes em pontos distintos de descida e subida de uma tecla.",
                            "Triggers up to 4 distinct actions across press and release travel points.",
                        )
                        .to_string(),
                    ),
                    Some(
                        button(
                            row![
                                icon(Icon::Save, 13, contrast_on(accent)),
                                text(l.t("Salvar Slot DKS", "Save DKS Slot"))
                                    .size(type_scale::CAPTION + 1)
                                    .font(FONT_MEDIUM),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                        )
                        .padding([6, 13])
                        .style(primary_button_style(accent))
                        .on_press(Message::SaveDks)
                        .into(),
                    ),
                    rows_col.into(),
                )
            }
            AdvSubTab::Toggle => card(
                "Toggle Switch (TGL)",
                None,
                None,
                text(l.t(
                    "TGL não é suportado pelo firmware DuckBread (protocolo v9).",
                    "TGL is not supported by DuckBread firmware protocol v9.",
                ))
                .size(type_scale::BODY)
                .color(TEXT_MUTED)
                .into(),
            ),
        };

        column![sub_bar, slot_row, editor].spacing(SP_MD).into()
    }
}
