//! Interactive 2D HSV (Saturation x Value + Hue bar) color picker widget.

use crate::theme::{BORDER_STRONG, BORDER_SUBTLE};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

const SV_W: f32 = 170.0;
const GAP: f32 = 12.0;
const HUE_W: f32 = 22.0;
const PICKER_H: f32 = 136.0;
const TOTAL_W: f32 = SV_W + GAP + HUE_W;

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let h = if delta < 1e-5 {
        0.0
    } else if (max - rf).abs() < 1e-5 {
        60.0 * (((gf - bf) / delta).rem_euclid(6.0))
    } else if (max - gf).abs() < 1e-5 {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };

    let s = if max < 1e-5 { 0.0 } else { delta / max };
    (h, s, max)
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);

    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    Sv,
    Hue,
}

#[derive(Default)]
pub struct ColorPickerState {
    dragging: Option<DragTarget>,
}

pub struct ColorPickerCanvas {
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
}

impl ColorPickerCanvas {
    pub fn view<'a, M: 'a>(
        self,
        on_change: impl Fn(f32, f32, f32) -> M + 'a,
    ) -> Element<'a, M> {
        Canvas::new(ColorPickerProgram {
            hue: self.hue,
            sat: self.sat,
            val: self.val,
            on_change: Box::new(on_change),
        })
        .width(Length::Fixed(TOTAL_W))
        .height(Length::Fixed(PICKER_H))
        .into()
    }
}

struct ColorPickerProgram<'a, M> {
    hue: f32,
    sat: f32,
    val: f32,
    on_change: Box<dyn Fn(f32, f32, f32) -> M + 'a>,
}

impl<'a, M> ColorPickerProgram<'a, M> {
    fn sv_rect() -> Rectangle {
        Rectangle::new(Point::ORIGIN, Size::new(SV_W, PICKER_H))
    }

    fn hue_rect() -> Rectangle {
        Rectangle::new(Point::new(SV_W + GAP, 0.0), Size::new(HUE_W, PICKER_H))
    }

    fn sv_from_pos(&self, pos: Point) -> (f32, f32, f32) {
        let s = (pos.x / SV_W).clamp(0.0, 1.0);
        let v = (1.0 - (pos.y / PICKER_H)).clamp(0.0, 1.0);
        (self.hue, s, v)
    }

    fn hue_from_pos(&self, pos: Point) -> (f32, f32, f32) {
        let h = ((pos.y / PICKER_H).clamp(0.0, 1.0)) * 359.9;
        (h, self.sat, self.val)
    }
}

impl<'a, M> canvas::Program<M> for ColorPickerProgram<'a, M> {
    type State = ColorPickerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<M>) {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if Self::sv_rect().contains(pos) {
                        state.dragging = Some(DragTarget::Sv);
                        let (h, s, v) = self.sv_from_pos(pos);
                        return (iced::event::Status::Captured, Some((self.on_change)(h, s, v)));
                    }
                    if Self::hue_rect().contains(pos) {
                        state.dragging = Some(DragTarget::Hue);
                        let (h, s, v) = self.hue_from_pos(pos);
                        return (iced::event::Status::Captured, Some((self.on_change)(h, s, v)));
                    }
                }
                (iced::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(target) = state.dragging {
                    if let Some(pos) = cursor.position_from(bounds.position()) {
                        let (h, s, v) = match target {
                            DragTarget::Sv => self.sv_from_pos(pos),
                            DragTarget::Hue => self.hue_from_pos(pos),
                        };
                        return (iced::event::Status::Captured, Some((self.on_change)(h, s, v)));
                    }
                }
                (iced::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.take().is_some() {
                    return (iced::event::Status::Captured, None);
                }
                (iced::event::Status::Ignored, None)
            }
            _ => (iced::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // 1. 2D Saturation x Value square
        let cols = 34usize;
        let rows = 28usize;
        let cw = SV_W / cols as f32;
        let ch = PICKER_H / rows as f32;
        for iy in 0..rows {
            let v = 1.0 - (iy as f32 + 0.5) / rows as f32;
            for ix in 0..cols {
                let s = (ix as f32 + 0.5) / cols as f32;
                let (r, g, b) = hsv_to_rgb(self.hue, s, v);
                frame.fill_rectangle(
                    Point::new(ix as f32 * cw, iy as f32 * ch),
                    Size::new(cw + 0.7, ch + 0.7),
                    Color::from_rgb8(r, g, b),
                );
            }
        }

        let sv_border = Path::rounded_rectangle(Point::ORIGIN, Size::new(SV_W, PICKER_H), 8.0.into());
        frame.stroke(
            &sv_border,
            Stroke::default().with_color(BORDER_STRONG).with_width(1.2),
        );

        // SV selector circle
        let sel_x = (self.sat.clamp(0.0, 1.0) * SV_W).clamp(5.0, SV_W - 5.0);
        let sel_y = ((1.0 - self.val.clamp(0.0, 1.0)) * PICKER_H).clamp(5.0, PICKER_H - 5.0);
        let outer = Path::circle(Point::new(sel_x, sel_y), 6.0);
        frame.stroke(
            &outer,
            Stroke::default().with_color(Color::BLACK).with_width(2.5),
        );
        frame.stroke(
            &outer,
            Stroke::default().with_color(Color::WHITE).with_width(1.5),
        );

        // 2. Vertical Rainbow Hue bar
        let hue_steps = 60usize;
        let hh = PICKER_H / hue_steps as f32;
        let hx = SV_W + GAP;
        for i in 0..hue_steps {
            let h = (i as f32 + 0.5) / hue_steps as f32 * 360.0;
            let (r, g, b) = hsv_to_rgb(h, 1.0, 1.0);
            frame.fill_rectangle(
                Point::new(hx, i as f32 * hh),
                Size::new(HUE_W, hh + 0.7),
                Color::from_rgb8(r, g, b),
            );
        }

        let hue_border =
            Path::rounded_rectangle(Point::new(hx, 0.0), Size::new(HUE_W, PICKER_H), 6.0.into());
        frame.stroke(
            &hue_border,
            Stroke::default().with_color(BORDER_SUBTLE).with_width(1.2),
        );

        // Hue thumb indicator
        let hy = ((self.hue.rem_euclid(360.0) / 360.0) * PICKER_H).clamp(3.0, PICKER_H - 3.0);
        let thumb = Path::rounded_rectangle(
            Point::new(hx - 1.5, hy - 3.0),
            Size::new(HUE_W + 3.0, 6.0),
            3.0.into(),
        );
        frame.fill(&thumb, Color::WHITE);
        frame.stroke(
            &thumb,
            Stroke::default().with_color(Color::BLACK).with_width(1.0),
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.dragging.is_some() {
            return mouse::Interaction::Crosshair;
        }
        if let Some(pos) = cursor.position_in(bounds) {
            if Self::sv_rect().contains(pos) || Self::hue_rect().contains(pos) {
                return mouse::Interaction::Pointer;
            }
        }
        mouse::Interaction::default()
    }
}
