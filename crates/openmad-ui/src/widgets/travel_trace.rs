//! Live oscilloscope travel trace canvas (0.00 - 3.50 mm over time).

use crate::theme::{with_alpha, BG_BASE, BORDER_SUBTLE, GREEN_OK, TEXT_MUTED};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

pub struct TravelTraceCanvas<'a> {
    pub samples: &'a [f32],
    pub actuation_mm: f32,
    pub dead_top_mm: f32,
    pub dead_bot_mm: f32,
    pub accent: Color,
}

impl<'a> TravelTraceCanvas<'a> {
    pub fn view<M: 'a>(self) -> Element<'a, M> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fixed(92.0))
            .into()
    }
}

impl<'a, M> canvas::Program<M> for TravelTraceCanvas<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let bg = Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 10.0.into());
        frame.fill(&bg, BG_BASE);
        frame.stroke(
            &bg,
            Stroke::default().with_color(BORDER_SUBTLE).with_width(1.0),
        );

        let pad_x = 8.0;
        let pad_y = 8.0;
        let w = (bounds.width - pad_x * 2.0).max(20.0);
        let h = (bounds.height - pad_y * 2.0).max(16.0);
        let y_for_mm = |mm: f32| -> f32 {
            let frac = (mm / 3.5).clamp(0.0, 1.0);
            pad_y + frac * h
        };

        // Top & bottom dead-band shading
        if self.dead_top_mm > 0.01 {
            let top_h = (y_for_mm(self.dead_top_mm) - pad_y).max(2.0);
            frame.fill_rectangle(
                Point::new(pad_x, pad_y),
                Size::new(w, top_h),
                Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            );
        }
        if self.dead_bot_mm > 0.01 {
            let bot_y = y_for_mm((3.5 - self.dead_bot_mm).max(0.0));
            let bot_h = ((pad_y + h) - bot_y).max(2.0);
            frame.fill_rectangle(
                Point::new(pad_x, bot_y),
                Size::new(w, bot_h),
                Color::from_rgba(1.0, 1.0, 1.0, 0.03),
            );
        }

        // Reference horizontal mm gridlines
        for mm in [1.0_f32, 2.0, 3.0] {
            let gy = y_for_mm(mm);
            let grid = Path::line(Point::new(pad_x, gy), Point::new(pad_x + w, gy));
            frame.stroke(
                &grid,
                Stroke::default()
                    .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))
                    .with_width(1.0),
            );
            frame.fill_text(Text {
                content: format!("{mm:.0}mm"),
                position: Point::new(pad_x + 4.0, gy - 11.0),
                color: with_alpha(TEXT_MUTED, 0.65),
                size: 9.0.into(),
                ..Text::default()
            });
        }

        // Actuation threshold line
        let act_y = y_for_mm(self.actuation_mm);
        let act_line = Path::line(Point::new(pad_x, act_y), Point::new(pad_x + w, act_y));
        frame.stroke(
            &act_line,
            Stroke::default()
                .with_color(with_alpha(GREEN_OK, 0.72))
                .with_width(1.3),
        );

        // Travel trace curve
        let n = self.samples.len();
        if n >= 2 {
            let step_x = w / ((n - 1).max(1) as f32);
            let line = Path::new(|b| {
                for (i, &mm) in self.samples.iter().enumerate() {
                    let pt = Point::new(pad_x + (i as f32) * step_x, y_for_mm(mm));
                    if i == 0 {
                        b.move_to(pt);
                    } else {
                        b.line_to(pt);
                    }
                }
            });
            let area = Path::new(|b| {
                b.move_to(Point::new(pad_x, pad_y));
                for (i, &mm) in self.samples.iter().enumerate() {
                    b.line_to(Point::new(pad_x + (i as f32) * step_x, y_for_mm(mm)));
                }
                b.line_to(Point::new(pad_x + ((n - 1) as f32) * step_x, pad_y));
                b.close();
            });
            frame.fill(&area, with_alpha(self.accent, 0.16));
            frame.stroke(
                &line,
                Stroke::default().with_color(self.accent).with_width(2.0),
            );
        } else {
            frame.fill_text(Text {
                content: "Waiting for keystroke telemetry…".to_string(),
                position: Point::new(bounds.width * 0.5, bounds.height * 0.5),
                color: with_alpha(TEXT_MUTED, 0.7),
                size: 11.0.into(),
                horizontal_alignment: iced::alignment::Horizontal::Center,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }

        vec![frame.into_geometry()]
    }
}
