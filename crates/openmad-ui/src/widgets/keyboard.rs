//! Interactive 3D-sculpted keyboard matrix widget using iced::widget::Canvas.

use crate::theme::{
    with_alpha, BORDER_STRONG, BORDER_SUBTLE, FONT_INTER, FONT_MEDIUM, FONT_SEMIBOLD, GREEN_OK,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::widgets::color_picker::hsv_to_rgb;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use openmad_core::keycodes;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy)]
pub struct KeyCap {
    pub row: usize,
    pub col: usize,
    pub width_u: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardViewMode {
    Keymap,
    Lighting,
    Performance,
    Advanced,
}

#[derive(Debug, Clone)]
pub enum KeyboardEvent {
    ClickKey { index: usize, toggle: bool },
}

pub fn build_layout(rows: usize, cols: usize, populated: &[bool]) -> Vec<Vec<KeyCap>> {
    if rows == 5 && cols == 15 {
        let l65 = layout_65();
        if layout_fits(&l65, populated, cols) {
            return l65;
        }
    } else if rows == 5 && cols == 14 {
        let l60 = layout_60();
        if layout_fits(&l60, populated, cols) {
            return l60;
        }
    }
    grid_layout(rows, cols, populated)
}

fn layout_fits(layout: &[Vec<KeyCap>], populated: &[bool], cols: usize) -> bool {
    if populated.is_empty() {
        return true;
    }
    layout.iter().flatten().all(|k| {
        let idx = k.row * cols + k.col;
        populated.get(idx).copied().unwrap_or(true)
    })
}

fn grid_layout(rows: usize, cols: usize, populated: &[bool]) -> Vec<Vec<KeyCap>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .filter(|&c| {
                    let idx = r * cols + c;
                    populated.is_empty() || populated.get(idx).copied().unwrap_or(true)
                })
                .map(|c| KeyCap {
                    row: r,
                    col: c,
                    width_u: 1.0,
                })
                .collect()
        })
        .collect()
}

fn layout_65() -> Vec<Vec<KeyCap>> {
    vec![
        (0..13)
            .map(|c| KeyCap { row: 0, col: c, width_u: 1.0 })
            .chain([
                KeyCap { row: 0, col: 13, width_u: 2.0 },
                KeyCap { row: 0, col: 14, width_u: 1.0 },
            ])
            .collect(),
        [KeyCap { row: 1, col: 0, width_u: 1.5 }]
            .into_iter()
            .chain((1..13).map(|c| KeyCap { row: 1, col: c, width_u: 1.0 }))
            .chain([
                KeyCap { row: 1, col: 13, width_u: 1.5 },
                KeyCap { row: 1, col: 14, width_u: 1.0 },
            ])
            .collect(),
        [KeyCap { row: 2, col: 0, width_u: 1.75 }]
            .into_iter()
            .chain((1..12).map(|c| KeyCap { row: 2, col: c, width_u: 1.0 }))
            .chain([
                KeyCap { row: 2, col: 13, width_u: 2.25 },
                KeyCap { row: 2, col: 14, width_u: 1.0 },
            ])
            .collect(),
        [KeyCap { row: 3, col: 0, width_u: 2.25 }]
            .into_iter()
            .chain((2..12).map(|c| KeyCap { row: 3, col: c, width_u: 1.0 }))
            .chain([
                KeyCap { row: 3, col: 12, width_u: 1.75 },
                KeyCap { row: 3, col: 13, width_u: 1.0 },
                KeyCap { row: 3, col: 14, width_u: 1.0 },
            ])
            .collect(),
        vec![
            KeyCap { row: 4, col: 0, width_u: 1.25 },
            KeyCap { row: 4, col: 1, width_u: 1.25 },
            KeyCap { row: 4, col: 2, width_u: 1.25 },
            KeyCap { row: 4, col: 6, width_u: 6.25 },
            KeyCap { row: 4, col: 9, width_u: 1.0 },
            KeyCap { row: 4, col: 10, width_u: 1.0 },
            KeyCap { row: 4, col: 11, width_u: 1.0 },
            KeyCap { row: 4, col: 12, width_u: 1.0 },
            KeyCap { row: 4, col: 13, width_u: 1.0 },
            KeyCap { row: 4, col: 14, width_u: 1.0 },
        ],
    ]
}

