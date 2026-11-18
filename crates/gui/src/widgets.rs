//! Small canvas widgets for the optional design blocks: the level history in
//! the strip head and the rotary knob with an LED ring.

use iced::mouse;
use iced::widget::canvas::{self, Fill, Frame, Geometry, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::style::*;
use crate::Message;

/// The last seconds of a strip's level as a small filled curve.
pub struct Sparkline {
    /// 0..1, oldest first.
    pub points: Vec<f32>,
    pub color: Color,
    /// How many samples fill the full width.
    pub span: usize,
}

impl canvas::Program<Message> for Sparkline {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        f.fill_rectangle(Point::ORIGIN, Size::new(w, h), well());
        if self.points.len() > 1 {
            let step = w / (self.span.max(2) - 1) as f32;
            // Newest sample at the right edge.
            let x0 = w - step * (self.points.len() - 1) as f32;
            let at = |k: usize, v: f32| Point::new(x0 + step * k as f32, h - 2.0 - v * (h - 4.0));
            let area = Path::new(|b| {
                b.move_to(Point::new(x0, h));
                for (k, v) in self.points.iter().enumerate() {
                    b.line_to(at(k, *v));
                }
                b.line_to(Point::new(w, h));
                b.close();
            });
            f.fill(
                &area,
                Fill::from(Color {
                    a: 0.22,
                    ..self.color
                }),
            );
            let line = Path::new(|b| {
                for (k, v) in self.points.iter().enumerate() {
                    if k == 0 {
                        b.move_to(at(k, *v));
                    } else {
                        b.line_to(at(k, *v));
                    }
                }
            });
            f.stroke(
                &line,
                Stroke::default().with_color(self.color).with_width(1.5),
            );
        }
        vec![f.into_geometry()]
    }
}

/// Dragging state of a knob: where it was grabbed and the value then.
#[derive(Default)]
pub struct KnobState {
    drag: Option<(f32, f32)>,
}

/// A rotary knob with a ring of LEDs. Drag up or down, or use the wheel.
pub struct Knob<'a> {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub on_change: Box<dyn Fn(f32) -> Message + 'a>,
}

/// Number of LEDs in the ring, spread over 270 degrees.
const LEDS: usize = 17;

impl Knob<'_> {
    fn frac(&self) -> f32 {
        ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// The nearest step inside the range.
    fn snap(&self, v: f32) -> f32 {
        let v = v.clamp(self.min, self.max);
        if self.step > 0.0 {
            (self.min + ((v - self.min) / self.step).round() * self.step).clamp(self.min, self.max)
        } else {
            v
        }
    }
}

impl canvas::Program<Message> for Knob<'_> {
    type State = KnobState;

    fn update(
        &self,
        state: &mut KnobState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::Event::Mouse;
        let range = self.max - self.min;
        match event {
            Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if cursor.is_over(bounds) => {
                let y = cursor.position()?.y;
                state.drag = Some((y, self.value));
                Some(canvas::Action::capture())
            }
            Mouse(mouse::Event::CursorMoved { .. }) => {
                let (y0, v0) = state.drag?;
                let y = cursor.position()?.y;
                // 160 px of travel for the whole range.
                let next = self.snap(v0 + (y0 - y) / 160.0 * range);
                (next != self.value)
                    .then(|| canvas::Action::publish((self.on_change)(next)).and_capture())
                    .or_else(|| Some(canvas::Action::capture()))
            }
            Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.drag.is_some() => {
                state.drag = None;
                Some(canvas::Action::capture())
            }
            Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => y.signum(),
                };
                let notch = if self.step > 0.0 {
                    self.step
                } else {
                    range / 50.0
                };
                let next = self.snap(self.value + dy.signum() * notch);
                (next != self.value)
                    .then(|| canvas::Action::publish((self.on_change)(next)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &KnobState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let s = bounds.width.min(bounds.height);
        let c = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let ring = s * 0.43;
        let frac = self.frac();
        let lit_color = part(Part::Knob).unwrap_or_else(accent_light);
        let off = mix(well(), border(), 0.55);
        let lit_n = (frac * (LEDS - 1) as f32 + 0.0001).floor() as usize;
        for j in 0..LEDS {
            let t = j as f32 / (LEDS - 1) as f32;
            let a = (-135.0 + 270.0 * t).to_radians();
            let at = Point::new(c.x + a.sin() * ring, c.y - a.cos() * ring);
            // The first LED is always lit, so a knob at its minimum still shows where it is.
            let lit = j <= lit_n;
            let r = (s * 0.05).max(1.5);
            if lit {
                // Soft halo, strongest at the tip of the lit arc.
                let tip = if j == lit_n { 0.35 } else { 0.14 };
                f.fill(
                    &Path::circle(at, r * 2.1),
                    Fill::from(Color {
                        a: tip,
                        ..lit_color
                    }),
                );
            }
            f.fill(
                &Path::circle(at, r),
                Fill::from(if lit {
                    mix(lit_color, Color::WHITE, if j == lit_n { 0.3 } else { 0.0 })
                } else {
                    off
                }),
            );
        }
        let body = s * 0.31;
        // Drop shadow, then the cap with a dark rim and a flat face.
        f.fill(
            &Path::circle(Point::new(c.x, c.y + 1.5), body + 1.0),
            Fill::from(Color {
                a: 0.45,
                ..Color::BLACK
            }),
        );
        f.fill(&Path::circle(c, body), Fill::from(rgb(46, 49, 55)));
        f.stroke(
            &Path::circle(c, body),
            Stroke::default()
                .with_color(rgb(12, 13, 15))
                .with_width(1.5),
        );
        f.fill(&Path::circle(c, body * 0.84), Fill::from(rgb(24, 25, 29)));
        // Light edge on top, like the console caps.
        f.stroke(
            &Path::new(|b| {
                b.arc(canvas::path::Arc {
                    center: c,
                    radius: body - 1.0,
                    start_angle: iced::Radians(-2.6),
                    end_angle: iced::Radians(-0.55),
                });
            }),
            Stroke::default()
                .with_color(rgb(110, 116, 124))
                .with_width(1.0),
        );
        // The pointer is a short line on the rim; the value sits in the middle.
        let a = (-135.0 + 270.0 * frac).to_radians();
        let tip = Point::new(c.x + a.sin() * body * 0.92, c.y - a.cos() * body * 0.92);
        let root = Point::new(c.x + a.sin() * body * 0.62, c.y - a.cos() * body * 0.62);
        f.stroke(
            &Path::line(root, tip),
            Stroke::default()
                .with_color(lit_color)
                .with_width(2.6)
                .with_line_cap(canvas::LineCap::Round),
        );
        let shown = if self.step >= 1.0 || self.max - self.min >= 100.0 {
            format!("{:.0}", self.value)
        } else {
            format!("{:.1}", self.value)
        };
        f.fill_text(canvas::Text {
            content: shown,
            position: c,
            color: rgb(214, 218, 224),
            size: iced::Pixels((s * 0.17).max(9.0)),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        vec![f.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &KnobState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some() || cursor.is_over(bounds) {
            mouse::Interaction::ResizingVertically
        } else {
            mouse::Interaction::default()
        }
    }
}
