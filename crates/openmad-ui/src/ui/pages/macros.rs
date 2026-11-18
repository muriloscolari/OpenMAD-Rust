//! Onboard macros page (16 shared slots).

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, danger_button_style, elevated_card_style, input_style, pick_list_style,
    primary_button_style, secondary_button_style, tab_button_style, type_scale, FONT_MEDIUM,
    RED_DANGER, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::ui::components::{card, empty_state};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, pick_list, row, text, text_input, Column, Row, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::features::MacroStep;
use openmad_core::keycodes;

impl OpenMadApp {
    pub fn view_macros_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        let mut slots = Row::new().spacing(4);
        for i in 0..16 {
            slots = slots.push(
                button(text(format!("M{i}")).size(type_scale::CAPTION).font(FONT_MEDIUM))
                    .padding([6, 10])
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

        let add_row = row![
            pick_list(kinds, Some(self.macro_step_kind.clone()), Message::MacroStepKindChanged)
                .style(pick_list_style)
                .padding([6, 10]),
            text_input(
                l.t("Keycode (0x04) / ms / texto", "Keycode (0x04) / ms / text"),
                &self.macro_step_val,
            )
            .on_input(Message::MacroStepValChanged)
            .style(input_style(accent)),
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
        ]
        .spacing(SP_SM)
        .align_y(Alignment::Center);

        let steps_view: Element<'_, Message> = if self.macro_steps.is_empty() {
            empty_state(
                Icon::CircleDot,
                l.t("Macro vazia", "Empty macro slot"),
                l.t(
                    "Adicione passos de tecla, atraso ou texto acima para gravar neste slot.",
                    "Add key, delay, or text steps above to record into this slot.",
                ),
                None,
                None,
                accent,
            )
        } else {
            let mut steps_col = Column::new().spacing(6);
            for (idx, step) in self.macro_steps.iter().enumerate() {
                let desc = match step {
                    MacroStep::Tap { keycode } => format!("Tap  · {}", keycodes::label(*keycode)),
                    MacroStep::Down { keycode } => format!("Down · {}", keycodes::label(*keycode)),
                    MacroStep::Up { keycode } => format!("Up   · {}", keycodes::label(*keycode)),
                    MacroStep::Delay { delay_ms } => format!("Delay · {delay_ms} ms"),
                    MacroStep::Text { text } => format!("Text · \"{text}\""),
                };
                steps_col = steps_col.push(
                    container(
                        row![
                            text(format!("{idx:02}."))
                                .size(type_scale::CAPTION + 1)
                                .font(FONT_MEDIUM)
                                .color(TEXT_MUTED),
                            text(desc)
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM)
                                .color(TEXT_PRIMARY),
                            Space::with_width(Length::Fill),
                            button(icon(Icon::Trash, 13, RED_DANGER))
                                .padding([4, 8])
                                .style(danger_button_style)
                                .on_press(Message::RemoveMacroStep(idx)),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .padding([8, 12]),
                    )
                    .style(elevated_card_style),
                );
            }
            steps_col.into()
        };

        let header_actions = row![
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
        .spacing(8);

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
            column![slots, add_row, steps_view].spacing(SP_MD).into(),
        )
    }
}
