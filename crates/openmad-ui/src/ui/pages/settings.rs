//! Settings page with macOS-inspired grouped sections (General, Updates, LED Gain, About).

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, pick_list_style, primary_button_style, secondary_button_style, type_scale,
    FONT_MEDIUM, GREEN_OK, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, slider_row, toggle_row};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, pick_list, row, text, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::updater::APP_VERSION;

impl OpenMadApp {
    pub fn view_settings_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        let langs = vec!["Português (BR)".to_string(), "English".to_string()];
        let cur_lang = if l.is_pt() {
            "Português (BR)".to_string()
        } else {
            "English".to_string()
        };

        let general_card = card(
            l.t("Geral", "General"),
            None,
            None,
            column![
                row![
                    column![
                        text(l.t("Idioma da Interface", "Interface Language"))
                            .size(type_scale::BODY)
                            .font(FONT_MEDIUM)
                            .color(TEXT_PRIMARY),
                        text(l.t(
                            "Altera imediatamente todos os textos do aplicativo.",
                            "Immediately switches all UI labels.",
                        ))
                        .size(type_scale::CAPTION)
                        .color(TEXT_MUTED),
                    ]
                    .spacing(2),
                    Space::with_width(Length::Fill),
                    pick_list(langs, Some(cur_lang), Message::SelectLanguage)
                        .style(pick_list_style)
                        .padding([6, 10]),
                ]
                .align_y(Alignment::Center),
                toggle_row(
                    l.t("Iniciar junto com o sistema", "Start at login"),
                    Some(
                        l.t(
                            "Abre automaticamente minimizado na bandeja do sistema.",
                            "Launches automatically minimized to the system tray.",
                        )
                        .to_string(),
                    ),
                    self.settings.start_at_login,
                    accent,
                    Message::StartAtLoginToggled,
                ),
            ]
            .spacing(SP_MD)
            .into(),
        );

        let upd_text = match &self.update_info {
            None => format!("{} v{APP_VERSION}", l.t("Versão instalada:", "Installed version:")),
            Some(Ok(u)) if u.available => format!(
                "{}: v{} ({} v{})",
                l.t("Atualização disponível", "Update available"),
                u.latest,
                l.t("atual", "current"),
                u.current
            ),
            Some(Ok(u)) => format!("{} (v{})", l.t("Atualizado", "Up to date"), u.current),
            Some(Err(e)) => e.clone(),
        };
        let has_upd = matches!(&self.update_info, Some(Ok(u)) if u.available);

        let mut upd_actions = row![
            button(
                row![
                    icon(Icon::Refresh, 13, TEXT_PRIMARY),
                    text(l.t("Verificar Agora", "Check Now")).size(type_scale::CAPTION + 1),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([6, 12])
            .style(secondary_button_style)
            .on_press(Message::CheckUpdates),
        ]
        .spacing(8);

        if has_upd {
            upd_actions = upd_actions.push(
                button(
                    row![
                        icon(Icon::Download, 13, contrast_on(accent)),
                        text(l.t("Instalar Atualização", "Install Update"))
                            .size(type_scale::CAPTION + 1)
                            .font(FONT_MEDIUM),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([6, 12])
                .style(primary_button_style(accent))
                .on_press(Message::InstallUpdate),
            );
        }

        let updates_card = card(
            l.t("Atualizações", "Updates"),
            None,
            Some(badge(format!("v{APP_VERSION}"), if has_upd { accent } else { GREEN_OK })),
            column![
                toggle_row(
                    l.t("Verificar atualizações ao iniciar", "Check for updates on startup"),
                    None,
                    self.settings.check_updates,
                    accent,
                    Message::CheckUpdatesPrefToggled,
                ),
                row![
                    text(upd_text)
                        .size(type_scale::BODY)
                        .color(TEXT_SECONDARY),
                    Space::with_width(Length::Fill),
                    upd_actions,
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(SP_MD)
            .into(),
        );

        let gain_card = card(
            l.t("Calibração de Cor dos LEDs (Ganho RGB)", "LED Channel Color Calibration (Gain)"),
            Some(
                l.t(
                    "Equilibra o balanço de branco dos LEDs em tempo real (verde e azul costumam ser mais fortes que o vermelho).",
                    "Balances LED white point in real time (green/blue diodes are naturally brighter than red).",
                )
                .to_string(),
            ),
            None,
            column![
                slider_row(
                    l.t("Ganho Vermelho", "Red Gain"),
                    0.1..=1.0,
                    self.settings.led_gain[0],
                    0.01,
                    format!("{:.2}", self.settings.led_gain[0]),
                    Color::from_rgb8(255, 69, 58),
                    Message::GainRChanged,
                ),
                slider_row(
                    l.t("Ganho Verde", "Green Gain"),
                    0.1..=1.0,
                    self.settings.led_gain[1],
                    0.01,
                    format!("{:.2}", self.settings.led_gain[1]),
                    Color::from_rgb8(48, 209, 88),
                    Message::GainGChanged,
                ),
                slider_row(
                    l.t("Ganho Azul", "Blue Gain"),
                    0.1..=1.0,
                    self.settings.led_gain[2],
                    0.01,
                    format!("{:.2}", self.settings.led_gain[2]),
                    Color::from_rgb8(10, 132, 255),
                    Message::GainBChanged,
                ),
                row![
                    button(
                        text(l.t("Brilho Máximo Original (1.0, 1.0, 1.0)", "Original Max Brightness (1.0, 1.0, 1.0)"))
                            .size(type_scale::CAPTION + 1),
                    )
                    .padding([7, 13])
                    .style(primary_button_style(accent))
                    .on_press(Message::SetGainPreset([1.0, 1.0, 1.0])),
                    button(
                        text(l.t("Balanço Suave (1.0, 0.65, 0.75)", "Soft White Balance (1.0, 0.65, 0.75)"))
                            .size(type_scale::CAPTION + 1),
                    )
                    .padding([7, 13])
                    .style(secondary_button_style)
                    .on_press(Message::SetGainPreset([1.0, 0.65, 0.75])),
                ]
                .spacing(SP_SM),
            ]
            .spacing(SP_SM)
            .into(),
        );

        container(
            column![general_card, updates_card, gain_card]
                .spacing(SP_MD)
                .max_width(760.0),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    }
}
