//! System & Flash page: hardware diagnostics, Flash commit, JSON backup/restore, firmware link, and factory reset.

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, danger_button_style, elevated_card_style, input_style, primary_button_style,
    secondary_button_style, type_scale, FONT_MEDIUM, FONT_SEMIBOLD, RED_DANGER, SP_MD, SP_SM,
    TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::components::card;
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Color, Element, Length};

impl OpenMadApp {
    pub fn view_system_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        let diag_grid = container(
            row![
                self.stat_cell(l.t("Placa", "Board"), self.snap.board.name.clone()),
                self.stat_cell(
                    "Product ID",
                    format!("{:#06x}", self.snap.board.product_id),
                ),
                self.stat_cell(
                    l.t("Matriz", "Geometry"),
                    format!("{} × {}", self.snap.board.rows, self.snap.board.cols),
                ),
                self.stat_cell(
                    l.t("Protocolo", "Protocol"),
                    self.snap
                        .protocol_version
                        .map(|v| format!("v{v}"))
                        .unwrap_or_else(|| "—".into()),
                ),
                self.stat_cell(
                    "Firmware",
                    self.snap
                        .firmware_version
                        .map(|v| format!("v{v}"))
                        .unwrap_or_else(|| "—".into()),
                ),
            ]
            .spacing(SP_MD)
            .padding(12),
        )
        .width(Length::Fill)
        .style(elevated_card_style);

        let actions_row = row![
            button(
                row![
                    icon(Icon::HardDrive, 14, contrast_on(accent)),
                    text(l.t("Salvar na Flash", "Save to Flash"))
                        .size(type_scale::BODY)
                        .font(FONT_MEDIUM),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .style(primary_button_style(accent))
            .on_press(Message::CommitToFlash),
            button(
                row![
                    icon(Icon::Download, 14, TEXT_PRIMARY),
                    text(l.t("Exportar Backup", "Export Backup")).size(type_scale::BODY),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .style(secondary_button_style)
            .on_press(Message::ExportBackup),
            button(
                row![
                    icon(Icon::Upload, 14, TEXT_PRIMARY),
                    text(l.t("Restaurar Backup", "Restore Backup")).size(type_scale::BODY),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 14])
            .style(secondary_button_style)
            .on_press(Message::RestoreBackup),
            Space::with_width(Length::Fill),
            button(
                row![
                    icon(Icon::ExternalLink, 14, TEXT_SECONDARY),
                    text(l.t("Firmware Web", "Web Firmware")).size(type_scale::BODY),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(secondary_button_style)
            .on_press(Message::OpenFirmwareUrl),
            button(
                row![
                    icon(Icon::ExternalLink, 14, TEXT_SECONDARY),
                    text(l.t("Reportar Placa", "Report Board")).size(type_scale::BODY),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(secondary_button_style)
            .on_press(Message::OpenGithubIssue),
        ]
        .spacing(SP_SM)
        .align_y(Alignment::Center);

        let info_card = card(
            l.t("Diagnóstico, Backup e Memória Flash", "Diagnostics, Backup & Flash"),
            Some(
                l.t(
                    "No uso diário os ajustes ficam na RAM do teclado. Use \"Salvar na Flash\" para gravar permanentemente ou exporte um backup JSON completo.",
                    "Daily tweaks apply to volatile RAM. Use Save to Flash to persist across power cycles, or export a full JSON backup.",
                )
                .to_string(),
            ),
            None,
            column![diag_grid, actions_row].spacing(SP_MD).into(),
        );

        let danger_card = card(
            l.t(
                "Factory Reset (Memória Interna do Teclado)",
                "Factory Reset (Onboard Memory)",
            ),
            Some(
                l.t(
                    "Digite RESET abaixo para confirmar a limpeza da calibração e perfil salvo no teclado.",
                    "Type RESET below to confirm wiping onboard calibration and profile memory.",
                )
                .to_string(),
            ),
            None,
            row![
                text_input(l.t("Digite RESET", "Type RESET"), &self.factory_confirm)
                    .on_input(Message::FactoryResetInputChanged)
                    .width(Length::Fixed(180.0))
                    .style(input_style(RED_DANGER)),
                button(
                    row![
                        icon(Icon::AlertTriangle, 14, RED_DANGER),
                        text("Factory Reset").size(type_scale::BODY).font(FONT_MEDIUM),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([8, 14])
                .style(danger_button_style)
                .on_press(Message::ConfirmFactoryReset),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
        );

        column![info_card, danger_card].spacing(SP_MD).into()
    }

    fn stat_cell<'a>(&self, label: &'a str, val: impl Into<String>) -> Element<'a, Message> {
        column![
            text(label)
                .size(type_scale::CAPTION)
                .color(TEXT_MUTED),
            text(val.into())
                .size(type_scale::BODY)
                .font(FONT_SEMIBOLD)
                .color(TEXT_PRIMARY),
        ]
        .spacing(2)
        .width(Length::Fill)
        .into()
    }
}