fn layout_60() -> Vec<Vec<KeyCap>> {
    vec![
        (0..13)
            .map(|c| KeyCap { row: 0, col: c, width_u: 1.0 })
            .chain([KeyCap { row: 0, col: 13, width_u: 2.0 }])
            .collect(),
        [KeyCap { row: 1, col: 0, width_u: 1.5 }]
            .into_iter()
            .chain((1..13).map(|c| KeyCap { row: 1, col: c, width_u: 1.0 }))
            .chain([KeyCap { row: 1, col: 13, width_u: 1.5 }])
            .collect(),
        [KeyCap { row: 2, col: 0, width_u: 1.75 }]
            .into_iter()
            .chain((1..12).map(|c| KeyCap { row: 2, col: c, width_u: 1.0 }))
            .chain([KeyCap { row: 2, col: 13, width_u: 2.25 }])
            .collect(),
        [KeyCap { row: 3, col: 0, width_u: 2.25 }]
            .into_iter()
            .chain((2..12).map(|c| KeyCap { row: 3, col: c, width_u: 1.0 }))
            .chain([KeyCap { row: 3, col: 13, width_u: 2.75 }])
            .collect(),
        vec![
            KeyCap { row: 4, col: 0, width_u: 1.25 },
            KeyCap { row: 4, col: 1, width_u: 1.25 },
            KeyCap { row: 4, col: 2, width_u: 1.25 },
            KeyCap { row: 4, col: 6, width_u: 6.25 },
            KeyCap { row: 4, col: 10, width_u: 1.25 },
            KeyCap { row: 4, col: 11, width_u: 1.25 },
            KeyCap { row: 4, col: 12, width_u: 1.25 },
            KeyCap { row: 4, col: 13, width_u: 1.25 },
        ],
    ]
}

fn actuation_heatmap_color(apc: f32) -> Color {
    if apc <= 0.80 {
        // Ultra-sensitive: cyan-blue
        Color::from_rgb(0.18, 0.68, 0.98)
    } else if apc <= 1.60 {
        // Gaming: emerald
        Color::from_rgb(0.24, 0.80, 0.48)
    } else if apc <= 2.30 {
        // Balanced: muted slate
        Color::from_rgb(0.55, 0.58, 0.66)
    } else {
        // Deep: amber-orange
        Color::from_rgb(0.96, 0.62, 0.16)
    }
}

pub struct KeyboardCanvas<'a> {
    pub rows: usize,
    pub cols: usize,
    pub populated: &'a [bool],
    pub selected: &'a BTreeSet<usize>,
    pub mode: BoardViewMode,
    pub layer: usize,
    pub keymap: &'a [u8],
    pub actuation: &'a [f32],
    pub trip_mm: &'a [f32],
    pub key_colors: &'a [[u8; 3]],
    pub light_effect: u8,
    pub light_color: [u8; 3],
    pub accent: Color,
}

#[derive(Default)]
pub struct KeyboardCanvasState {
    hovered: Option<usize>,
    modifiers: iced::keyboard::Modifiers,
}

impl<'a> KeyboardCanvas<'a> {
    pub fn view<M: 'a>(
        self,
        on_event: impl Fn(KeyboardEvent) -> M + 'a,
    ) -> Element<'a, M> {
        let layout = build_layout(self.rows, self.cols, self.populated);
        let height = ((self.rows as f32) * 42.0 + 22.0).min(242.0);
        Canvas::new(KeyboardProgram {
            data: self,
            layout,
            on_event: Box::new(on_event),
        })
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .into()
    }
}

struct KeyboardProgram<'a, M> {
    data: KeyboardCanvas<'a>,
    layout: Vec<Vec<KeyCap>>,
    on_event: Box<dyn Fn(KeyboardEvent) -> M + 'a>,
}

impl<'a, M> KeyboardProgram<'a, M> {
    fn key_rects(&self, bounds: Size) -> Vec<(usize, Rectangle)> {
        let pad = 12.0;
        let gap = 5.0;
        let max_u: f32 = self
            .layout
            .iter()
            .map(|r| r.iter().map(|k| k.width_u).sum::<f32>())
            .fold(15.0_f32, f32::max);
        let avail_w = (bounds.width - pad * 2.0).max(200.0);
        let unit = ((avail_w - (max_u - 1.0) * gap) / max_u).clamp(24.0, 52.0);
        let row_h = ((bounds.height - pad * 2.0 - gap * (self.layout.len().saturating_sub(1) as f32))
            / (self.layout.len().max(1) as f32))
            .clamp(26.0, 44.0);

        let total_w = max_u * unit;
        let start_x = ((bounds.width - total_w) * 0.5).max(pad);
        let total_h = (self.layout.len() as f32) * row_h
            + (self.layout.len().saturating_sub(1) as f32) * gap;
        let start_y = ((bounds.height - total_h) * 0.5).max(pad);

        let mut rects = Vec::new();
        for (r_idx, row) in self.layout.iter().enumerate() {
            let mut x = start_x;
            let y = start_y + (r_idx as f32) * (row_h + gap);
            for k in row {
                let w = k.width_u * unit - gap;
                let idx = k.row * self.data.cols + k.col;
                rects.push((
                    idx,
                    Rectangle::new(Point::new(x, y), Size::new(w.max(12.0), row_h)),
                ));
                x += k.width_u * unit;
            }
        }
        rects
    }
}

