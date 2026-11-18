//! Interactive Apple-style keyboard matrix widget using iced::widget::Canvas.

use crate::theme::{
    ACCENT_BLUE, BORDER_STRONG, BORDER_SUBTLE, FONT_INTER, GREEN_OK, TEXT_PRIMARY, TEXT_SECONDARY,
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
        let max_u: f32 = layout
            .iter()
            .map(|r| r.iter().map(|k| k.width_u).sum::<f32>())
            .fold(15.0_f32, f32::max);
        let height = (self.rows as f32) * 46.0 + 24.0;
        let _ = max_u;
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
        let pad = 10.0;
        let gap = 4.0;
        let max_u: f32 = self
            .layout
            .iter()
            .map(|r| r.iter().map(|k| k.width_u).sum::<f32>())
            .fold(15.0_f32, f32::max);
        let avail_w = (bounds.width - pad * 2.0).max(200.0);
        let unit = ((avail_w - (max_u - 1.0) * gap) / max_u).clamp(26.0, 52.0);
        let row_h = ((bounds.height - pad * 2.0 - gap * (self.layout.len().saturating_sub(1) as f32))
            / (self.layout.len().max(1) as f32))
            .clamp(28.0, 48.0);

        let total_w = max_u * unit;
        let start_x = ((bounds.width - total_w) * 0.5).max(pad);

        let mut rects = Vec::new();
        for (r_idx, row) in self.layout.iter().enumerate() {
            let mut x = start_x;
            let y = pad + (r_idx as f32) * (row_h + gap);
            for k in row {
                let w = k.width_u * unit - gap;
                let idx = k.row * self.data.cols + k.col;
                rects.push((idx, Rectangle::new(Point::new(x, y), Size::new(w.max(12.0), row_h))));
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
                        let toggle = state.modifiers.control() || state.modifiers.logo() || state.modifiers.shift();
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

        // Keyboard chassis background (recessed dark aluminum tray)
        let chassis = Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 16.0.into());
        frame.fill(&chassis, Color::from_rgb(0.075, 0.075, 0.094));
        frame.stroke(
            &chassis,
            Stroke::default().with_color(BORDER_STRONG).with_width(1.0),
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

            // Outer sculpted keycap base
            let cap_path = Path::rounded_rectangle(rect.position(), rect.size(), 7.0.into());
            let base_bg = if is_selected {
                Color::from_rgba(0.039, 0.518, 1.0, 0.28)
            } else if is_hovered {
                Color::from_rgb(0.19, 0.19, 0.23)
            } else {
                Color::from_rgb(0.115, 0.115, 0.140)
            };
            frame.fill(&cap_path, base_bg);

            // Inner sculpted key top surface
            let top_rect = Rectangle::new(
                Point::new(rect.x + 2.5, rect.y + 2.0),
                Size::new((rect.width - 5.0).max(6.0), (rect.height - 6.0).max(6.0)),
            );
            let top_path = Path::rounded_rectangle(top_rect.position(), top_rect.size(), 5.0.into());
            let mut top_bg = if is_selected {
                Color::from_rgb(0.10, 0.25, 0.44)
            } else if is_hovered {
                Color::from_rgb(0.24, 0.24, 0.29)
            } else {
                Color::from_rgb(0.165, 0.165, 0.198)
            };

            let r = idx / self.data.cols;
            let c = idx % self.data.cols;

            let lighting_rgb: Option<[u8; 3]> = if self.data.mode == BoardViewMode::Lighting {
                match self.data.light_effect {
                    0 => None,
                    1 => self.data.key_colors.get(idx).copied(),
                    4 | 5 | 13 | 19 | 23 | 25 | 30 | 39 => {
                        let hue = ((c as f32 + r as f32 * 0.4) / (self.data.cols.max(1) as f32)) * 360.0;
                        let (rr, gg, bb) = hsv_to_rgb(hue, 0.85, 1.0);
                        Some([rr, gg, bb])
                    }
                    _ => Some(self.data.light_color),
                }
            } else {
                None
            };

            if let Some([lr, lg, lb]) = lighting_rgb {
                if lr > 0 || lg > 0 || lb > 0 {
                    top_bg = Color::from_rgb(
                        top_bg.r * 0.72 + (lr as f32 / 255.0) * 0.28,
                        top_bg.g * 0.72 + (lg as f32 / 255.0) * 0.28,
                        top_bg.b * 0.72 + (lb as f32 / 255.0) * 0.28,
                    );
                }
            }
            frame.fill(&top_path, top_bg);

            // Live travel fill in Performance mode (full travel = 3.50 mm)
            if self.data.mode == BoardViewMode::Performance {
                if let Some(&mm) = self.data.trip_mm.get(idx) {
                    if mm > 0.04 {
                        let frac = (mm / 3.5).clamp(0.0, 1.0);
                        let fill_h = rect.height * frac;
                        let bar = Path::rounded_rectangle(
                            Point::new(rect.x, rect.y + rect.height - fill_h),
                            Size::new(rect.width, fill_h),
                            7.0.into(),
                        );
                        frame.fill(&bar, Color::from_rgba(0.188, 0.820, 0.345, 0.38));
                    }
                }
            }

            // Per-key / active RGB strip in Lighting mode
            if let Some([lr, lg, lb]) = lighting_rgb {
                let strip = Path::rounded_rectangle(
                    Point::new(rect.x + 5.0, rect.y + rect.height - 5.5),
                    Size::new((rect.width - 10.0).max(4.0), 3.0),
                    1.5.into(),
                );
                frame.fill(&strip, Color::from_rgb8(lr, lg, lb));
            }

            let border_color = if is_selected {
                ACCENT_BLUE
            } else if is_hovered {
                BORDER_STRONG
            } else {
                BORDER_SUBTLE
            };
            frame.stroke(
                &cap_path,
                Stroke::default()
                    .with_color(border_color)
                    .with_width(if is_selected { 1.8 } else { 1.0 }),
            );

            // Key legend
            let top_label = decoded_km
                .get(r)
                .and_then(|row| row.get(c))
                .map(|&kc| keycodes::label(kc))
                .unwrap_or_else(|| format!("{r},{c}"));

            frame.fill_text(Text {
                content: top_label,
                position: Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.36),
                color: if is_selected { Color::WHITE } else { TEXT_PRIMARY },
                size: 11.0.into(),
                font: FONT_INTER,
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
                    frame.fill_text(Text {
                        content: sub,
                        position: Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.74),
                        color: if trip > 0.04 { GREEN_OK } else { TEXT_SECONDARY },
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
