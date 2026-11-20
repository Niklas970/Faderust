//! Marking up the desk inside the app: a picture of the main window, areas
//! drawn on it with a note each, then one press puts the picture (with the
//! numbered areas) and the list on the clipboard.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::widget::image;
use iced::{Color, Element, Length, Pixels, Point, Rectangle, Renderer, Size, Theme};

use crate::{Edit, Message};

/// One marked area, in fractions of the picture (0..1).
#[derive(Clone)]
pub struct Mark {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub note: String,
}

pub struct Markup {
    pub handle: image::Handle,
    rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
    /// Display scale of the window the picture was taken in.
    scale: f32,
    pub marks: Vec<Mark>,
    pub version: String,
}

const PINK: [u8; 3] = [255, 59, 129];

impl Markup {
    pub fn new(rgba: &[u8], w: u32, h: u32, scale: f32, version: String) -> Self {
        Self {
            handle: image::Handle::from_rgba(w, h, rgba.to_vec()),
            rgba: rgba.to_vec(),
            w,
            h,
            scale: scale.max(1.0),
            marks: Vec::new(),
            version,
        }
    }

    /// Size of the window in logical pixels.
    fn logical(&self) -> (u32, u32) {
        (
            (self.w as f32 / self.scale).round() as u32,
            (self.h as f32 / self.scale).round() as u32,
        )
    }

    /// The list of marks as text, to paste next to the picture.
    pub fn list(&self) -> String {
        let (lw, lh) = self.logical();
        let mut out = format!(
            "Faderust Design-Markierungen (v{}, Fenster {lw}×{lh} px)",
            self.version
        );
        for (i, m) in self.marks.iter().enumerate() {
            out.push_str(&format!(
                "\n{}. Bereich x={} y={} b={} h={}",
                i + 1,
                (m.x0 * lw as f32).round(),
                (m.y0 * lh as f32).round(),
                ((m.x1 - m.x0) * lw as f32).round(),
                ((m.y1 - m.y0) * lh as f32).round()
            ));
            if !m.note.trim().is_empty() {
                out.push_str(&format!(": {}", m.note.trim()));
            }
        }
        out
    }

    /// The picture with the marks burnt in: a frame, a light tint and the
    /// number on a badge in the corner.
    pub fn annotated(&self) -> Vec<u8> {
        let mut px = self.rgba.clone();
        let (w, h) = (self.w as i32, self.h as i32);
        let k = self.scale.round().max(1.0) as i32;
        for (i, m) in self.marks.iter().enumerate() {
            let x0 = (m.x0 * w as f32).round() as i32;
            let y0 = (m.y0 * h as f32).round() as i32;
            let x1 = ((m.x1 * w as f32).round() as i32).max(x0 + 1);
            let y1 = ((m.y1 * h as f32).round() as i32).max(y0 + 1);
            fill(&mut px, w, h, (x0, y0, x1, y1), PINK, 0.15);
            let t = 2 * k;
            for r in [
                (x0, y0, x1, y0 + t),
                (x0, y1 - t, x1, y1),
                (x0, y0, x0 + t, y1),
                (x1 - t, y0, x1, y1),
            ] {
                fill(&mut px, w, h, r, PINK, 1.0);
            }
            // Badge above the top left corner, or inside when there is no room.
            let digits: Vec<usize> = (i + 1)
                .to_string()
                .bytes()
                .map(|b| (b - b'0') as usize)
                .collect();
            let cell = 2 * k;
            let (bw, bh) = (digits.len() as i32 * 6 * cell + 2 * cell, 9 * cell);
            let by = if y0 - bh >= 0 { y0 - bh } else { y0 };
            fill(&mut px, w, h, (x0, by, x0 + bw, by + bh), PINK, 1.0);
            for (d, digit) in digits.iter().enumerate() {
                let ox = x0 + cell + d as i32 * 6 * cell;
                for (row, bits) in DIGITS[*digit].iter().enumerate() {
                    for col in 0..5 {
                        if bits >> (4 - col) & 1 == 1 {
                            let (cx, cy) = (ox + col * cell, by + cell + row as i32 * cell);
                            fill(&mut px, w, h, (cx, cy, cx + cell, cy + cell), [255; 3], 1.0);
                        }
                    }
                }
            }
        }
        px
    }
}