impl<'a, M> canvas::Program<M> for KeyboardProgram<'a, M> {
    type State = KeyboardCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<M>) {
        match event {
            canvas::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(mods)) => {
                state.modifiers = mods;
                (iced::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let next_hover = cursor.position_in(bounds).and_then(|pos| {
                    self.key_rects(bounds.size())
                        .into_iter()
                        .find(|(_, r)| r.contains(pos))
                        .map(|(i, _)| i)
                });
                if next_hover != state.hovered {
                    state.hovered = next_hover;
                    return (iced::event::Status::Captured, None);
                }
                (iced::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if let Some((idx, _)) = self
                        .key_rects(bounds.size())
                        .into_iter()
                        .find(|(_, r)| r.contains(pos))
                    {
                        let toggle = state.modifiers.control()
                            || state.modifiers.logo()
                            || state.modifiers.shift();
                        return (
                            iced::event::Status::Captured,
                            Some((self.on_event)(KeyboardEvent::ClickKey { index: idx, toggle })),
                        );
                    }
                }
                (iced::event::Status::Ignored, None)
            }
            _ => (iced::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Recessed CNC aluminum chassis tray
        let chassis = Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 14.0.into());
        frame.fill(&chassis, Color::from_rgb(0.063, 0.063, 0.078));
        frame.stroke(
            &chassis,
            Stroke::default().with_color(BORDER_SUBTLE).with_width(1.0),
        );

        let rects = self.key_rects(bounds.size());
        let decoded_km = if !self.data.keymap.is_empty() {
            keycodes::decode_keymap(
                self.data.keymap,
                self.data.layer,
                self.data.rows,
                self.data.cols,
            )
        } else {
            Vec::new()
        };

        for (idx, rect) in rects {
            let is_selected = self.data.selected.contains(&idx);
            let is_hovered = state.hovered == Some(idx);

            let r = idx / self.data.cols;
            let c = idx % self.data.cols;

            let lighting_rgb: Option<[u8; 3]> = if self.data.mode == BoardViewMode::Lighting {
                match self.data.light_effect {
                    0 => None,
                    1 => self.data.key_colors.get(idx).copied(),
                    4 | 5 | 13 | 19 | 23 | 25 | 30 | 39 => {
                        let hue =
                            ((c as f32 + r as f32 * 0.4) / (self.data.cols.max(1) as f32)) * 360.0;
                        let (rr, gg, bb) = hsv_to_rgb(hue, 0.85, 1.0);
                        Some([rr, gg, bb])
                    }
                    _ => Some(self.data.light_color),
                }
            } else {
                None
            };

            // Layer 1: Keycap 3D skirt / bottom bezel (darker)
            let cap_path = Path::rounded_rectangle(rect.position(), rect.size(), 7.0.into());
            let mut skirt_color = if is_selected {
                with_alpha(self.data.accent, 0.45)
            } else if is_hovered {
                Color::from_rgb(0.145, 0.145, 0.175)
            } else {
                Color::from_rgb(0.090, 0.090, 0.112)
            };
            if let Some([lr, lg, lb]) = lighting_rgb {
                if lr > 0 || lg > 0 || lb > 0 {
                    skirt_color = Color::from_rgb(
                        skirt_color.r * 0.55 + (lr as f32 / 255.0) * 0.45,
                        skirt_color.g * 0.55 + (lg as f32 / 255.0) * 0.45,
                        skirt_color.b * 0.55 + (lb as f32 / 255.0) * 0.45,
                    );
                }
            }
            frame.fill(&cap_path, skirt_color);

            // Layer 2: Sculpted keycap top dish (offset up by 1.5px, leaving 3.5px bottom skirt)
            let top_rect = Rectangle::new(
                Point::new(rect.x + 2.5, rect.y + 1.5),
                Size::new((rect.width - 5.0).max(6.0), (rect.height - 5.5).max(6.0)),
            );
            let top_path = Path::rounded_rectangle(top_rect.position(), top_rect.size(), 5.5.into());
            let mut top_bg = if is_selected {
                Color::from_rgb(
                    0.12 + self.data.accent.r * 0.24,
                    0.12 + self.data.accent.g * 0.24,
                    0.14 + self.data.accent.b * 0.24,
                )
            } else if is_hovered {
                Color::from_rgb(0.22, 0.22, 0.27)
            } else {
                Color::from_rgb(0.155, 0.155, 0.190)
            };

            // Lighting glow tint across keycap surface
            if let Some([lr, lg, lb]) = lighting_rgb {
                if lr > 0 || lg > 0 || lb > 0 {
                    top_bg = Color::from_rgb(
                        top_bg.r * 0.68 + (lr as f32 / 255.0) * 0.32,
                        top_bg.g * 0.68 + (lg as f32 / 255.0) * 0.32,
                        top_bg.b * 0.68 + (lb as f32 / 255.0) * 0.32,
                    );
                }
            }

            // Performance heatmap subtle tint
            if self.data.mode == BoardViewMode::Performance {
                if let Some(&apc) = self.data.actuation.get(idx) {
                    let heat = actuation_heatmap_color(apc);
                    top_bg = Color::from_rgb(
                        top_bg.r * 0.84 + heat.r * 0.16,
                        top_bg.g * 0.84 + heat.g * 0.16,
                        top_bg.b * 0.84 + heat.b * 0.16,
                    );
                }
            }

            frame.fill(&top_path, top_bg);

            // Subtle top-edge specular highlight for 3D keycap feel
            let highlight = Path::line(
                Point::new(top_rect.x + 4.0, top_rect.y + 0.8),
                Point::new(top_rect.x + top_rect.width - 4.0, top_rect.y + 0.8),
            );
            frame.stroke(
                &highlight,
                Stroke::default()
                    .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.12))
                    .with_width(1.0),
            );

            // Live travel fill in Performance mode (full travel = 3.50 mm)
            if self.data.mode == BoardViewMode::Performance {
                if let Some(&mm) = self.data.trip_mm.get(idx) {
                    if mm > 0.04 {
                        let frac = (mm / 3.5).clamp(0.0, 1.0);
                        let fill_h = (top_rect.height * frac).max(4.0);
                        let bar = Path::rounded_rectangle(
                            Point::new(top_rect.x, top_rect.y + top_rect.height - fill_h),
                            Size::new(top_rect.width, fill_h),
                            5.0.into(),
                        );
                        frame.fill(&bar, Color::from_rgba(0.188, 0.820, 0.345, 0.45));
                    }
                }
            }

            // Outer selection / hover stroke
            let border_color = if is_selected {
                self.data.accent
            } else if is_hovered {
                BORDER_STRONG
            } else {
                BORDER_SUBTLE
            };
            frame.stroke(
                &cap_path,
                Stroke::default()
                    .with_color(border_color)
                    .with_width(if is_selected { 2.0 } else { 1.0 }),
            );

            // Key legend
            let top_label = decoded_km
                .get(r)
                .and_then(|row| row.get(c))
                .map(|&kc| keycodes::label(kc))
                .unwrap_or_else(|| format!("{r},{c}"));

            let label_y = if self.data.mode == BoardViewMode::Performance {
                top_rect.y + top_rect.height * 0.35
            } else {
                top_rect.y + top_rect.height * 0.50
            };

            frame.fill_text(Text {
                content: top_label,
                position: Point::new(top_rect.x + top_rect.width * 0.5, label_y),
                color: if is_selected { Color::WHITE } else { TEXT_PRIMARY },
                size: 11.0.into(),
                font: if is_selected { FONT_SEMIBOLD } else { FONT_MEDIUM },
                horizontal_alignment: iced::alignment::Horizontal::Center,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Text::default()
            });

            if self.data.mode == BoardViewMode::Performance {
                if let Some(&apc) = self.data.actuation.get(idx) {
                    let trip = self.data.trip_mm.get(idx).copied().unwrap_or(0.0);
                    let sub = if trip > 0.04 {
                        format!("{trip:.2}")
                    } else {
                        format!("{apc:.2}")
                    };
                    let sub_color = if trip > 0.04 {
                        GREEN_OK
                    } else {
                        actuation_heatmap_color(apc)
                    };
                    frame.fill_text(Text {
                        content: sub,
                        position: Point::new(
                            top_rect.x + top_rect.width * 0.5,
                            top_rect.y + top_rect.height * 0.74,
                        ),
                        color: if sub_color == Color::from_rgb(0.55, 0.58, 0.66) {
                            TEXT_SECONDARY
                        } else {
                            sub_color
                        },
                        size: 9.0.into(),
                        font: FONT_INTER,
                        horizontal_alignment: iced::alignment::Horizontal::Center,
                        vertical_alignment: iced::alignment::Vertical::Center,
                        ..Text::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.hovered.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
