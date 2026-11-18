//! Performance page: live travel gauge, presets, actuation, Rapid Trigger, deadband, calibration & hardware flags.

use crate::app::{Message, OpenMadApp, PerfSubTab};
use crate::theme::{
    contrast_on, primary_button_style, secondary_button_style, type_scale, AMBER_WARN, FONT_MEDIUM,
    FONT_SEMIBOLD, GREEN_OK, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, segmented, slider_row, toggle_row};
use crate::ui::icons::{icon, Icon};
use crate::widgets::gauge::TravelGaugeBar;
use crate::widgets::travel_trace::TravelTraceCanvas;
use iced::widget::{button, column, row, text, toggler, Space};
use iced::{Alignment, Color, Element, Length};
use openmad_core::keycodes;

impl OpenMadApp {
    pub fn view_performance_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let sub_items = [
            (
                PerfSubTab::Travel,
                l.t("Curso & Rapid Trigger", "Travel & Rapid Trigger"),
            ),
            (PerfSubTab::Calibration, l.t("Calibração", "Calibration")),
            (
                PerfSubTab::Performance,
                l.t("Modos & Recursos", "Features & Modes"),
            ),
        ];
        let sub_bar = segmented(&sub_items, self.perf_sub, accent, Message::SelectPerfSubTab);

        let body: Element<'_, Message> = match self.perf_sub {
            PerfSubTab::Travel => {
                let deepest = self
                    .snap
                    .trip_mm
                    .iter()
                    .enumerate()
                    .filter(|(i, &mm)| mm > 0.04 && self.snap.populated.get(*i).copied().unwrap_or(true))
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(i, _)| i);
                let focus_idx = deepest
                    .or_else(|| self.selected_keys.iter().next().copied())
                    .unwrap_or(0);

                let live_mm = self.snap.trip_mm.get(focus_idx).copied().unwrap_or(0.0);
                let peak_mm = self.snap.peak_mm.get(focus_idx).copied().unwrap_or(0.0);
                let focus_name = self
                    .snap
                    .config
                    .as_ref()
                    .and_then(|c| {
                        let cols = self.snap.board.cols;
                        let r = focus_idx / cols;
                        let c_col = focus_idx % cols;
                        let decoded = keycodes::decode_keymap(
                            &c.keymap,
                            self.layer,
                            self.snap.board.rows,
                            cols,
                        );
                        decoded
                            .get(r)
                            .and_then(|row| row.get(c_col))
                            .map(|&kc| keycodes::label(kc))
                    })
                    .unwrap_or_else(|| format!("#{focus_idx}"));

                let gauge_header_actions = row![
                    button(
                        row![
                            icon(Icon::RotateCcw, 13, TEXT_SECONDARY),
                            text(l.t("Zerar Picos", "Reset Peaks")).size(type_scale::CAPTION + 1),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 11])
                    .style(secondary_button_style)
                    .on_press(Message::ResetPeaks),
                ];

