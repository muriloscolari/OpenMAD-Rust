//! System & Flash page: hardware diagnostics, report + volatile write probe, Flash commit, JSON backup/restore, and factory reset.

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, danger_button_style, elevated_card_style, input_style, primary_button_style,
    secondary_button_style, type_scale, AMBER_WARN, FONT_MEDIUM, FONT_SEMIBOLD, GREEN_OK,
    RED_DANGER, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, toggle_row};
use crate::ui::icons::{icon, Icon};
use iced::widget::{button, column, container, row, text, text_input, Column, Space};
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

        let mut actions_row = row![
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
            .on_press(Message::RequestCommitToFlash),
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
        ]
        .spacing(SP_SM)
        .align_y(Alignment::Center);

        if !self.snap.board.verified && !self.snap.read_only {
            actions_row = actions_row.push(
                button(
                    row![
                        icon(Icon::AlertTriangle, 14, AMBER_WARN),
                        text(l.t("Re-bloquear Escrita", "Lock Read-Only")).size(type_scale::BODY),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .style(secondary_button_style)
                .on_press(Message::LockBoard),
            );
        }

        let info_card = card(
            l.t("Diagnóstico, Backup e Memória Flash", "Diagnostics, Backup & Flash"),
            Some(
                l.t(
                    "No uso diário os ajustes ficam na RAM do teclado. Use \"Salvar na Flash\" para gravar permanentemente ou exporte um backup JSON completo.",
                    "Daily tweaks apply to volatile RAM. Use Save to Flash to persist across power cycles, or export a full JSON backup.",
                )
                .to_string(),
            ),
            Some(badge(
                if self.snap.board.verified {
                    l.t("Placa Verificada", "Verified Board")
                } else {
                    l.t("Placa Não Verificada", "Unverified Board")
                },
                if self.snap.board.verified { GREEN_OK } else { AMBER_WARN },
            )),
            column![diag_grid, actions_row].spacing(SP_MD).into(),
        );

        let mut iface_list = Column::new().spacing(4);
        if self.snap.hid_interfaces.is_empty() {
            iface_list = iface_list.push(
                text(l.t(
                    "Nenhuma interface HID 0x373B detectada no momento.",
                    "No 0x373B HID interfaces currently detected.",
                ))
                .size(type_scale::CAPTION + 1)
                .color(TEXT_MUTED),
            );
        } else {
            for iface in &self.snap.hid_interfaces {
                iface_list = iface_list.push(
                    text(format!(
                        "PID {:#06x} · IF#{} · usage {:#06x}:{:#04x} · {}",
                        iface.product_id,
                        iface.interface_number,
                        iface.usage_page,
                        iface.usage,
                        if iface.product_string.is_empty() {
                            "—"
                        } else {
                            &iface.product_string
                        }
                    ))
                    .size(type_scale::CAPTION)
                    .color(TEXT_SECONDARY),
                );
            }
        }

        let diag_card = card(
            l.t("Relatório de Suporte de Placa", "Board Support & Diagnostic Report"),
            Some(
                l.t(
                    "Gera um arquivo JSON seguro (sem número de série ou perfis pessoais) com leituras do firmware para confirmar suporte a novos modelos.",
                    "Generates a privacy-safe JSON report (no serial number or profiles) with firmware readbacks to verify support for new models.",
                )
                .to_string(),
            ),
            Some(
                button(
                    row![
                        icon(Icon::ExternalLink, 14, contrast_on(accent)),
                        text(if self.diag_running {
                            l.t("Gerando Relatório…", "Generating Report…")
                        } else {
                            l.t("Gerar Relatório de Placa", "Generate Board Report")
                        })
                        .size(type_scale::BODY)
                        .font(FONT_MEDIUM),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([7, 14])
                .style(primary_button_style(accent))
                .on_press(Message::RunDiagnostic)
                .into(),
            ),
            column![
                toggle_row(
                    l.t("Incluir teste seguro de escrita (Write Probe em RAM)", "Include volatile write probe (RAM only)"),
                    Some(
                        l.t(
                            "Escreve 1 valor de atuação na tecla 0 em RAM (FlashOp::Normal), confere a leitura e restaura o valor original.",
                            "Writes one actuation value to key 0 in RAM (FlashOp::Normal), reads it back, and restores the original.",
                        )
                        .to_string(),
                    ),
                    self.diag_write_probe,
                    accent,
                    Message::DiagWriteProbeToggled,
                ),
                container(iface_list.padding(10))
                    .width(Length::Fill)
                    .style(elevated_card_style),
            ]
            .spacing(SP_SM)
            .into(),
        );

        let danger_card = card(
            l.t(
                "Factory Reset & Recuperação de EEPROM",
                "Factory Reset & EEPROM Recovery",
            ),
            Some(
                l.t(
                    "Digite RESET abaixo para limpar calibração e memória interna, ou restaure apenas o keymap padrão de fábrica.",
                    "Type RESET below to wipe onboard calibration and memory, or reset only the dynamic keymap to factory defaults.",
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
                Space::with_width(Length::Fill),
                button(
                    row![
                        icon(Icon::RotateCcw, 14, RED_DANGER),
                        text(l.t("Resetar Keymap (EEPROM)", "Reset Keymap (EEPROM)")).size(type_scale::BODY),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                )
                .padding([8, 14])
                .style(danger_button_style)
                .on_press(Message::RequestResetKeymap),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
        );

        column![info_card, diag_card, danger_card]
            .spacing(SP_MD)
            .into()
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
