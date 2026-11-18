//! App Trigger page: per-application automatic profile switching.

use crate::app::{Message, OpenMadApp};
use crate::theme::{
    contrast_on, elevated_card_style, ghost_button_style, input_style, pick_list_style,
    primary_button_style, secondary_button_style, toggler_style, type_scale, FONT_MEDIUM,
    FONT_SEMIBOLD, RED_DANGER, SP_MD, SP_SM, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::components::{badge, card, empty_state};
use crate::ui::icons::{icon, Icon};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, toggler, Column, Row,
    Space,
};
use iced::{Alignment, Color, Element, Length};
use openmad_core::switcher::watcher_backend_name;

impl OpenMadApp {
    pub fn view_app_trigger_page(&self, accent: Color) -> Element<'_, Message> {
        let l = self.lang;
        let none_lbl = l.t("(Nenhum)", "(None)").to_string();
        let mut prof_choices = vec![none_lbl.clone()];
        prof_choices.extend(self.profiles.iter().cloned());
        let cur_default = self
            .switcher_cfg
            .default_profile
            .clone()
            .unwrap_or(none_lbl);

        let status_actions = row![
            badge(format!("Backend: {}", watcher_backend_name()), accent),
            toggler(self.switcher_cfg.enabled)
                .style(toggler_style(accent))
                .on_toggle(Message::ToggleAutoSwitcher),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let timing_row = row![
            text(l.t("Debounce (ms):", "Debounce (ms):"))
                .size(type_scale::CAPTION + 1)
                .color(TEXT_SECONDARY),
            text_input("400", &self.switcher_cfg.debounce_ms.to_string())
                .on_input(Message::SwitcherDebounceChanged)
                .width(Length::Fixed(72.0))
                .padding([5, 8])
                .size(type_scale::CAPTION + 1)
                .style(input_style(accent)),
            text(l.t("Permanência mín. (ms):", "Min Dwell (ms):"))
                .size(type_scale::CAPTION + 1)
                .color(TEXT_SECONDARY),
            text_input("800", &self.switcher_cfg.min_dwell_ms.to_string())
                .on_input(Message::SwitcherDwellChanged)
                .width(Length::Fixed(72.0))
                .padding([5, 8])
                .size(type_scale::CAPTION + 1)
                .style(input_style(accent)),
            Space::with_width(Length::Fill),
            text(l.t("Perfil Padrão (Fallback):", "Default Fallback:"))
                .size(type_scale::BODY)
                .color(TEXT_SECONDARY),
            pick_list(prof_choices, Some(cur_default), Message::SetDefaultProfile)
                .style(pick_list_style)
                .padding([6, 10]),
        ]
        .spacing(SP_SM)
        .align_y(Alignment::Center);

        let status_body = column![
            row![
                icon(Icon::AppWindow, 14, accent),
                text(l.t("App em foco:", "Foreground App:"))
                    .size(type_scale::BODY)
                    .color(TEXT_SECONDARY),
                text(
                    self.snap
                        .foreground_exe
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("—"),
                )
                .size(type_scale::BODY)
                .font(FONT_SEMIBOLD)
                .color(accent),
            ]
            .spacing(SP_SM)
            .align_y(Alignment::Center),
            timing_row,
        ]
        .spacing(SP_SM);

        let status_card = card(
            l.t("Troca Automática por App", "Auto-Switcher"),
            Some(
                l.t(
                    "Aplica automaticamente o perfil vinculado ao focar uma janela (sem desgaste de memória flash).",
                    "Automatically switches profiles in RAM when a matching app window gains focus (zero flash wear).",
                )
                .to_string(),
            ),
            Some(status_actions.into()),
            status_body.into(),
        );

        let mut proc_row = Row::new().spacing(6);
        for p in self.processes.iter().take(16) {
            proc_row = proc_row.push(
                button(
                    row![
                        icon(Icon::Plus, 11, accent),
                        text(p.exe.clone()).size(type_scale::CAPTION),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                )
                .padding([5, 10])
                .style(secondary_button_style)
                .on_press(Message::PickRunningProcess(p.exe.clone())),
            );
        }

        let add_body = column![
            row![
                text_input(
                    l.t("Executável (ex: cs2, vivaldi-bin)", "Executable (e.g. cs2, firefox)"),
                    &self.trigger_exe,
                )
                .on_input(Message::TriggerExeChanged)
                .style(input_style(accent)),
                button(text(l.t("Procurar…", "Browse…")).size(type_scale::CAPTION + 1))
                    .padding([8, 12])
                    .style(secondary_button_style)
                    .on_press(Message::BrowseTriggerExe),
                text_input(
                    l.t("Título da janela (opcional, aceita *)", "Window Title (optional, * glob)"),
                    &self.trigger_title,
                )
                .on_input(Message::TriggerTitleChanged)
                .style(input_style(accent)),
                button(
                    row![
                        icon(Icon::Plus, 13, contrast_on(accent)),
                        text(l.t("Adicionar Regra", "Add Rule"))
                            .size(type_scale::BODY)
                            .font(FONT_MEDIUM),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([8, 14])
                .style(primary_button_style(accent))
                .on_press(Message::AddTriggerRule),
            ]
            .spacing(SP_SM),
            row![
                text(l.t(
                    "Janelas abertas (clique para preencher):",
                    "Running windows (click to fill):",
                ))
                .size(type_scale::CAPTION + 1)
                .color(TEXT_SECONDARY),
                Space::with_width(Length::Fill),
                button(
                    row![
                        icon(Icon::Refresh, 12, TEXT_SECONDARY),
                        text(l.t("Atualizar Janelas", "Refresh"))
                            .size(type_scale::CAPTION),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                )
                .padding([4, 10])
                .style(secondary_button_style)
                .on_press(Message::RefreshProcesses),
            ]
            .align_y(Alignment::Center),
            scrollable(proc_row).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default(),
            )),
        ]
        .spacing(SP_SM);

        let add_card = card(
            format!(
                "{}{}",
                l.t("Vincular App ao Perfil", "Bind App to Profile"),
                self.selected_profile
                    .as_deref()
                    .map(|p| format!(" ({p})"))
                    .unwrap_or_default()
            ),
            None,
            None,
            add_body.into(),
        );

        let rules_body: Element<'_, Message> = if self.switcher_cfg.rules.is_empty() {
            empty_state(
                Icon::AppWindow,
                l.t("Nenhuma regra configurada", "No rules configured yet"),
                l.t(
                    "Clique em uma janela aberta acima, procure um executável ou digite o nome para vincular um perfil.",
                    "Click a running window above, browse an executable, or type a name to bind a profile.",
                ),
                None,
                None,
                accent,
            )
        } else {
            let total = self.switcher_cfg.rules.len();
            let mut rules_col = Column::new().spacing(6);
            for (i, r) in self.switcher_cfg.rules.iter().enumerate() {
                let title_str = if r.title.is_empty() {
                    "—".to_string()
                } else {
                    r.title.clone()
                };
                let mut rule_row = row![
                    icon(Icon::AppWindow, 14, accent),
                    text(&r.exe)
                        .size(type_scale::BODY)
                        .font(FONT_SEMIBOLD)
                        .color(TEXT_PRIMARY)
                        .width(Length::Fixed(180.0)),
                    text(title_str)
                        .size(type_scale::CAPTION + 1)
                        .color(TEXT_MUTED)
                        .width(Length::Fill),
                    badge(&r.profile, accent),
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                if i > 0 {
                    rule_row = rule_row.push(
                        button(icon(Icon::ChevronUp, 13, TEXT_SECONDARY))
                            .padding([4, 6])
                            .style(ghost_button_style)
                            .on_press(Message::MoveTriggerRuleUp(i)),
                    );
                }
                if i + 1 < total {
                    rule_row = rule_row.push(
                        button(icon(Icon::ChevronDown, 13, TEXT_SECONDARY))
                            .padding([4, 6])
                            .style(ghost_button_style)
                            .on_press(Message::MoveTriggerRuleDown(i)),
                    );
                }
                rule_row = rule_row.push(
                    button(icon(Icon::Trash, 13, RED_DANGER))
                        .padding([4, 8])
                        .style(ghost_button_style)
                        .on_press(Message::RemoveTriggerRule(i)),
                );

                rules_col = rules_col.push(
                    container(rule_row.padding([8, 12])).style(elevated_card_style),
                );
            }
            rules_col.into()
        };

        let rules_card = card(
            l.t("Regras Configuradas (ordem de prioridade)", "Configured Rules (priority order)"),
            None,
            None,
            rules_body,
        );

        column![status_card, add_card, rules_card]
            .spacing(SP_MD)
            .into()
    }
}
