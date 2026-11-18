//! Live travel gauge bar (0.00 mm - 3.50 mm) with actuation threshold and deadband markers.

use crate::theme::{with_alpha, BG_ELEVATED, BORDER_SUBTLE, GREEN_OK};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

pub struct TravelGaugeBar {
    pub live_mm: f32,
    pub peak_mm: f32,
    pub actuation_mm: f32,
    pub dead_top_mm: f32,
    pub dead_bot_mm: f32,
    pub accent: Color,
}

impl TravelGaugeBar {
    pub fn view<'a, M: 'a>(self) -> Element<'a, M> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fixed(18.0))
            .into()
    }
}

impl<M> canvas::Program<M> for TravelGaugeBar {
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

        let track = Path::rounded_rectangle(Point::ORIGIN, bounds.size(), 9.0.into());
        frame.fill(&track, BG_ELEVATED);
        frame.stroke(
            &track,
            Stroke::default().with_color(BORDER_SUBTLE).with_width(1.0),
        );

        // Top and bottom dead zone shading
        if self.dead_top_mm > 0.01 {
            let w = (bounds.width * (self.dead_top_mm / 3.5).clamp(0.0, 0.35)).max(2.0);
            let dz = Path::rounded_rectangle(
                Point::ORIGIN,
                Size::new(w, bounds.height),
                9.0.into(),
            );
            frame.fill(&dz, Color::from_rgba(1.0, 1.0, 1.0, 0.05));
        }
        if self.dead_bot_mm > 0.01 {
            let w = (bounds.width * (self.dead_bot_mm / 3.5).clamp(0.0, 0.35)).max(2.0);
            let dz = Path::rounded_rectangle(
                Point::new(bounds.width - w, 0.0),
                Size::new(w, bounds.height),
                9.0.into(),
            );
            frame.fill(&dz, Color::from_rgba(1.0, 1.0, 1.0, 0.05));
        }

        let peak_frac = (self.peak_mm / 3.5).clamp(0.0, 1.0);
        if peak_frac > 0.01 {
            let peak_w = (bounds.width * peak_frac).max(8.0);
            let peak_bar = Path::rounded_rectangle(
                Point::ORIGIN,
                Size::new(peak_w, bounds.height),
                9.0.into(),
            );
            frame.fill(&peak_bar, with_alpha(self.accent, 0.24));
        }

        let live_frac = (self.live_mm / 3.5).clamp(0.0, 1.0);
        if live_frac > 0.01 {
            let live_w = (bounds.width * live_frac).max(8.0);
            let fill_color = if self.live_mm >= self.actuation_mm {
                GREEN_OK
            } else {
                self.accent
            };
            let live_bar = Path::rounded_rectangle(
                Point::ORIGIN,
                Size::new(live_w, bounds.height),
                9.0.into(),
            );
            frame.fill(&live_bar, fill_color);
        }

        // Actuation marker tick
        let act_frac = (self.actuation_mm / 3.5).clamp(0.0, 1.0);
        let tick_x = (bounds.width * act_frac).clamp(2.0, bounds.width - 2.0);
        let tick = Path::line(
            Point::new(tick_x, 2.0),
            Point::new(tick_x, bounds.height - 2.0),
        );
        frame.stroke(
            &tick,
            Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.90))
                .with_width(2.0),
        );

        vec![frame.into_geometry()]
    }
}
