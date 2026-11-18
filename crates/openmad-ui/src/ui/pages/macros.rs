//! Onboard macros page (16 shared slots, live recorder, visual key picker, and buffer meter).

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, danger_button_style, elevated_card_style, input_style, pick_list_style,
    primary_button_style, secondary_button_style, tab_button_style, type_scale, AMBER_WARN,
    FONT_MEDIUM, GREEN_OK, RED_DANGER, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::ui::components::{badge, card, empty_state, keycode_picker};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, pick_list, row, text, text_input, Column, Row, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::features::{encoded_macro_len, MacroStep};
use openmad_core::keycodes;

impl OpenMadApp {
    pub fn view_macros_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        let mut slots = Row::new().spacing(4);
        let mut total_used = 0usize;
        let mut buf_cap = 0usize;
        if let Some(ref cfg) = self.snap.config {
            buf_cap = cfg.macro_buffer_size;
            for (idx, m) in cfg.macros.iter().enumerate() {
                let steps = if idx == self.macro_slot {
                    &self.macro_steps
                } else {
                    m
                };
                total_used += encoded_macro_len(steps) + 1;
            }
        } else {
            total_used = encoded_macro_len(&self.macro_steps) + 1;
        }

        for i in 0..16 {
            let has_steps = if i == self.macro_slot {
                !self.macro_steps.is_empty()
            } else {
                self.snap
                    .config
                    .as_ref()
                    .and_then(|c| c.macros.get(i))
                    .is_some_and(|m| !m.is_empty())
            };
            let lbl = if has_steps {
                format!("M{i}•")
            } else {
                format!("M{i}")
            };
            slots = slots.push(
                button(
                    text(lbl)
                        .size(type_scale::CAPTION)
                        .font(if has_steps { crate::theme::FONT_SEMIBOLD } else { FONT_MEDIUM }),
                )
                .padding([6, 9])
                .style(tab_button_style(self.macro_slot == i, accent))
                .on_press(Message::SelectMacroSlot(i)),
            );
        }

        let kinds = vec![
            "tap".to_string(),
            "down".to_string(),
            "up".to_string(),
            "delay".to_string(),
            "text".to_string(),
        ];

        let mut add_row = row![
            pick_list(kinds, Some(self.macro_step_kind.clone()), Message::MacroStepKindChanged)
                .style(pick_list_style)
                .padding([6, 10]),
            text_input(
                l.t("Keycode (0x04) / ms / texto", "Keycode (0x04) / ms / text"),
                &self.macro_step_val,
            )
            .on_input(Message::MacroStepValChanged)
            .on_submit(Message::AddMacroStep)
            .style(input_style(accent)),
        ]
        .spacing(SP_SM)
        .align_y(Alignment::Center);

