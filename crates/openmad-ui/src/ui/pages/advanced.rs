//! Advanced keys page (Rappy Snappy, SOCD with 4 priority modes, Hold/Click visual picker, DKS 4x4 matrix).

use crate::app::{AdvPickTarget, AdvSubTab, Message, OpenMadApp};
use crate::theme::{
    contrast_on, danger_button_style, elevated_card_style, primary_button_style,
    tab_button_style, toggle_primary_style, type_scale, AMBER_WARN,
    FONT_MEDIUM, FONT_SEMIBOLD, GREEN_OK, RED_DANGER, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, keycode_picker, segmented, slider_row, toggle_row};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, row, text, Column, Row};
use iced::{Alignment, Color, Element, Length};
use openmad_core::keycodes;
use openmad_core::protocol::AdvancedKeyMode;

impl OpenMadApp {
    fn key_label_at(&self, r: u8, c: u8) -> String {
        if let Some(ref cfg) = self.snap.config {
            let cols = self.snap.board.cols.max(1);
            let dec = keycodes::decode_keymap(&cfg.keymap, self.layer, self.snap.board.rows, cols);
            if let Some(&kc) = dec.get(r as usize).and_then(|row| row.get(c as usize)) {
                return format!("{} ({r},{c})", keycodes::label(kc));
            }
        }
        format!("({r},{c})")
    }

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
            let occupied = self.snap.config.as_ref().is_some_and(|c| match self.adv_sub {
                AdvSubTab::RappySnappy => c
                    .advanced_keys
                    .get(i)
                    .is_some_and(|ak| ak.mode == AdvancedKeyMode::Rs),
                AdvSubTab::Socd => c
                    .advanced_keys
                    .get(i)
                    .is_some_and(|ak| matches!(
                        ak.mode,
                        AdvancedKeyMode::Socd
                            | AdvancedKeyMode::SocdKey1
                            | AdvancedKeyMode::SocdKey2
                            | AdvancedKeyMode::SocdBalance
                    )),
                AdvSubTab::HoldClick => c.tap_dance.get(i).is_some_and(|td| td.is_active()),
                AdvSubTab::Dks => c.dks.get(i).is_some_and(|d| !d.is_empty()),
                AdvSubTab::Toggle => false,
            });
            let slot_lbl = if occupied {
                format!("{i}•")
            } else {
                format!("{i}")
            };
            slot_row = slot_row.push(
                button(
                    text(slot_lbl)
                        .size(type_scale::CAPTION)
                        .font(if occupied { FONT_SEMIBOLD } else { FONT_MEDIUM }),
                )
                .padding([5, 8])
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
                            "Clique nos botões Tecla 1 / Tecla 2 e selecione no teclado acima, depois escolha o modo de prioridade.",
                            "Click Key 1 / Key 2 below, click keys on the keyboard above, and pick a priority mode.",
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
                            icon(Icon::Trash, 13, RED_DANGER),
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

                let k1_picking = self.adv_pick_target == Some(AdvPickTarget::Key1);
                let k2_picking = self.adv_pick_target == Some(AdvPickTarget::Key2);
                let k1_r = self.adv_k1_r.parse::<u8>().unwrap_or(0);
                let k1_c = self.adv_k1_c.parse::<u8>().unwrap_or(0);
                let k2_r = self.adv_k2_r.parse::<u8>().unwrap_or(0);
                let k2_c = self.adv_k2_c.parse::<u8>().unwrap_or(0);

                let pair_picker_row = row![
                    column![
                        text(l.t("Tecla 1", "Key 1"))
                            .size(type_scale::CAPTION)
                            .color(TEXT_MUTED),
                        button(
                            row![
                                icon(Icon::Keyboard, 13, if k1_picking { contrast_on(accent) } else { accent }),
                                text(if k1_picking {
                                    l.t("Clique no teclado acima…", "Click key above…").to_string()
                                } else {
                                    self.key_label_at(k1_r, k1_c)
                                })
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM),
                            ]
                            .spacing(7)
                            .align_y(Alignment::Center),
                        )
                        .padding([8, 14])
                        .style(toggle_primary_style(k1_picking, accent))
                        .on_press(Message::AdvPickTargetSelected(if k1_picking {
                            None
                        } else {
                            Some(AdvPickTarget::Key1)
                        })),
                    ]
                    .spacing(4),
                    column![
                        text(l.t("Tecla 2", "Key 2"))
                            .size(type_scale::CAPTION)
                            .color(TEXT_MUTED),
                        button(
                            row![
                                icon(Icon::Keyboard, 13, if k2_picking { contrast_on(accent) } else { GREEN_OK }),
                                text(if k2_picking {
                                    l.t("Clique no teclado acima…", "Click key above…").to_string()
                                } else {
                                    self.key_label_at(k2_r, k2_c)
                                })
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM),
                            ]
                            .spacing(7)
                            .align_y(Alignment::Center),
                        )
                        .padding([8, 14])
                        .style(toggle_primary_style(k2_picking, accent))
                        .on_press(Message::AdvPickTargetSelected(if k2_picking {
                            None
                        } else {
                            Some(AdvPickTarget::Key2)
                        })),
                    ]
                    .spacing(4),
                ]
                .spacing(16)
                .align_y(Alignment::Center);

                let mut body = column![pair_picker_row].spacing(SP_SM);

                if self.adv_sub == AdvSubTab::Socd {
                    let socd_modes = [
                        (
                            AdvancedKeyMode::Socd,
                            l.t("Última Tecla (Last Input)", "Last Input Priority"),
                        ),
                        (
                            AdvancedKeyMode::SocdKey1,
                            l.t("Prioridade Tecla 1", "Key 1 Priority"),
                        ),
                        (
                            AdvancedKeyMode::SocdKey2,
                            l.t("Prioridade Tecla 2", "Key 2 Priority"),
                        ),
                        (
                            AdvancedKeyMode::SocdBalance,
                            l.t("Neutro (Anula Ambas)", "Neutral (Cancel Out)"),
                        ),
                    ];
                    let active_socd = match self.adv_mode {
                        AdvancedKeyMode::Socd
                        | AdvancedKeyMode::SocdKey1
                        | AdvancedKeyMode::SocdKey2
                        | AdvancedKeyMode::SocdBalance => self.adv_mode,
                        _ => AdvancedKeyMode::Socd,
                    };
                    body = body.push(
                        row![
                            text(l.t("Modo SOCD:", "SOCD Mode:"))
                                .size(type_scale::BODY)
                                .color(TEXT_SECONDARY),
                            segmented(&socd_modes, active_socd, accent, Message::AdvModeChanged),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }

                body = body
                    .push(slider_row(
                        l.t("Limiar", "Threshold"),
                        0.10..=3.50,
                        self.adv_apc.min(3.50),
                        0.05,
                        format!("{:.2} mm", self.adv_apc),
                        accent,
                        Message::AdvApcChanged,
                    ))
                    .push(toggle_row(
                        l.t("Rapid Trigger no par", "Rapid Trigger on Pair"),
                        None,
                        self.adv_rt,
                        accent,
                        Message::AdvRtToggled,
                    ));

                card(title, Some(desc.to_string()), Some(actions.into()), body.into())
            }
            AdvSubTab::HoldClick => {
                let tap_kc = crate::app::parse_u16_hex_or_dec(&self.mt_tap).unwrap_or(0);
                let hold_kc = crate::app::parse_u16_hex_or_dec(&self.mt_hold).unwrap_or(0);
                let timer_ms = self.mt_timer.parse::<f32>().unwrap_or(200.0).clamp(10.0, 1000.0);
                let picking_tap = self.adv_pick_target == Some(AdvPickTarget::MtTap);
                let picking_hold = self.adv_pick_target == Some(AdvPickTarget::MtHold);

                let actions = row![
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
                    .on_press(Message::SaveTapDance),
                    button(
                        row![
                            icon(Icon::Trash, 13, RED_DANGER),
                            text(l.t("Limpar Slot", "Clear Slot")).size(type_scale::CAPTION + 1),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 12])
                    .style(danger_button_style)
                    .on_press(Message::ClearTapDance),
                ]
                .spacing(8);

                let mut body = column![
                    row![
                        column![
                            text(l.t("Toque Rápido (Tap)", "Quick Tap"))
                                .size(type_scale::CAPTION)
                                .color(TEXT_MUTED),
                            button(
                                text(format!("{} ({:#06x})", keycodes::label(tap_kc), tap_kc))
                                    .size(type_scale::BODY)
                                    .font(FONT_MEDIUM),
                            )
                            .padding([8, 16])
                            .style(toggle_primary_style(picking_tap, accent))
                            .on_press(Message::AdvPickTargetSelected(if picking_tap {
                                None
                            } else {
                                Some(AdvPickTarget::MtTap)
                            })),
                        ]
                        .spacing(4),
                        column![
                            text(l.t("Segurar (Hold)", "Hold"))
                                .size(type_scale::CAPTION)
                                .color(TEXT_MUTED),
                            button(
                                text(format!("{} ({:#06x})", keycodes::label(hold_kc), hold_kc))
                                    .size(type_scale::BODY)
                                    .font(FONT_MEDIUM),
                            )
                            .padding([8, 16])
                            .style(toggle_primary_style(picking_hold, accent))
                            .on_press(Message::AdvPickTargetSelected(if picking_hold {
                                None
                            } else {
                                Some(AdvPickTarget::MtHold)
                            })),
                        ]
                        .spacing(4),
                    ]
                    .spacing(16),
                    slider_row(
                        l.t("Tempo Hold (ms)", "Hold Timer (ms)"),
                        10.0..=1000.0,
                        timer_ms,
                        10.0,
                        format!("{:.0} ms", timer_ms),
                        accent,
                        |v| Message::MtTimerChanged(format!("{}", v as u16)),
                    ),
                ]
                .spacing(SP_MD);

                if picking_tap || picking_hold {
                    body = body.push(
                        container(
                            column![
                                badge(
                                    if picking_tap {
                                        l.t("Escolha o keycode para Tap:", "Pick Tap keycode:")
                                    } else {
                                        l.t("Escolha o keycode para Hold:", "Pick Hold keycode:")
                                    },
                                    accent,
                                ),
                                keycode_picker(
                                    l,
                                    self.pick_tab,
                                    &self.keycode_search,
                                    accent,
                                    Message::SelectKeyPickerTab,
                                    Message::KeycodeSearchChanged,
                                    Message::KeycodePicked,
                                ),
                            ]
                            .spacing(SP_SM)
                            .padding(10),
                        )
                        .style(elevated_card_style),
                    );
                }

                card(
                    "Hold / Click (Tap Dance / MT)",
                    Some(
                        l.t(
                            "Executa uma tecla no toque rápido e outra ao segurar. Clique nos botões acima para escolher visualmente.",
                            "Fires one keycode on quick tap and another when held past the timer. Click either button above to pick visually.",
                        )
                        .to_string(),
                    ),
                    Some(actions.into()),
                    body.into(),
                )
            }
            AdvSubTab::Dks => {
                let zones = [
                    ("↓ 1.00mm", l.t("Descida rasa", "Shallow down")),
                    ("↓ 3.00mm", l.t("Fundo", "Bottom out")),
                    ("↑ 3.00mm", l.t("Retorno fundo", "Early release")),
                    ("↑ 1.00mm", l.t("Retorno topo", "Top release")),
                ];

                let mut header_row = Row::new().spacing(8).align_y(Alignment::Center);
                header_row = header_row.push(
                    text(l.t("Ação / Keycode", "Action / Keycode"))
                        .size(type_scale::CAPTION)
                        .font(FONT_SEMIBOLD)
                        .color(TEXT_MUTED)
                        .width(Length::Fixed(150.0)),
                );
                for (z_mm, z_tip) in zones {
                    header_row = header_row.push(
                        column![
                            text(z_mm)
                                .size(type_scale::CAPTION)
                                .font(FONT_SEMIBOLD)
                                .color(TEXT_PRIMARY),
                            text(z_tip)
                                .size(type_scale::CAPTION - 1)
                                .color(TEXT_MUTED),
                        ]
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                    );
                }

                let mut matrix_col = Column::new().spacing(6);
                matrix_col = matrix_col.push(header_row);

                let mut active_dks_pick: Option<usize> = None;
                for (i, act) in self.dks_draft.actions.iter().enumerate() {
                    let kc = act.keycode();
                    let picking = self.adv_pick_target == Some(AdvPickTarget::Dks(i));
                    if picking {
                        active_dks_pick = Some(i);
                    }
                    let mut r_row = Row::new().spacing(8).align_y(Alignment::Center);
                    r_row = r_row.push(
                        button(
                            text(if kc == 0 {
                                format!("#{i} · —")
                            } else {
                                format!("#{i} · {}", keycodes::label(kc))
                            })
                            .size(type_scale::CAPTION + 1)
                            .font(FONT_MEDIUM),
                        )
                        .width(Length::Fixed(150.0))
                        .padding([7, 10])
                        .style(toggle_primary_style(picking, accent))
                        .on_press(Message::AdvPickTargetSelected(if picking {
                            None
                        } else {
                            Some(AdvPickTarget::Dks(i))
                        })),
                    );
                    for col_bit in 0..4usize {
                        let on = ((act.status >> col_bit) & 1) != 0;
                        r_row = r_row.push(
                            button(
                                container(
                                    text(if on { "●" } else { "+" })
                                        .size(type_scale::BODY)
                                        .color(if on { contrast_on(accent) } else { TEXT_MUTED }),
                                )
                                .center_x(Length::Fill),
                            )
                            .width(Length::Fill)
                            .padding([7, 0])
                            .style(toggle_primary_style(on, accent))
                            .on_press(Message::DksToggleBit(i, col_bit)),
                        );
                    }
                    matrix_col = matrix_col.push(r_row);
                }

                let mut body = column![matrix_col].spacing(SP_MD);
                if let Some(row_idx) = active_dks_pick {
                    body = body.push(
                        container(
                            column![
                                badge(
                                    format!(
                                        "{} #{row_idx}",
                                        l.t("Escolha a tecla para Ação", "Pick keycode for Action")
                                    ),
                                    accent,
                                ),
                                keycode_picker(
                                    l,
                                    self.pick_tab,
                                    &self.keycode_search,
                                    accent,
                                    Message::SelectKeyPickerTab,
                                    Message::KeycodeSearchChanged,
                                    Message::KeycodePicked,
                                ),
                            ]
                            .spacing(SP_SM)
                            .padding(10),
                        )
                        .style(elevated_card_style),
                    );
                }

                let actions = row![
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
                    .on_press(Message::SaveDks),
                    button(
                        row![
                            icon(Icon::Trash, 13, RED_DANGER),
                            text(l.t("Limpar Slot", "Clear Slot")).size(type_scale::CAPTION + 1),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 12])
                    .style(danger_button_style)
                    .on_press(Message::ClearDks),
                ]
                .spacing(8);

                card(
                    "Dynamic Key Stroke (DKS)",
                    Some(
                        l.t(
                            "Dispara até 4 ações em pontos distintos do curso. Clique no slot à esquerda para escolher a tecla e nos quadrados para marcar os pontos.",
                            "Triggers up to 4 actions across key travel points. Click a left slot to pick its keycode and toggle trigger cells across the 4 travel points.",
                        )
                        .to_string(),
                    ),
                    Some(actions.into()),
                    body.into(),
                )
            }
            AdvSubTab::Toggle => card(
                "Toggle Switch (TGL)",
                None,
                Some(badge(l.t("Não suportado no firmware", "Unsupported in firmware"), AMBER_WARN)),
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