                let gauge_body = column![
                    row![
                        column![
                            text(format!(
                                "{} · {}",
                                l.t("TECLA EM FOCO", "ACTIVE KEY"),
                                focus_name
                            ))
                            .size(type_scale::CAPTION)
                            .font(FONT_MEDIUM)
                            .color(TEXT_MUTED),
                            text(format!("{live_mm:.2} mm"))
                                .size(type_scale::DISPLAY)
                                .font(FONT_SEMIBOLD)
                                .color(if live_mm >= self.actuation_mm {
                                    GREEN_OK
                                } else {
                                    TEXT_PRIMARY
                                }),
                        ]
                        .spacing(2),
                        Space::with_width(Length::Fixed(24.0)),
                        column![
                            text(l.t("PICO REGISTRADO", "RECORDED PEAK"))
                                .size(type_scale::CAPTION)
                                .font(FONT_MEDIUM)
                                .color(TEXT_MUTED),
                            text(format!("{peak_mm:.2} mm"))
                                .size(20)
                                .font(FONT_SEMIBOLD)
                                .color(accent),
                        ]
                        .spacing(4),
                        Space::with_width(Length::Fill),
                        row![
                            button(text(l.t("Digitação (2.0)", "Typing (2.0)")).size(type_scale::CAPTION))
                                .padding([5, 10])
                                .style(secondary_button_style)
                                .on_press(Message::ApplyPerfPreset {
                                    actuation: 2.0,
                                    rt: false,
                                    press: 0.30,
                                    release: 0.30,
                                }),
                            button(text("Gaming (1.2)").size(type_scale::CAPTION))
                                .padding([5, 10])
                                .style(secondary_button_style)
                                .on_press(Message::ApplyPerfPreset {
                                    actuation: 1.2,
                                    rt: true,
                                    press: 0.30,
                                    release: 0.30,
                                }),
                            button(text(l.t("Competitivo (0.4)", "Competitive (0.4)")).size(type_scale::CAPTION))
                                .padding([5, 10])
                                .style(secondary_button_style)
                                .on_press(Message::ApplyPerfPreset {
                                    actuation: 0.4,
                                    rt: true,
                                    press: 0.15,
                                    release: 0.15,
                                }),
                        ]
                        .spacing(6),
                    ]
                    .align_y(Alignment::Center),
                    TravelGaugeBar {
                        live_mm,
                        peak_mm,
                        actuation_mm: self.actuation_mm,
                        dead_top_mm: self.dead_top_mm,
                        dead_bot_mm: self.dead_bot_mm,
                        accent,
                    }
                    .view(),
                    row![
                        text("0.00 mm").size(type_scale::CAPTION).color(TEXT_MUTED),
                        Space::with_width(Length::Fill),
                        text(format!(
                            "{}: {:.2} mm",
                            l.t("Ponto de atuação", "Actuation"),
                            self.actuation_mm
                        ))
                        .size(type_scale::CAPTION)
                        .font(FONT_MEDIUM)
                        .color(TEXT_SECONDARY),
                        Space::with_width(Length::Fill),
                        text("3.50 mm").size(type_scale::CAPTION).color(TEXT_MUTED),
                    ],
                    TravelTraceCanvas {
                        samples: &self.snap.focus_trace,
                        actuation_mm: self.actuation_mm,
                        dead_top_mm: self.dead_top_mm,
                        dead_bot_mm: self.dead_bot_mm,
                        accent,
                    }
                    .view(),
                    row![
                        badge("≤0.80mm", Color::from_rgb(0.18, 0.68, 0.98)),
                        badge("≤1.60mm", Color::from_rgb(0.24, 0.80, 0.48)),
                        badge("≤2.30mm", Color::from_rgb(0.55, 0.58, 0.66)),
                        badge(">2.30mm", AMBER_WARN),
                        badge("● RT", Color::from_rgb(0.392, 0.824, 1.0)),
                        Space::with_width(Length::Fill),
                        text(l.t(
                            "Osciloscópio ao vivo da tecla em foco",
                            "Live focused-key oscilloscope",
                        ))
                        .size(type_scale::CAPTION)
                        .color(TEXT_MUTED),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                ]
                .spacing(SP_SM);

                let gauge_card = card(
                    l.t("Medidor de Curso ao Vivo", "Live Travel Gauge"),
                    None,
                    Some(gauge_header_actions.into()),
                    gauge_body.into(),
                );

                let act_card = card(
                    l.t("Ponto de Atuação", "Actuation Point"),
                    Some(
                        l.t(
                            "Distância que a tecla precisa descer para registrar o clique (0.10 mm = ultra sensível, 2.00 mm = padrão).",
                            "Travel distance required to register a keystroke (0.10 mm = ultra-sensitive, 2.00 mm = standard).",
                        )
                        .to_string(),
                    ),
                    Some(
                        button(text(l.t("Aplicar na Seleção", "Apply to Selection")).size(type_scale::CAPTION + 1))
                            .padding([6, 12])
                            .style(secondary_button_style)
                            .on_press(Message::ApplyActuation)
                            .into(),
                    ),
                    slider_row(
                        l.t("Atuação", "Actuation"),
                        0.10..=3.50,
                        self.actuation_mm.min(3.50),
                        0.05,
                        format!("{:.2} mm", self.actuation_mm),
                        accent,
                        Message::ActuationChanged,
                    ),
                );

                let rt_actions = row![
                    toggler(self.rt_enabled)
                        .style(crate::theme::toggler_style(accent))
                        .on_toggle(Message::RtEnabledToggled),
                    Space::with_width(Length::Fixed(8.0)),
                    button(text(l.t("Aplicar na Seleção", "Apply to Selection")).size(type_scale::CAPTION + 1))
                        .padding([6, 12])
                        .style(secondary_button_style)
                        .on_press(Message::ApplyRapidTrigger),
                ]
                .align_y(Alignment::Center);

                let rt_body = if self.rt_enabled {
                    column![
                        slider_row(
                            l.t("Sensibilidade Pressionar", "Press Sensitivity"),
                            0.05..=2.00,
                            self.rt_press_mm,
                            0.05,
                            format!("{:.2} mm", self.rt_press_mm),
                            accent,
                            Message::RtPressChanged,
                        ),
                        slider_row(
                            l.t("Sensibilidade Soltar", "Release Sensitivity"),
                            0.05..=2.00,
                            self.rt_release_mm,
                            0.05,
                            format!("{:.2} mm", self.rt_release_mm),
                            accent,
                            Message::RtReleaseChanged,
                        ),
                    ]
                    .spacing(SP_SM)
                } else {
                    column![text(l.t(
                        "Rapid Trigger está desativado. Ative a chave acima para configurar a sensibilidade dinâmica.",
                        "Rapid Trigger is off. Toggle the switch above to configure dynamic press/release sensitivity.",
                    ))
                    .size(type_scale::CAPTION + 1)
                    .color(TEXT_MUTED)]
                };

                let rt_card = card(
                    "Rapid Trigger",
                    Some(
                        l.t(
                            "Reinicia a tecla instantaneamente ao começar a soltar, permitindo repetições muito mais rápidas.",
                            "Resets the key immediately as you begin releasing, enabling rapid repeated inputs.",
                        )
                        .to_string(),
                    ),
                    Some(rt_actions.into()),
                    rt_body.into(),
                );

                let db_card = card(
                    l.t("Zona Morta (Dead Band)", "Dead Band"),
                    Some(
                        l.t(
                            "Margem de proteção no repouso (topo) e no fim de curso (fundo) para evitar toques acidentais.",
                            "Safety margin at rest (top) and bottom-out to avoid accidental triggers or chatter.",
                        )
                        .to_string(),
                    ),
                    None,
                    column![
                        slider_row(
                            l.t("Zona Morta Topo", "Top Dead Zone"),
                            0.00..=1.00,
                            self.dead_top_mm,
                            0.05,
                            format!("{:.2} mm", self.dead_top_mm),
                            accent,
                            Message::DeadBandTopChanged,
                        ),
                        slider_row(
                            l.t("Zona Morta Fundo", "Bottom Dead Zone"),
                            0.00..=1.00,
                            self.dead_bot_mm,
                            0.05,
                            format!("{:.2} mm", self.dead_bot_mm),
                            accent,
                            Message::DeadBandBottomChanged,
                        ),
                    ]
                    .spacing(SP_SM)
                    .into(),
                );

                column![gauge_card, act_card, rt_card, db_card]
                    .spacing(SP_MD)
                    .into()
            }
            PerfSubTab::Calibration => {
                let calibrated = self.snap.populated.iter().filter(|&&p| p).count();
                let total = self.snap.board.total_keys();
                let focus_idx = self.selected_keys.iter().next().copied().unwrap_or(0);
                let live_adc = self.snap.adc_raw.get(focus_idx).copied().unwrap_or(0);
                let live_mm = self.snap.trip_mm.get(focus_idx).copied().unwrap_or(0.0);

                card(
                    l.t("Calibração dos Switches", "Switch Calibration"),
                    Some(
                        l.t(
                            "Clique em \"Iniciar Calibração\", pressione todas as teclas firmemente até o final e clique em \"Finalizar\". O teclado acima mostra o valor ADC bruto de cada sensor.",
                            "Click Start Calibration, press every key fully to bottom-out, then click Finish Calibration. The board above shows live raw ADC readings per switch.",
                        )
                        .to_string(),
                    ),
                    Some(
                        row![
                            badge(format!("ADC #{focus_idx}: {live_adc} ({live_mm:.2}mm)"), TEXT_SECONDARY),
                            badge(format!("{calibrated} / {total}"), accent),
                        ]
                        .spacing(6)
                        .into(),
                    ),
                    row![
                        button(
                            row![
                                icon(Icon::Zap, 14, contrast_on(accent)),
                                text(l.t("Iniciar Calibração", "Start Calibration"))
                                    .size(type_scale::BODY)
                                    .font(FONT_MEDIUM),
                            ]
                            .spacing(7)
                            .align_y(Alignment::Center),
                        )
                        .padding([8, 16])
                        .style(primary_button_style(accent))
                        .on_press(Message::RequestStartCalibration),
                        button(
                            row![
                                icon(Icon::Check, 14, TEXT_PRIMARY),
                                text(l.t("Finalizar Calibração", "Finish Calibration"))
                                    .size(type_scale::BODY),
                            ]
                            .spacing(7)
                            .align_y(Alignment::Center),
                        )
                        .padding([8, 16])
                        .style(secondary_button_style)
                        .on_press(Message::FinishCalibration),
                    ]
                    .spacing(10)
                    .into(),
                )
            }
            PerfSubTab::Performance => {
                let cfg = self.snap.config.as_ref();
                let gm = cfg.map(|c| c.game_mode != 0).unwrap_or(false);
                let bo = cfg.map(|c| c.bottom_optimize != 0).unwrap_or(false);
                let f = cfg.map(|c| c.feature).unwrap_or_default();

                card(
                    l.t("Modos e Recursos de Hardware", "Hardware Modes & Flags"),
                    None,
                    None,
                    column![
                        toggle_row(
                            l.t("Modo Jogo (Game Mode)", "Game Mode"),
                            Some(
                                l.t(
                                    "Prioriza resposta mínima de latência no controlador.",
                                    "Optimizes controller scan loop for minimum gaming latency.",
                                )
                                .to_string(),
                            ),
                            gm,
                            accent,
                            Message::GameModeToggled,
                        ),
                        toggle_row(
                            l.t("Otimização de Fim de Curso", "Bottom Optimize"),
                            Some(
                                l.t(
                                    "Suaviza a leitura do sensor quando o switch chega ao fundo.",
                                    "Stabilizes Hall sensor reading at bottom-out.",
                                )
                                .to_string(),
                            ),
                            bo,
                            accent,
                            Message::BottomOptimizeToggled,
                        ),
                        toggle_row(
                            l.t("Área RGB", "RGB Area"),
                            Some(
                                l.t(
                                    "Habilita iluminação de área secundária no teclado.",
                                    "Enables secondary RGB lighting zone.",
                                )
                                .to_string(),
                            ),
                            f.rgb_area != 0,
                            accent,
                            Message::FeatureRgbAreaToggled,
                        ),
                        toggle_row(
                            l.t("Inverter WASD / Setas", "Swap WASD / Arrows"),
                            Some(
                                l.t(
                                    "Troca a função física das teclas WASD pelas setas direcionais.",
                                    "Swaps WASD keys with arrow keys.",
                                )
                                .to_string(),
                            ),
                            f.wasd_switch != 0,
                            accent,
                            Message::FeatureWasdToggled,
                        ),
                        toggle_row(
                            l.t("Modo macOS", "macOS Mode"),
                            Some(
                                l.t(
                                    "Ajusta modificadores (Alt/Cmd) para o padrão Mac.",
                                    "Swaps Option/Command modifiers for macOS.",
                                )
                                .to_string(),
                            ),
                            f.mac_switch != 0,
                            accent,
                            Message::FeatureMacToggled,
                        ),
                        toggle_row(
                            l.t("Trava Tecla Windows (Win Lock)", "Win Lock"),
                            Some(
                                l.t(
                                    "Desabilita a tecla Super/Windows para evitar acidentes em jogos.",
                                    "Disables the Windows/Super key to prevent accidental presses.",
                                )
                                .to_string(),
                            ),
                            f.win_lock != 0,
                            accent,
                            Message::FeatureWinLockToggled,
                        ),
                        toggle_row(
                            "NKRO (N-Key Rollover)",
                            Some(
                                l.t(
                                    "Permite registrar todas as teclas simultaneamente sem ghosting.",
                                    "Allows simultaneous registration of all keys without ghosting.",
                                )
                                .to_string(),
                            ),
                            f.nkro_switch != 0,
                            accent,
                            Message::FeatureNkroToggled,
                        ),
                    ]
                    .spacing(SP_MD)
                    .into(),
                )
            }
        };

        column![sub_bar, body].spacing(SP_MD).into()
    }
}