        if matches!(self.macro_step_kind.as_str(), "tap" | "down" | "up") {
            add_row = add_row.push(
                button(
                    row![
                        icon(Icon::Keyboard, 13, TEXT_PRIMARY),
                        text(l.t("Escolher Tecla", "Pick Key")).size(type_scale::CAPTION + 1),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([7, 11])
                .style(secondary_button_style)
                .on_press(Message::ToggleMacroPicker),
            );
        }

        add_row = add_row.push(
            button(
                row![
                    icon(Icon::Plus, 13, TEXT_PRIMARY),
                    text(l.t("Adicionar Passo", "Add Step")).size(type_scale::CAPTION + 1),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([7, 13])
            .style(secondary_button_style)
            .on_press(Message::AddMacroStep),
        );

        let steps_view: Element<'_, Message> = if self.macro_steps.is_empty() {
            empty_state(
                Icon::CircleDot,
                l.t("Macro vazia", "Empty macro slot"),
                l.t(
                    "Clique em \"Gravar ao Vivo\" e digite normalmente, ou adicione passos acima.",
                    "Click Live Record and type normally, or add manual steps above.",
                ),
                None,
                None,
                accent,
            )
        } else {
            let len = self.macro_steps.len();
            let mut steps_col = Column::new().spacing(6);
            for (idx, step) in self.macro_steps.iter().enumerate() {
                let desc = match step {
                    MacroStep::Tap { keycode } => format!("Tap   · {}", keycodes::label(*keycode)),
                    MacroStep::Down { keycode } => format!("Down  · {}", keycodes::label(*keycode)),
                    MacroStep::Up { keycode } => format!("Up    · {}", keycodes::label(*keycode)),
                    MacroStep::Delay { delay_ms } => format!("Delay · {delay_ms} ms"),
                    MacroStep::Text { text } => format!("Text  · \"{text}\""),
                };
                let mut row_items = row![
                    text(format!("{idx:02}."))
                        .size(type_scale::CAPTION + 1)
                        .font(FONT_MEDIUM)
                        .color(TEXT_MUTED),
                    text(desc)
                        .size(type_scale::BODY)
                        .font(FONT_MEDIUM)
                        .color(TEXT_PRIMARY),
                    Space::with_width(Length::Fill),
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                if idx > 0 {
                    row_items = row_items.push(
                        button(icon(Icon::ChevronUp, 13, TEXT_PRIMARY))
                            .padding([4, 7])
                            .style(secondary_button_style)
                            .on_press(Message::MoveMacroStepUp(idx)),
                    );
                }
                if idx + 1 < len {
                    row_items = row_items.push(
                        button(icon(Icon::ChevronDown, 13, TEXT_PRIMARY))
                            .padding([4, 7])
                            .style(secondary_button_style)
                            .on_press(Message::MoveMacroStepDown(idx)),
                    );
                }
                row_items = row_items.push(
                    button(icon(Icon::Trash, 13, RED_DANGER))
                        .padding([4, 8])
                        .style(danger_button_style)
                        .on_press(Message::RemoveMacroStep(idx)),
                );

                steps_col = steps_col.push(
                    container(row_items.padding([8, 12]))
                        .style(elevated_card_style),
                );
            }
            steps_col.into()
        };

        let mem_badge = if buf_cap > 0 {
            let over = total_used > buf_cap;
            badge(
                format!(
                    "{}: {total_used}/{buf_cap} B",
                    l.t("Memória", "Memory")
                ),
                if over {
                    RED_DANGER
                } else if total_used * 100 / buf_cap.max(1) > 80 {
                    AMBER_WARN
                } else {
                    GREEN_OK
                },
            )
        } else {
            badge(format!("{}: {total_used} B", l.t("Uso", "Used")), accent)
        };

        let header_actions = row![
            mem_badge,
            button(
                row![
                    icon(
                        Icon::CircleDot,
                        13,
                        if self.macro_recording {
                            RED_DANGER
                        } else {
                            TEXT_PRIMARY
                        },
                    ),
                    text(if self.macro_recording {
                        l.t("Parar Gravação", "Stop Recording")
                    } else {
                        l.t("Gravar ao Vivo", "Live Record")
                    })
                    .size(type_scale::CAPTION + 1)
                    .font(FONT_MEDIUM),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(if self.macro_recording {
                danger_button_style
            } else {
                secondary_button_style
            })
            .on_press(Message::ToggleMacroRecording),
            button(
                row![
                    icon(Icon::Save, 13, contrast_on(accent)),
                    text(l.t("Salvar Slot", "Save Slot"))
                        .size(type_scale::CAPTION + 1)
                        .font(FONT_MEDIUM),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 13])
            .style(primary_button_style(accent))
            .on_press(Message::SaveMacroSlot),
            button(
                row![
                    icon(Icon::Trash, 13, RED_DANGER),
                    text(l.t("Limpar", "Clear")).size(type_scale::CAPTION + 1),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(danger_button_style)
            .on_press(Message::ClearMacroSlot),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut body = column![slots, add_row].spacing(SP_MD);
        if self.macro_recording {
            body = body.push(badge(
                l.t(
                    "Gravando teclas e atrasos ao vivo… digite no teclado e clique em Parar Gravação.",
                    "Recording keystrokes & delays live… type on your keyboard, then click Stop Recording.",
                ),
                RED_DANGER,
            ));
        }
        if self.macro_picker_open && matches!(self.macro_step_kind.as_str(), "tap" | "down" | "up")
        {
            body = body.push(
                container(
                    column![keycode_picker(
                        l,
                        self.pick_tab,
                        &self.keycode_search,
                        accent,
                        Message::SelectKeyPickerTab,
                        Message::KeycodeSearchChanged,
                        Message::KeycodePicked,
                    )]
                    .padding(10),
                )
                .style(elevated_card_style),
            );
        }
        body = body.push(steps_view);

        card(
            l.t("Macros Onboard (16 Slots)", "Onboard Macros (16 Slots)"),
            Some(
                l.t(
                    "Macros ficam armazenadas na memória interna do teclado e são compartilhadas por todos os perfis.",
                    "Macros live in the keyboard's onboard memory and are shared across every profile.",
                )
                .to_string(),
            ),
            Some(header_actions.into()),
            body.into(),
        )
    }
}
