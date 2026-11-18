//! Application shell: sidebar navigation, top header with profile selector & unified Save,
//! collapsible keyboard view, confirmation & diagnostic modals, and floating overlay menus / toasts.

use crate::app::{KeyZone, Message, NavSection, OpenMadApp, PerfSubTab};
use crate::theme::{
    card_style, confirm_button_style, contrast_on, danger_button_style, elevated_card_style,
    ghost_button_style, header_style, input_style, nav_button_style, pick_list_style,
    primary_button_style, secondary_button_style, sidebar_style, toast_style, type_scale,
    warn_banner_style, with_alpha, AMBER_WARN, BG_BASE, BORDER_SUBTLE, FONT_MEDIUM, FONT_SEMIBOLD,
    GREEN_OK, RED_DANGER, SP_LG, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::components::{badge, empty_state};
use crate::ui::icons::{icon, Icon};
use crate::widgets::keyboard::{BoardViewMode, KeyboardCanvas};
use iced::widget::{
    button, column, container, mouse_area, pick_list, row, scrollable, stack, text, text_input,
    Column, Row, Space,
};
use iced::{Alignment, Color, Element, Length};
use openmad_core::diagnostics::DiagnosticOutput;
use openmad_core::protocol::KNOWN_PROTOCOL_VERSION;
use openmad_core::updater::APP_VERSION;

impl OpenMadApp {
    pub fn view_shell(&self) -> Element<'_, Message> {
        let accent = self.accent();
        let sidebar = self.view_sidebar(accent);
        let header = self.view_header(accent);
        let body = self.view_body(accent);

        let main_col = column![header, body]
            .width(Length::Fill)
            .height(Length::Fill);

        let base = container(
            row![sidebar, main_col]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_BASE)),
            ..Default::default()
        });

        let mut layers: Vec<Element<'_, Message>> = vec![base.into()];

        if self.profile_menu_open || self.save_menu_open {
            let backdrop = mouse_area(
                Space::with_width(Length::Fill).height(Length::Fill),
            )
            .on_press(Message::CloseMenus);
            layers.push(backdrop.into());
        }

        if self.profile_menu_open {
            layers.push(self.view_profile_menu(accent));
        }

        if self.save_menu_open {
            layers.push(self.view_save_menu(accent));
        }

        if let Some(ref dlg) = self.confirm_dialog {
            layers.push(self.view_confirm_modal(dlg, accent));
        }

        if let Some(ref diag) = self.diag_output {
            layers.push(self.view_diag_modal(diag, accent));
        }

        if let Some((ref msg, is_err, _)) = self.toast {
            layers.push(self.view_toast_overlay(msg, is_err));
        }

        stack(layers).into()
    }

    fn view_sidebar(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        let brand = row![
            container(icon(Icon::Keyboard, 16, accent))
                .padding(8)
                .style(move |_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(with_alpha(accent, 0.14))),
                    border: iced::Border {
                        color: with_alpha(accent, 0.35),
                        width: 1.0,
                        radius: 9.0.into(),
                    },
                    ..Default::default()
                }),
            column![
                text("OpenMAD")
                    .size(16)
                    .font(FONT_SEMIBOLD)
                    .color(TEXT_PRIMARY),
                text(format!("v{APP_VERSION}"))
                    .size(type_scale::CAPTION)
                    .color(TEXT_MUTED),
            ]
            .spacing(1),
            Space::with_width(Length::Fill),
            button(
                text(if l.is_pt() { "PT" } else { "EN" })
                    .size(type_scale::CAPTION)
                    .font(FONT_MEDIUM),
            )
            .padding([4, 8])
            .style(secondary_button_style)
            .on_press(Message::ToggleLanguage),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let primary_items = [
            (NavSection::Keys, Icon::Keyboard, l.t("Teclas", "Keymap")),
            (NavSection::Lighting, Icon::Lightbulb, l.t("Iluminação", "Lighting")),
            (NavSection::Performance, Icon::Zap, l.t("Performance", "Performance")),
            (NavSection::Advanced, Icon::Layers, l.t("Avançado", "Advanced")),
            (NavSection::Macros, Icon::CircleDot, "Macros"),
            (NavSection::AppTrigger, Icon::AppWindow, l.t("Troca por App", "App Trigger")),
        ];

        let mut nav_primary = Column::new().spacing(4);
        for (sec, ic, lbl) in primary_items {
            nav_primary = nav_primary.push(self.nav_item(sec, ic, lbl, accent));
        }

        let divider = container(Space::with_height(Length::Fixed(1.0)))
            .width(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(BORDER_SUBTLE)),
                ..Default::default()
            });

        let sys_items = [
            (NavSection::System, Icon::HardDrive, l.t("Sistema & Flash", "System & Flash")),
            (NavSection::Settings, Icon::Settings, l.t("Configurações", "Settings")),
        ];
        let mut nav_sys = Column::new().spacing(4);
        for (sec, ic, lbl) in sys_items {
            nav_sys = nav_sys.push(self.nav_item(sec, ic, lbl, accent));
        }

        let status_color = if self.snap.connected {
            if self.snap.read_only {
                AMBER_WARN
            } else {
                GREEN_OK
            }
        } else if self.snap.in_bootloader {
            AMBER_WARN
        } else {
            RED_DANGER
        };

        let dev_title = if self.snap.connected {
            self.snap.board.name.to_string()
        } else if self.snap.in_bootloader {
            l.t("Modo Bootloader", "Bootloader Mode").to_string()
        } else {
            l.t("Desconectado", "Disconnected").to_string()
        };

        let dev_sub = if self.snap.connected {
            let pv = self.snap.protocol_version.unwrap_or(0);
            if self.snap.read_only {
                format!("v{pv} · {}", l.t("Somente leitura", "Read-only"))
            } else {
                format!("fw proto v{pv} · USB")
            }
        } else {
            l.t("Conecte via USB", "Connect via USB").to_string()
        };

        let device_card = container(
            column![
                row![
                    container(Space::with_width(Length::Fixed(8.0)).height(Length::Fixed(8.0)))
                        .style(move |_| iced::widget::container::Style {
                            background: Some(iced::Background::Color(status_color)),
                            border: iced::Border {
                                radius: 999.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    column![
                        text(dev_title)
                            .size(type_scale::CAPTION + 1)
                            .font(FONT_SEMIBOLD)
                            .color(TEXT_PRIMARY),
                        text(dev_sub)
                            .size(type_scale::CAPTION)
                            .color(TEXT_MUTED),
                    ]
                    .spacing(1),
                ]
                .spacing(9)
                .align_y(Alignment::Center),
                button(
                    row![
                        icon(Icon::Minimize, 13, TEXT_SECONDARY),
                        text(l.t("Minimizar p/ bandeja", "Minimize to tray"))
                            .size(type_scale::CAPTION + 1)
                            .font(FONT_MEDIUM)
                            .color(TEXT_SECONDARY),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([6, 10])
                .style(secondary_button_style)
                .on_press(Message::HideToTray),
            ]
            .spacing(10)
            .padding(12),
        )
        .width(Length::Fill)
        .style(elevated_card_style);

        let content = column![
            brand,
            Space::with_height(Length::Fixed(6.0)),
            nav_primary,
            divider,
            nav_sys,
            Space::with_height(Length::Fill),
            device_card,
        ]
        .spacing(10)
        .padding(SP_LG);

        container(content)
            .width(Length::Fixed(228.0))
            .height(Length::Fill)
            .style(sidebar_style)
            .into()
    }

    fn nav_item<'a>(
        &self,
        section: NavSection,
        ic: Icon,
        label: &'a str,
        accent: Color,
    ) -> Element<'a, Message> {
        let active = self.section == section;
        let icon_color = if active { accent } else { TEXT_SECONDARY };
        let text_color = if active { TEXT_PRIMARY } else { TEXT_SECONDARY };

        let bar = container(Space::with_width(Length::Fixed(3.0)).height(Length::Fixed(16.0)))
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(if active {
                    accent
                } else {
                    Color::TRANSPARENT
                })),
                border: iced::Border {
                    radius: 999.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        button(
            row![
                bar,
                icon(ic, 15, icon_color),
                text(label)
                    .size(type_scale::BODY)
                    .font(if active { FONT_SEMIBOLD } else { FONT_MEDIUM })
                    .color(text_color),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([8, 10])
        .style(nav_button_style(active, accent))
        .on_press(Message::SelectSection(section))
        .into()
    }

    fn view_header(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let title = self.section.title(l);

        let prof_list = if self.profiles.is_empty() {
            vec![l.t("(Sem perfil)", "(No profile)").to_string()]
        } else {
            self.profiles.clone()
        };
        let cur_prof = self
            .selected_profile
            .clone()
            .or_else(|| prof_list.first().cloned());

        let prof_picker = pick_list(prof_list, cur_prof, Message::SelectProfile)
            .style(pick_list_style)
            .padding([6, 10])
            .text_size(type_scale::BODY);

        let mut right_row = Row::new().spacing(SP_SM).align_y(Alignment::Center);

        if self.has_unsaved_changes() {
            right_row = right_row.push(badge(
                l.t("Alterações não salvas", "Unsaved changes"),
                AMBER_WARN,
            ));
        }

        right_row = right_row.push(
            row![
                text(l.t("Perfil:", "Profile:"))
                    .size(type_scale::CAPTION + 1)
                    .color(TEXT_MUTED),
                prof_picker,
                button(icon(Icon::MoreHorizontal, 15, TEXT_PRIMARY))
                    .padding([7, 9])
                    .style(secondary_button_style)
                    .on_press(Message::ToggleProfileMenu),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );

        let save_btn = row![
            button(
                row![
                    icon(Icon::Save, 14, contrast_on(accent)),
                    text(l.t("Salvar", "Save"))
                        .size(type_scale::BODY)
                        .font(FONT_SEMIBOLD),
                    icon(Icon::ChevronDown, 13, contrast_on(accent)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([7, 13])
            .style(primary_button_style(accent))
            .on_press(Message::ToggleSaveMenu),
        ];

        right_row = right_row.push(save_btn);

        container(
            row![
                text(title)
                    .size(type_scale::TITLE)
                    .font(FONT_SEMIBOLD)
                    .color(TEXT_PRIMARY),
                Space::with_width(Length::Fill),
                right_row,
            ]
            .align_y(Alignment::Center)
            .padding([12, 22]),
        )
        .width(Length::Fill)
        .style(header_style)
        .into()
    }

    fn view_body(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let mut col = Column::new().spacing(SP_MD).padding([16, 22]);

        if self.snap.connected && !self.snap.firmware_verified {
            let pv = self.snap.protocol_version.unwrap_or(0);
            col = col.push(
                container(
                    row![
                        icon(Icon::AlertTriangle, 15, AMBER_WARN),
                        text(format!(
                            "{} v{pv} != v{KNOWN_PROTOCOL_VERSION}. {}",
                            l.t("Aviso: Protocolo do firmware", "Warning: Firmware protocol"),
                            l.t("Faça um backup antes de gravar.", "Take a backup before writing.")
                        ))
                        .size(type_scale::CAPTION + 1)
                        .color(AMBER_WARN),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(warn_banner_style),
            );
        }

        if self.snap.connected && self.snap.read_only {
            col = col.push(
                container(
                    row![
                        icon(Icon::AlertTriangle, 15, AMBER_WARN),
                        text(
                            self.snap
                                .read_only_reason
                                .as_deref()
                                .unwrap_or(l.t("Placa em modo somente leitura", "Board opened read-only")),
                        )
                        .size(type_scale::CAPTION + 1)
                        .color(AMBER_WARN),
                        Space::with_width(Length::Fill),
                        button(text(l.t("Desbloquear Escrita", "Unlock Writes")).size(type_scale::CAPTION + 1))
                            .padding([4, 10])
                            .style(secondary_button_style)
                            .on_press(Message::RequestUnlockBoard),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 12])
                .style(warn_banner_style),
            );
        }

        if matches!(
            self.section,
            NavSection::Keys | NavSection::Lighting | NavSection::Performance | NavSection::Advanced
        ) {
            col = col.push(self.view_keyboard_card(accent));
        }

        let page_content: Element<'_, Message> = match self.section {
            NavSection::Keys => self.view_keys_page(accent),
            NavSection::Lighting => self.view_lighting_page(accent),
            NavSection::Performance => self.view_performance_page(accent),
            NavSection::Advanced => self.view_advanced_page(accent),
            NavSection::Macros => self.view_macros_page(accent),
            NavSection::AppTrigger => self.view_app_trigger_page(accent),
            NavSection::System => self.view_system_page(accent),
            NavSection::Settings => self.view_settings_page(accent),
        };

        col = col.push(scrollable(page_content).height(Length::Fill));

        col.width(Length::Fill).height(Length::Fill).into()
    }

    fn view_keyboard_card(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;

        if !self.snap.connected && self.screenshot_target.is_none() {
            let checklist = column![
                text(l.t("1. Verifique se o cabo USB-C está conectado diretamente ao PC.", "1. Ensure the USB-C cable is plugged directly into your PC."))
                    .size(type_scale::CAPTION + 1)
                    .color(TEXT_MUTED),
                text(l.t("2. No Linux, garanta que a regra udev 60-openmad.rules está instalada.", "2. On Linux, make sure the 60-openmad.rules udev rule is installed."))
                    .size(type_scale::CAPTION + 1)
                    .color(TEXT_MUTED),
                text(l.t("3. O teclado não pode estar em modo bootloader / atualização de firmware.", "3. Make sure the keyboard is not in bootloader / firmware update mode."))
                    .size(type_scale::CAPTION + 1)
                    .color(TEXT_MUTED),
            ]
            .spacing(4)
            .align_x(Alignment::Start);

            let err_note = self.snap.error.as_deref().map(|e| {
                text(e).size(type_scale::CAPTION).color(AMBER_WARN)
            });

            let extra = column![checklist]
                .push_maybe(err_note)
                .spacing(6)
                .align_x(Alignment::Center)
                .into();

            return empty_state(
                Icon::Usb,
                l.t("Nenhum teclado detectado", "No keyboard detected"),
                l.t(
                    "Aguardando dispositivo MAD68 HE / MAD60 HE (VID 0x373B)…",
                    "Waiting for MAD68 HE / MAD60 HE device (VID 0x373B)…",
                ),
                Some(extra),
                None,
                accent,
            );
        }

        let sel_hint = if self.adv_pick_target.is_some()
            && self.section == NavSection::Advanced
        {
            l.t(
                "Modo seleção de par: clique em uma tecla abaixo para atribuir",
                "Pair picking mode: click a key below to assign",
            )
            .to_string()
        } else if self.selected_keys.is_empty() {
            l.t(
                "0 teclas selecionadas (ajustes aplicam em todo o teclado)",
                "0 keys selected (edits apply to the whole keyboard)",
            )
            .to_string()
        } else {
            format!(
                "{} {}",
                self.selected_keys.len(),
                l.t("tecla(s) selecionada(s)", "key(s) selected")
            )
        };

        let toolbar = row![
            text(sel_hint)
                .size(type_scale::CAPTION + 1)
                .font(FONT_MEDIUM)
                .color(TEXT_SECONDARY),
            Space::with_width(Length::Fill),
            button(text("WASD").size(type_scale::CAPTION))
                .padding([4, 8])
                .style(secondary_button_style)
                .on_press(Message::SelectKeyZone(KeyZone::Wasd)),
            button(text(l.t("Setas", "Arrows")).size(type_scale::CAPTION))
                .padding([4, 8])
                .style(secondary_button_style)
                .on_press(Message::SelectKeyZone(KeyZone::Arrows)),
            button(text(l.t("Números", "Numbers")).size(type_scale::CAPTION))
                .padding([4, 8])
                .style(secondary_button_style)
                .on_press(Message::SelectKeyZone(KeyZone::Numbers)),
            button(text("Mods").size(type_scale::CAPTION))
                .padding([4, 8])
                .style(secondary_button_style)
                .on_press(Message::SelectKeyZone(KeyZone::Modifiers)),
            button(text(l.t("Todas", "All")).size(type_scale::CAPTION))
                .padding([4, 9])
                .style(secondary_button_style)
                .on_press(Message::SelectAllKeys),
            button(text(l.t("Limpar", "Clear")).size(type_scale::CAPTION))
                .padding([4, 9])
                .style(secondary_button_style)
                .on_press(Message::SelectNoKeys),
            button(text(l.t("Inverter", "Invert")).size(type_scale::CAPTION))
                .padding([4, 9])
                .style(secondary_button_style)
                .on_press(Message::InvertKeys),
            button(
                icon(
                    if self.keyboard_collapsed {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronUp
                    },
                    14,
                    TEXT_SECONDARY,
                ),
            )
            .padding([4, 8])
            .style(ghost_button_style)
            .on_press(Message::ToggleKeyboardCollapsed),
        ]
        .spacing(5)
        .align_y(Alignment::Center);

        let mut card_col = Column::new().spacing(SP_SM).padding([10, 12]);
        card_col = card_col.push(toolbar);

        if !self.keyboard_collapsed {
            let mode = match self.section {
                NavSection::Lighting => BoardViewMode::Lighting,
                NavSection::Performance => BoardViewMode::Performance,
                NavSection::Advanced => BoardViewMode::Advanced,
                _ => BoardViewMode::Keymap,
            };
            let cfg_ref = self.snap.config.as_ref();
            let adv_pair_highlight = if self.section == NavSection::Advanced {
                Some((
                    (
                        self.adv_k1_r.parse::<usize>().unwrap_or(0),
                        self.adv_k1_c.parse::<usize>().unwrap_or(0),
                    ),
                    (
                        self.adv_k2_r.parse::<usize>().unwrap_or(0),
                        self.adv_k2_c.parse::<usize>().unwrap_or(0),
                    ),
                ))
            } else {
                None
            };
            let kb = KeyboardCanvas {
                rows: self.snap.board.rows,
                cols: self.snap.board.cols,
                populated: &self.snap.populated,
                selected: &self.selected_keys,
                mode,
                layer: self.layer,
                keymap: cfg_ref.map(|c| c.keymap.as_slice()).unwrap_or(&[]),
                actuation: cfg_ref.map(|c| c.actuation.as_slice()).unwrap_or(&[]),
                rapid_trigger: cfg_ref.map(|c| c.rapid_trigger.as_slice()).unwrap_or(&[]),
                trip_mm: &self.snap.trip_mm,
                adc_raw: &self.snap.adc_raw,
                calibration_view: self.section == NavSection::Performance
                    && self.perf_sub == PerfSubTab::Calibration,
                key_colors: cfg_ref.map(|c| c.key_colors.as_slice()).unwrap_or(&[]),
                light_effect: self.light_draft.effect,
                light_color: [self.light_draft.r, self.light_draft.g, self.light_draft.b],
                advanced_keys: cfg_ref.map(|c| c.advanced_keys.as_slice()).unwrap_or(&[]),
                adv_pair_highlight,
                accent,
            };
            card_col = card_col.push(kb.view(Message::Keyboard));
        }

        container(card_col)
            .width(Length::Fill)
            .style(card_style)
            .into()
    }

    fn view_profile_menu(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let has_sel = self.selected_profile.is_some();

        let new_row = row![
            text_input(l.t("Novo perfil…", "New profile…"), &self.new_profile_name)
                .on_input(Message::NewProfileNameChanged)
                .on_submit(Message::CreateProfile(false))
                .padding([6, 9])
                .size(type_scale::CAPTION + 1)
                .style(input_style(accent)),
            button(icon(Icon::Plus, 14, contrast_on(accent)))
                .padding([6, 10])
                .style(primary_button_style(accent))
                .on_press(Message::CreateProfile(false)),
        ]
        .spacing(6);

        let mut items = Column::new().spacing(4);
        items = items.push(new_row);
        items = items.push(self.menu_action_btn(
            Icon::Upload,
            l.t("Importar JSON…", "Import JSON…"),
            Message::ImportProfile,
            false,
        ));
        if has_sel {
            items = items.push(self.menu_action_btn(
                Icon::Download,
                l.t("Exportar perfil", "Export profile"),
                Message::ExportProfile,
                false,
            ));
            items = items.push(self.menu_action_btn(
                Icon::Copy,
                l.t("Duplicar perfil", "Duplicate profile"),
                Message::DuplicateProfile,
                false,
            ));
            items = items.push(self.menu_action_btn(
                Icon::Pencil,
                l.t("Renomear (usa campo acima)", "Rename (uses box above)"),
                Message::RenameProfile,
                false,
            ));
            items = items.push(self.menu_action_btn(
                Icon::HardDrive,
                l.t("Definir como padrão Onboard", "Set as Onboard default"),
                Message::RequestSaveProfileOnboard,
                false,
            ));
            items = items.push(self.menu_action_btn(
                Icon::Trash,
                l.t("Excluir perfil", "Delete profile"),
                Message::RequestDeleteProfile,
                true,
            ));
        }

        let panel = container(items.padding(10))
            .width(Length::Fixed(260.0))
            .style(toast_style(false));

        container(
            row![
                Space::with_width(Length::Fill),
                column![Space::with_height(Length::Fixed(56.0)), panel],
                Space::with_width(Length::Fixed(118.0)),
            ],
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_save_menu(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let prof_label = self
            .selected_profile
            .as_deref()
            .map(|n| format!("{} \"{n}\"", l.t("Salvar no perfil", "Save to profile")))
            .unwrap_or_else(|| l.t("Salvar no perfil", "Save to profile").to_string());

        let panel = container(
            column![
                button(
                    row![
                        icon(Icon::Save, 14, accent),
                        column![
                            text(prof_label)
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM)
                                .color(TEXT_PRIMARY),
                            text(l.t("Salva em JSON no computador", "Saves JSON profile on disk"))
                                .size(type_scale::CAPTION)
                                .color(TEXT_MUTED),
                        ]
                        .spacing(1),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 10])
                .style(ghost_button_style)
                .on_press(Message::SaveCurrentProfile),
                button(
                    row![
                        icon(Icon::HardDrive, 14, GREEN_OK),
                        column![
                            text(l.t("Gravar na Memória Flash", "Write to Keyboard Flash"))
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM)
                                .color(TEXT_PRIMARY),
                            text(l.t("Persiste no teclado ao desligar", "Persists on keyboard across reboots"))
                                .size(type_scale::CAPTION)
                                .color(TEXT_MUTED),
                        ]
                        .spacing(1),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([8, 10])
                .style(ghost_button_style)
                .on_press(Message::RequestCommitToFlash),
            ]
            .spacing(4)
            .padding(8),
        )
        .width(Length::Fixed(275.0))
        .style(toast_style(false));

        container(
            row![
                Space::with_width(Length::Fill),
                column![Space::with_height(Length::Fixed(56.0)), panel],
                Space::with_width(Length::Fixed(22.0)),
            ],
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_confirm_modal<'a>(
        &self,
        dlg: &'a crate::app::ConfirmDialog,
        accent: Color,
    ) -> Element<'a, Message> {
        let l = self.lang;
        let confirm_btn = button(
            text(&dlg.confirm_label)
                .size(type_scale::BODY)
                .font(FONT_MEDIUM),
        )
        .padding([8, 16])
        .style(confirm_button_style(dlg.danger, accent))
        .on_press(Message::ConfirmDialogAccept);

        let panel = container(
            column![
                row![
                    icon(
                        Icon::AlertTriangle,
                        18,
                        if dlg.danger { RED_DANGER } else { AMBER_WARN },
                    ),
                    text(&dlg.title)
                        .size(type_scale::TITLE - 2)
                        .font(FONT_SEMIBOLD)
                        .color(TEXT_PRIMARY),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text(&dlg.body)
                    .size(type_scale::BODY)
                    .color(TEXT_SECONDARY),
                row![
                    Space::with_width(Length::Fill),
                    button(text(l.t("Cancelar", "Cancel")).size(type_scale::BODY))
                        .padding([8, 14])
                        .style(secondary_button_style)
                        .on_press(Message::ConfirmDialogCancel),
                    confirm_btn,
                ]
                .spacing(10),
            ]
            .spacing(SP_MD)
            .padding(SP_LG),
        )
        .width(Length::Fixed(460.0))
        .style(card_style);

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.65))),
                ..Default::default()
            })
            .into()
    }

    fn view_diag_modal<'a>(
        &self,
        diag: &'a DiagnosticOutput,
        accent: Color,
    ) -> Element<'a, Message> {
        let l = self.lang;
        let wp = &diag.report.write_probe;
        let wp_badge = if !wp.ran {
            badge(l.t("Somente leitura", "Read-only"), TEXT_SECONDARY)
        } else if wp.match_ok == Some(true) && wp.restored != Some(false) {
            badge(l.t("Write Probe: PASSOU", "Write Probe: PASSED"), GREEN_OK)
        } else {
            badge(l.t("Write Probe: FALHOU", "Write Probe: FAILED"), RED_DANGER)
        };

        let panel = container(
            column![
                row![
                    icon(Icon::HardDrive, 18, accent),
                    text(l.t("Relatório de Diagnóstico da Placa", "Board Diagnostic Report"))
                        .size(type_scale::TITLE - 2)
                        .font(FONT_SEMIBOLD)
                        .color(TEXT_PRIMARY),
                    Space::with_width(Length::Fill),
                    wp_badge,
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text(format!(
                    "{}: {}",
                    l.t("Salvo em", "Saved to"),
                    diag.path.display()
                ))
                .size(type_scale::CAPTION + 1)
                .color(TEXT_MUTED),
                container(
                    scrollable(
                        text(&diag.json)
                            .size(type_scale::CAPTION)
                            .color(TEXT_SECONDARY),
                    )
                    .height(Length::Fixed(280.0)),
                )
                .width(Length::Fill)
                .padding(10)
                .style(elevated_card_style),
                row![
                    Space::with_width(Length::Fill),
                    button(text(l.t("Fechar", "Close")).size(type_scale::BODY))
                        .padding([8, 14])
                        .style(secondary_button_style)
                        .on_press(Message::CloseDiagModal),
                    button(
                        row![
                            icon(Icon::Copy, 13, TEXT_PRIMARY),
                            text(l.t("Copiar JSON", "Copy JSON")).size(type_scale::BODY),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([8, 14])
                    .style(secondary_button_style)
                    .on_press(Message::CopyDiagJson),
                    button(
                        row![
                            icon(Icon::ExternalLink, 13, contrast_on(accent)),
                            text(l.t("Abrir Issue no GitHub", "Open GitHub Issue"))
                                .size(type_scale::BODY)
                                .font(FONT_MEDIUM),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([8, 14])
                    .style(primary_button_style(accent))
                    .on_press(Message::OpenDiagIssueUrl),
                ]
                .spacing(10),
            ]
            .spacing(SP_MD)
            .padding(SP_LG),
        )
        .width(Length::Fixed(640.0))
        .style(card_style);

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.65))),
                ..Default::default()
            })
            .into()
    }

    fn menu_action_btn<'a>(
        &self,
        ic: Icon,
        label: &'a str,
        msg: Message,
        danger: bool,
    ) -> Element<'a, Message> {
        let color = if danger { RED_DANGER } else { TEXT_PRIMARY };
        let b = button(
            row![icon(ic, 14, color), text(label).size(type_scale::CAPTION + 1).color(color)]
                .spacing(9)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([7, 10]);

        if danger {
            b.style(danger_button_style).on_press(msg).into()
        } else {
            b.style(ghost_button_style).on_press(msg).into()
        }
    }

    fn view_toast_overlay<'a>(&self, msg: &'a str, is_err: bool) -> Element<'a, Message> {
        let accent_c = if is_err { RED_DANGER } else { GREEN_OK };
        let ic = if is_err { Icon::AlertTriangle } else { Icon::Check };

        let toast = container(
            row![
                icon(ic, 15, accent_c),
                text(msg)
                    .size(type_scale::BODY)
                    .font(FONT_MEDIUM)
                    .color(TEXT_PRIMARY),
                button(icon(Icon::X, 13, TEXT_SECONDARY))
                    .padding([3, 6])
                    .style(ghost_button_style)
                    .on_press(Message::DismissToast),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([10, 14])
        .style(toast_style(is_err));

        container(
            column![
                Space::with_height(Length::Fill),
                row![Space::with_width(Length::Fill), toast],
            ]
            .padding(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