/// `rect` = (x0, y0, x1, y1) in pixels, clipped to the picture.
fn fill(px: &mut [u8], w: i32, h: i32, rect: (i32, i32, i32, i32), c: [u8; 3], a: f32) {
    let (x0, y0) = (rect.0.clamp(0, w), rect.1.clamp(0, h));
    let (x1, y1) = (rect.2.clamp(0, w), rect.3.clamp(0, h));
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * w + x) * 4) as usize;
            for (k, v) in c.iter().enumerate() {
                px[i + k] = (f32::from(px[i + k]) * (1.0 - a) + f32::from(*v) * a).round() as u8;
            }
            px[i + 3] = 255;
        }
    }
}

/// 5 × 7 digits, one row per byte (the low five bits).
const DIGITS: [[u8; 7]; 10] = [
    [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
    [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
    [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
    [0x1f, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0e],
    [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
    [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e],
    [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e],
    [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
    [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
    [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c],
];

/// Where the picture sits in `bounds`: as large as fits, centred.
fn fit(bounds: Rectangle, w: u32, h: u32) -> Rectangle {
    let s = (bounds.width / w as f32).min(bounds.height / h as f32);
    let (iw, ih) = (w as f32 * s, h as f32 * s);
    Rectangle::new(
        Point::new((bounds.width - iw) / 2.0, (bounds.height - ih) / 2.0),
        Size::new(iw, ih),
    )
}

/// The picture with the marks; dragging draws a new area, a plain click a
/// small one.
pub struct Surface<'a>(pub &'a Markup);

#[derive(Default)]
pub struct Drag(Option<(Point, Point)>);

impl canvas::Program<Message> for Surface<'_> {
    type State = Drag;

    fn update(
        &self,
        state: &mut Drag,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::Event::Mouse;
        let at = fit(bounds, self.0.w, self.0.h);
        let clamp = |p: Point| {
            Point::new(
                p.x.clamp(at.x, at.x + at.width),
                p.y.clamp(at.y, at.y + at.height),
            )
        };
        match event {
            Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_in(bounds)?;
                let on = Rectangle::new(at.position(), at.size());
                if !on.contains(p) {
                    return None;
                }
                state.0 = Some((p, p));
                Some(canvas::Action::request_redraw().and_capture())
            }
            Mouse(mouse::Event::CursorMoved { .. }) if state.0.is_some() => {
                let p = cursor
                    .position()
                    .map(|c| Point::new(c.x - bounds.x, c.y - bounds.y))?;
                if let Some((a, _)) = state.0 {
                    state.0 = Some((a, clamp(p)));
                }
                Some(canvas::Action::request_redraw())
            }
            Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.0.is_some() => {
                let (a, b) = state.0.take()?;
                let (mut x0, mut x1) = (a.x.min(b.x), a.x.max(b.x));
                let (mut y0, mut y1) = (a.y.min(b.y), a.y.max(b.y));
                if x1 - x0 < 6.0 && y1 - y0 < 6.0 {
                    // A click: a small area around it.
                    (x0, x1) = (a.x - 14.0, a.x + 14.0);
                    (y0, y1) = (a.y - 14.0, a.y + 14.0);
                }
                let nx = |x: f32| ((x - at.x) / at.width).clamp(0.0, 1.0);
                let ny = |y: f32| ((y - at.y) / at.height).clamp(0.0, 1.0);
                Some(
                    canvas::Action::publish(Message::Edit(Edit::MarkupAdd([
                        nx(x0),
                        ny(y0),
                        nx(x1),
                        ny(y1),
                    ])))
                    .and_capture(),
                )
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Drag,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let at = fit(bounds, self.0.w, self.0.h);
        let pink = Color::from_rgb8(PINK[0], PINK[1], PINK[2]);
        let rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
            (
                Point::new(at.x + x0 * at.width, at.y + y0 * at.height),
                Size::new((x1 - x0) * at.width, (y1 - y0) * at.height),
            )
        };
        for (i, m) in self.0.marks.iter().enumerate() {
            let (p, s) = rect(m.x0, m.y0, m.x1, m.y1);
            f.fill_rectangle(p, s, Color { a: 0.15, ..pink });
            f.stroke(
                &Path::rectangle(p, s),
                Stroke::default().with_color(pink).with_width(2.0),
            );
            f.fill_rectangle(
                Point::new(p.x, (p.y - 18.0).max(at.y)),
                Size::new(8.0 * (i + 1).to_string().len() as f32 + 10.0, 18.0),
                pink,
            );
            f.fill_text(Text {
                content: (i + 1).to_string(),
                position: Point::new(p.x + 5.0, (p.y - 18.0).max(at.y) + 2.0),
                color: Color::WHITE,
                size: Pixels(13.0),
                font: crate::style::BOLD,
                ..Text::default()
            });
        }
        if let Some((a, b)) = state.0 {
            let p = Point::new(a.x.min(b.x), a.y.min(b.y));
            let s = Size::new((a.x - b.x).abs(), (a.y - b.y).abs());
            f.fill_rectangle(p, s, Color { a: 0.15, ..pink });
            f.stroke(
                &Path::rectangle(p, s),
                Stroke::default().with_color(pink).with_width(2.0),
            );
        }
        vec![f.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Drag,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

/// The canvas as an element.
pub fn surface(m: &Markup) -> Element<'_, Message> {
    // The picture is a widget of its own: images in a canvas are drawn above
    // its shapes, so the areas would be hidden.
    iced::widget::stack![
        image(m.handle.clone())
            .content_fit(iced::ContentFit::Contain)
            .width(Length::Fill)
            .height(Length::Fill),
        Canvas::new(Surface(m))
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markup() -> Markup {
        let mut m = Markup::new(&vec![0u8; 100 * 60 * 4], 100, 60, 2.0, "0.8.7".into());
        m.marks.push(Mark {
            x0: 0.2,
            y0: 0.5,
            x1: 0.6,
            y1: 0.9,
            note: "Zahlen größer".into(),
        });
        m
    }

    #[test]
    fn list_names_area_and_note_in_window_pixels() {
        let l = markup().list();
        // 100 × 60 physical at scale 2 = 50 × 30 logical.
        assert!(l.contains("Fenster 50×30 px"), "{l}");
        assert!(
            l.contains("1. Bereich x=10 y=15 b=20 h=12: Zahlen größer"),
            "{l}"
        );
    }

    #[test]
    fn annotated_picture_has_the_frame_and_stays_the_same_size() {
        let m = markup();
        let px = m.annotated();
        assert_eq!(px.len(), 100 * 60 * 4);
        // Frame pixel of the area (x = 20, y = 45).
        let i = (45 * 100 + 20) * 4;
        assert_eq!(&px[i..i + 3], &PINK);
        // Far away from the marks nothing changed.
        assert_eq!(&px[0..3], &[0, 0, 0]);
    }

    #[test]
    fn every_digit_has_pixels() {
        assert!(DIGITS.iter().all(|d| d.iter().any(|r| *r != 0)));
    }

    #[test]
    fn picture_fits_and_centres() {
        let r = fit(
            Rectangle::new(Point::ORIGIN, Size::new(200.0, 100.0)),
            100,
            100,
        );
        assert_eq!((r.x, r.y, r.width, r.height), (50.0, 0.0, 100.0, 100.0));
    }
}
