//! The dialogs, laid out like the web UI's: panel, 18 px title, small
//! uppercase section headings, grey hints, buttons in the footer.

use crate::i18n::{text, tr};
use crate::style::{check, input as text_input, look_text, pick as pick_list, scroll, Look};
use iced::alignment::Vertical;
use iced::widget::{
    button, column, container, mouse_area, opaque, row, slider, space, vertical_slider,
};
use iced::{Background, Border, Color, Element, Length};

use crate::style::*;
use faderust_core::names::device_label;

use crate::{App, Dialog, Edit, Message, Target, HIDE_AFTER, RELEASE};

/// Height of the equaliser faders in px.
const EQ_H: f32 = 150.0;

fn edit(e: Edit) -> Message {
    Message::Edit(e)
}

/// An effect setting: a slider, or a rotary knob when the design asks for it.
fn ctl<'a>(
    app: &App,
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    step: f32,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    if app.prefs.knobs {
        iced::widget::Canvas::new(crate::widgets::Knob {
            value,
            min: *range.start(),
            max: *range.end(),
            step,
            on_change: Box::new(on_change),
        })
        .width(64)
        .height(64)
        .into()
    } else {
        slider(range, value, on_change).step(step).into()
    }
}

fn close() -> Message {
    Message::Open(None)
}

fn h3(label: impl Into<String>) -> Element<'static, Message> {
    column![
        space().height(8),
        text(tr(&label.into()).to_uppercase())
            .size(12)
            .font(BOLD)
            .color(muted_color())
    ]
    .into()
}

/// A titled group in a card, so a wide dialog reads as a grid of blocks.
fn card<'a>(
    title: impl Into<String>,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(tr(&title.into()).to_uppercase())
                .size(12)
                .font(BOLD)
                .color(muted_color()),
            content.into()
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(Background::Color(card_bg())),
        border: Border {
            color: border(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    })
    .into()
}

/// One column of cards; the portions set how wide it is next to the others.
fn cards<'a>(portion: u16, items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    column(items)
        .spacing(12)
        .width(Length::FillPortion(portion))
        .into()
}

/// One option as a switch with a title and a short explanation: easier to
/// read than a bare check box.
fn opt<'a>(
    title: &'static str,
    desc: &'static str,
    on: bool,
    msg: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        column![
            text(title).size(13).font(BOLD).color(text_color()),
            text(desc).size(12).color(muted_color()),
        ]
        .spacing(2)
        .width(Length::Fill),
        iced::widget::toggler(on)
            .on_toggle(msg)
            .size(22)
            .style(|_, status| {
                use iced::widget::toggler::Status;
                let (on, hot) = match status {
                    Status::Active { is_toggled } => (is_toggled, false),
                    Status::Hovered { is_toggled } => (is_toggled, true),
                    Status::Disabled { is_toggled } => (is_toggled, false),
                };
                iced::widget::toggler::Style {
                    background: Background::Color(if on { accent() } else { well_light() }),
                    background_border_width: 1.0,
                    background_border_color: if on || hot { accent() } else { border() },
                    foreground: Background::Color(if on { on_accent() } else { muted_color() }),
                    foreground_border_width: 0.0,
                    foreground_border_color: Color::TRANSPARENT,
                    text_color: Some(text_color()),
                    border_radius: None,
                    padding_ratio: 0.2,
                }
            }),
    ]
    .spacing(14)
    .align_y(Vertical::Center)
    .into()
}

/// Options of one card, one under the other.
fn opts<'a>(items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    column(items).spacing(12).into()
}

fn hint(label: impl Into<String>) -> Element<'static, Message> {
    text(label.into()).size(12).color(muted_color()).into()
}

fn btn(label: impl Into<String>, msg: Message, primary: bool) -> Element<'static, Message> {
    let look = if primary {
        Look::Primary
    } else {
        Look::Secondary
    };
    button(text(label.into()).size(13).color(look_text(look)))
        .padding([6, 14])
        .on_press(msg)
        .style(flat(look))
        .into()
}

/// A row of buttons of which one is on, like the web UI's segmented control.
fn segmented(items: Vec<(String, bool, Message)>) -> Element<'static, Message> {
    row(items.into_iter().map(|(label, on, msg)| {
        let look = if on { Look::Primary } else { Look::Secondary };
        button(text(label).size(13).color(look_text(look)))
            .padding([6, 14])
            .on_press(msg)
            .style(flat(look))
            .into()
    }))
    .spacing(1)
    .into()
}

thread_local! {
    /// True while the dialog is drawn in a window of its own.
    static STANDALONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Draws the next dialog for a window of its own instead of an overlay.
pub fn set_standalone(on: bool) {
    STANDALONE.with(|s| s.set(on));
}

fn frame<'a>(
    title: String,
    width: f32,
    tall: bool,
    body: Element<'a, Message>,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    // In the settings window the bar above already names the dialog and has
    // the close button; only dialogs without a button there keep a title.
    let standalone = STANDALONE.with(|s| s.get());
    let in_nav = ["Presets", "Windows", "Geräte", "Design", "Soundboard"].contains(&title.as_str());
    let head: Element<'a, Message> = if standalone {
        if in_nav {
            space().height(0).into()
        } else {
            text(title).size(18).color(text_color()).into()
        }
    } else {
        row![
            text(title).size(18).color(text_color()),
            space().width(Length::Fill),
            button(text("✕").size(12).color(text_color()).center())
                .width(28)
                .height(28)
                .padding(0)
                .on_press(close())
                .style(flat(Look::Secondary)),
        ]
        .align_y(Vertical::Center)
        .into()
    };
    let card = container(
        column![
            head,
            scroll(container(body).padding(iced::Padding::default().right(28))).height(
                if STANDALONE.with(|s| s.get()) {
                    Length::Fill
                } else if tall {
                    Length::Fixed(680.0)
                } else {
                    Length::Shrink
                }
            ),
            container(footer)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(12),
    )
    .padding(20)
    .width(Length::Fill)
    .max_width(if STANDALONE.with(|s| s.get()) {
        f32::MAX
    } else {
        width
    })
    .height(if STANDALONE.with(|s| s.get()) {
        Length::Fill
    } else {
        Length::Shrink
    })
    .max_height(if STANDALONE.with(|s| s.get()) {
        f32::MAX
    } else if tall {
        780.0
    } else {
        760.0
    })
    .style(|_| container::Style {
        background: Some(Background::Color(panel())),
        border: Border {
            color: border(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    });
    if STANDALONE.with(|s| s.get()) {
        return container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
    // The backdrop swallows clicks; a click on it closes the dialog.
    opaque(
        mouse_area(
            container(opaque(card))
                .center(Length::Fill)
                .padding(24)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.5,
                        ..Color::BLACK
                    })),
                    ..container::Style::default()
                }),
        )
        .on_press(close()),
    )
}

fn no_footer() -> Element<'static, Message> {
    space().width(0).height(0).into()
}

pub fn view(app: &App, d: Dialog) -> Element<'_, Message> {
    match d {
        Dialog::Fx(i) => fx(app, i),
        Dialog::Apps => apps(app),
        Dialog::Presets => presets(app),
        Dialog::Windows => windows(app),
        Dialog::Design => design(app),
        Dialog::Settings => settings(app),
        Dialog::Log => log(app),
        Dialog::Selftest => selftest(app),
        Dialog::Soundboard => soundboard(app),
        Dialog::Markup => markup(app),
    }
}

/// The marking window: the picture of the desk, and the list of areas.
fn markup(app: &App) -> Element<'_, Message> {
    let Some(mk) = &app.markup else {
        return hint("Kein Bild: „Design markieren“ unter Geräte → Erweitert macht eins.");
    };
    let list = mk
        .marks
        .iter()
        .enumerate()
        .fold(column![].spacing(8), |col, (i, m)| {
            col.push(
                row![
                    container(
                        text(format!("{}", i + 1))
                            .size(12)
                            .font(BOLD)
                            .color(Color::WHITE)
                            .center()
                    )
                    .width(24)
                    .height(24)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(Vertical::Center)
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::from_rgb8(255, 59, 129))),
                        border: Border {
                            radius: 12.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
                    text_input("Was soll anders sein?", &m.note)
                        .on_input(move |t| edit(Edit::MarkupNote(i, t)))
                        .width(Length::Fill),
                    button(text("✕").size(12).color(text_color()).center())
                        .width(28)
                        .height(28)
                        .padding(0)
                        .on_press(edit(Edit::MarkupDelete(i)))
                        .style(flat(Look::Secondary)),
                ]
                .spacing(8)
                .align_y(Vertical::Center),
            )
        });
    let side = column![
        hint("Ziehe auf dem Bild ein Rechteck über das, was du meinst, oder klicke auf eine Stelle. Schreibe dazu, was anders sein soll."),
        scroll(list).height(Length::Fill),
        row![
            btn("Alle löschen", edit(Edit::MarkupClear), false),
            btn("Neues Bild", edit(Edit::MarkupStart), false),
        ]
        .spacing(8),
        btn("Bild und Liste kopieren", edit(Edit::MarkupCopy), true),
        hint("Das Bild mit den nummerierten Bereichen liegt danach in der Zwischenablage, die Liste als Text dazu."),
    ]
    .spacing(12)
    .width(320);
    row![crate::markup::surface(mk), side]
        .spacing(12)
        .height(Length::Fill)
        .into()
}

fn fx(app: &App, i: usize) -> Element<'_, Message> {
    // The strip can be gone (removed in the devices dialog) while this is open.
    let (Some(f), Some(strip)) = (app.fx.get(i), app.strips.get(i)) else {
        return frame("Effekte".into(), 1040.0, false, hint("Lade …"), no_footer());
    };
    let lit = (strip.shown + 90.0).clamp(1.0, 89.0) as u16;
    // One band of the equaliser: value on top, the fader, frequency below.
    let band = |b: usize, name: &'static str, hz: &'static str| {
        column![
            text(format!("{:+.1}", f.eq[b]))
                .font(MONO)
                .size(13)
                .color(text_color()),
            vertical_slider(-12.0..=12.0, f.eq[b], move |v| edit(Edit::Eq(i, b, v)))
                .step(0.5_f32)
                .height(EQ_H),
            text(tr(name)).size(13).color(text_color()),
            text(hz).font(MONO).size(11).color(muted_color()),
        ]
        .spacing(6)
        .align_x(iced::alignment::Horizontal::Center)
        .width(Length::FillPortion(1))
    };
    // Scale on the left, like a hardware equaliser.
    let eq_scale = column![
        space().height(24),
        column(["+12", "+6", "0", "-6", "-12"].into_iter().map(|t| {
            text(t)
                .font(MONO)
                .size(11)
                .color(if t == "0" {
                    text_color()
                } else {
                    muted_color()
                })
                .into()
        }))
        .height(EQ_H)
        .width(30)
        .align_x(iced::alignment::Horizontal::Right)
        .spacing((EQ_H - 5.0 * 14.0) / 4.0),
    ];
    let echo_row = |name: &'static str, value: String, bar: Element<'static, Message>| {
        row![
            container(text(name).size(13).color(text_color())).width(150),
            bar,
            container(text(value).font(MONO).size(13).color(text_color()))
                .width(70)
                .align_right(Length::Fill),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
    };
    // Words instead of percentages: each choice sets one value, the one
    // nearest to the current value is shown as on.
    let nearest = |value: f32, options: &[f32]| -> usize {
        options
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (**a - value).abs().total_cmp(&(**b - value).abs()))
            .map_or(0, |(k, _)| k)
    };
    const FEEDBACK: [(&str, f32); 4] = [
        ("Keine", 0.0),
        ("Wenige", 0.2),
        ("Mittel", 0.45),
        ("Viele", 0.7),
    ];
    const MIX: [(&str, f32); 3] = [("Dezent", 0.15), ("Mittel", 0.3), ("Deutlich", 0.5)];
    const ECHO_STYLES: [(&str, f32, f32, f32); 4] = [
        ("Slapback", 120.0, 0.15, 0.3),
        ("Raum", 250.0, 0.3, 0.25),
        ("Hall", 500.0, 0.5, 0.3),
        ("Weit", 800.0, 0.6, 0.35),
    ];
    const EQ_STYLES: [(&str, [f32; 3]); 4] = [
        ("Flach", [0.0, 0.0, 0.0]),
        ("Stimme klar", [-2.0, 3.0, 1.5]),
        ("Warm", [3.0, -1.0, -3.0]),
        ("Hell", [-3.0, 2.0, 4.0]),
    ];
    let fb_now = nearest(
        f.echo_feedback,
        &FEEDBACK.iter().map(|(_, v)| *v).collect::<Vec<_>>(),
    );
    let mix_now = nearest(f.echo_mix, &MIX.iter().map(|(_, v)| *v).collect::<Vec<_>>());
    let style_now = ECHO_STYLES.iter().position(|(_, ms, fb, mix)| {
        (f.echo_ms - ms).abs() < 5.0
            && (f.echo_feedback - fb).abs() < 0.03
            && (f.echo_mix - mix).abs() < 0.03
    });
    let eq_now = EQ_STYLES
        .iter()
        .position(|(_, g)| (0..3).all(|b| (f.eq[b] - g[b]).abs() < 0.3));
    let echo_style = segmented(
        ECHO_STYLES
            .iter()
            .enumerate()
            .map(|(k, (name, ms, fb, mix))| {
                (
                    tr(name),
                    style_now == Some(k),
                    edit(Edit::EchoStyle(i, *ms, *fb, *mix)),
                )
            })
            .collect(),
    );
    let echo_feedback = segmented(
        FEEDBACK
            .iter()
            .enumerate()
            .map(|(k, (name, v))| (tr(name), fb_now == k, edit(Edit::EchoFeedback(i, *v))))
            .collect(),
    );
    let echo_mix = segmented(
        MIX.iter()
            .enumerate()
            .map(|(k, (name, v))| (tr(name), mix_now == k, edit(Edit::EchoMix(i, *v))))
            .collect(),
    );
    let eq_style = segmented(
        EQ_STYLES
            .iter()
            .enumerate()
            .map(|(k, (name, g))| (tr(name), eq_now == Some(k), edit(Edit::EqStyle(i, *g))))
            .collect(),
    );
    let gate_bar = row![
        space().width(160),
        container(row![
            container(space())
                .width(Length::FillPortion(lit))
                .height(8)
                .style(|_| container::Style {
                    background: Some(Background::Color(signal())),
                    ..container::Style::default()
                }),
            space().width(Length::FillPortion(90 - lit)),
        ])
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(panel_2())),
            border: Border {
                radius: 4.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        }),
    ];
    let denoise_card = card(
        "Rauschfilter",
        column![
            check(f.denoise)
                .label(tr("Hintergrundgeräusche entfernen"))
                .on_toggle(move |v| edit(Edit::Denoise(i, v))),
            hint("Filtert Tastatur, Lüfter und Rauschen aus der Stimme."),
            row![
                container(text("Stärke").size(13).color(text_color())).width(150),
                segmented(
                    [("Leicht", 0.4_f32), ("Mittel", 0.7), ("Stark", 1.0)]
                        .into_iter()
                        .map(|(name, amount)| (
                            tr(name),
                            (f.denoise_amount - amount).abs() < 0.05,
                            edit(Edit::DenoiseAmount(i, amount))
                        ))
                        .collect()
                ),
            ]
            .align_y(Vertical::Center),
        ]
        .spacing(8),
    );
    let low_cut_card = card(
        "Low-Cut",
        column![
            segmented(
                [0u16, 80, 120, 160]
                    .into_iter()
                    .map(|hz| (
                        if hz == 0 {
                            "Aus".into()
                        } else {
                            format!("{hz} Hz")
                        },
                        f.low_cut == hz,
                        edit(Edit::LowCut(i, hz))
                    ))
                    .collect()
            ),
            hint("Schneidet Brummen und Trittschall unten ab."),
        ]
        .spacing(8),
    );
    let eq_card = card(
        "EQ",
        column![
            eq_style,
            row![
                eq_scale,
                band(0, "Bass", "120 Hz"),
                band(1, "Präsenz", "3 kHz"),
                band(2, "Höhen", "8 kHz"),
            ]
            .spacing(8),
            hint("Regler ziehen: ±12 dB in Schritten von 0,5."),
        ]
        .spacing(10),
    );
    let gate_card = card(
        "Gate",
        column![
            check(f.gate)
                .label(tr("Stumm unter der Schwelle"))
                .on_toggle(move |v| edit(Edit::Gate(i, v))),
            row![
                container(text("Schwelle").size(13).color(text_color())).width(150),
                ctl(app, -90.0..=-10.0, f.gate_db, 1.0, move |v| edit(
                    Edit::GateDb(i, v.round())
                )),
                container(
                    text(format!("{} dB", f.gate_db))
                        .font(MONO)
                        .size(13)
                        .color(text_color())
                )
                .width(70)
                .align_right(Length::Fill),
            ]
            .spacing(10)
            .align_y(Vertical::Center),
            gate_bar,
            hint("Der Balken zeigt den Pegel, der Strich die Schwelle."),
        ]
        .spacing(8),
    );
    let echo_card = card(
        "Echo",
        column![
            check(f.echo)
                .label(tr("Echo-Effekt"))
                .on_toggle(move |v| edit(Edit::Echo(i, v))),
            echo_row("Stil", String::new(), echo_style),
            echo_row(
                "Abstand",
                format!("{} ms", f.echo_ms.round()),
                ctl(app, 50.0..=1000.0, f.echo_ms, 10.0, move |v| edit(
                    Edit::EchoMs(i, (v / 10.0).round() * 10.0)
                ))
            ),
            echo_row("Nachhall", String::new(), echo_feedback),
            echo_row("Lautstärke", String::new(), echo_mix),
            hint("Stil stellt alles auf einmal ein. Darunter kannst du Abstand, Nachhall und Lautstärke des Echos einzeln ändern."),
        ]
        .spacing(8),
    );
    let body = row![
        cards(1, vec![denoise_card, low_cut_card, eq_card]),
        cards(1, vec![gate_card, echo_card, card("Ducking", duck(app, i))]),
    ]
    .spacing(12)
    .align_y(Vertical::Top);
    frame(
        format!("{}: {}", tr("Effekte"), strip.label),
        1040.0,
        false,
        body.into(),
        row![btn("Zurücksetzen", edit(Edit::ResetFx(i)), false),]
            .spacing(8)
            .into(),
    )
}

/// Ducking: this input gets quieter while another one is loud.
fn duck(app: &App, i: usize) -> Element<'static, Message> {
    let d = app.duck.get(i).copied().unwrap_or_default();
    let off = Choice {
        id: String::new(),
        label: tr("Aus").to_string(),
    };
    let mut choices = vec![off.clone()];
    for (j, s) in app.strips.iter().enumerate().filter(|(j, _)| *j != i) {
        choices.push(Choice {
            id: j.to_string(),
            label: s.label.clone(),
        });
    }
    let current = d
        .trigger
        .and_then(|t| choices.iter().find(|c| c.id == t.to_string()).cloned())
        .unwrap_or(off);
    let line = |name: &'static str,
                range: std::ops::RangeInclusive<f32>,
                value: f32,
                unit: &'static str,
                step: f32,
                make: fn(usize, f32) -> Edit| {
        row![
            container(text(name).size(13).color(text_color())).width(150),
            ctl(app, range, value, step, move |v| edit(make(i, v))),
            container(
                text(format!("{value:.0} {unit}"))
                    .font(MONO)
                    .size(13)
                    .color(text_color())
            )
            .width(70)
            .align_right(Length::Fill),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
    };
    let mut body = column![
        row![
            container(text("Auslöser").size(13).color(text_color())).width(150),
            pick_list(choices, Some(current), move |c| edit(Edit::DuckTrigger(
                i,
                c.id.parse().ok()
            )))
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Vertical::Center),
        hint("Wird leiser, sobald der gewählte Eingang spricht, z. B. Musik unter dem Mikrofon."),
    ]
    .spacing(8);
    if d.trigger.is_some() {
        body = body
            .push(line(
                "Schwelle",
                -80.0..=-10.0,
                d.threshold_db,
                "dB",
                1.0,
                Edit::DuckThreshold,
            ))
            .push(line(
                "Absenkung",
                0.0..=40.0,
                d.amount_db,
                "dB",
                1.0,
                Edit::DuckAmount,
            ))
            .push(line(
                "Einsatz",
                1.0..=500.0,
                d.attack_ms,
                "ms",
                1.0,
                Edit::DuckAttack,
            ))
            .push(line(
                "Rückkehr",
                20.0..=3000.0,
                d.release_ms,
                "ms",
                10.0,
                Edit::DuckRelease,
            ));
    }
    body.into()
}

/// A device for a pick list: shown by its label, identified by `id`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Choice {
    pub id: String,
    pub label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn error_line(app: &App) -> Element<'static, Message> {
    if app.dialog_error.is_empty() {
        space().height(0).into()
    } else {
        text(app.dialog_error.clone())
            .size(13)
            .color(danger())
            .into()
    }
}

fn apps(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    match &app.apps_data {
        None => list = list.push(hint("Lade …")),
        Some(data) if !data.supported => {
            list = list.push(hint(
                "Ausgabegeräte pro Programm gibt es nur unter Windows.",
            ))
        }
        Some(data) => {
            if !data.devices.iter().any(|d| d.cable) {
                list = list.push(
                    container(
                        text("Noch kein Faderust Kabel. Mit „+ Eingang“ nimmt Faderust ein Programm direkt auf. Damit es dann nicht zusätzlich über die Boxen läuft, braucht es ein Gerät, das du nicht hörst: das Faderust Kabel (Einrichtung unter Geräte).")
                            .size(13)
                            .color(text_color()),
                    )
                    .padding([10, 12])
                    .style(|_| container::Style {
                        border: Border {
                            color: accent(),
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        ..container::Style::default()
                    }),
                );
            }
            for cable in app.unused_cables() {
                list = list.push(
                    row![
                        text(format!(
                            "{} {}",
                            device_label(&cable),
                            tr("wird von keinem Eingang genutzt.")
                        ))
                        .size(13)
                        .color(text_color()),
                        space().width(Length::Fill),
                        btn(
                            "Als Eingang anlegen",
                            edit(Edit::AppInput(cable.clone(), device_label(&cable))),
                            true
                        ),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            }
            let quiet = app.quiet_cable();
            let cfg = app.config();
            // Programs playing now, then programs that are inputs but silent.
            let mut rows: Vec<(String, String, Vec<u32>)> = data
                .apps
                .iter()
                .map(|a| {
                    (
                        format!("app:{}", a.exe.to_lowercase()),
                        a.name.clone(),
                        a.pids.clone(),
                    )
                })
                .collect();
            for i in &cfg.inputs {
                let device = i.device.to_lowercase();
                if device.starts_with("app:") && !rows.iter().any(|r| r.0 == device) {
                    rows.push((device, i.label.clone(), vec![]));
                }
            }
            if rows.is_empty() {
                list = list.push(hint(
                    "Gerade spielt kein Programm Ton ab. Starte die Wiedergabe und tippe auf „Aktualisieren“.",
                ));
            }
            for (device, name, pids) in rows {
                let on = cfg
                    .inputs
                    .iter()
                    .any(|i| i.device.eq_ignore_ascii_case(&device));
                let switch = {
                    let (device, name, pids) = (device.clone(), name.clone(), pids.clone());
                    let cable = quiet.as_ref().map(|q| q.id.clone());
                    check(on).label(tr("Über Faderust")).on_toggle(move |v| {
                        edit(if v {
                            Edit::ProgramOn(
                                device.clone(),
                                name.clone(),
                                pids.clone(),
                                cable.clone(),
                            )
                        } else {
                            Edit::ProgramOff(device.clone(), pids.clone())
                        })
                    })
                };
                let state = if pids.is_empty() {
                    tr("beendet")
                } else {
                    String::new()
                };
                list = list.push(
                    row![
                        container(text(name).size(14).color(text_color())).width(Length::Fill),
                        text(state).size(12).color(muted_color()),
                        switch,
                    ]
                    .spacing(12)
                    .align_y(Vertical::Center),
                );
            }
            if quiet.is_none() && !data.apps.is_empty() {
                list = list.push(
                    row![
                        hint(
                            "Ohne ein Faderust Kabel hörst du ein aufgenommenes Programm doppelt."
                        ),
                        space().width(Length::Fill),
                        btn("Faderust Kabel einrichten", edit(Edit::UsbSetup), true),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            }
        }
    }
    let body = column![
        error_line(app),
        list,
        hint("Ein Programm mit „Über Faderust“ wird auf das stille Faderust Kabel gestellt und als Eingang aufgenommen; beim Ausschalten geht es zurück auf Windows-Standard."),
        text("Windows merkt sich die Zuordnung pro Programm. Manche Programme übernehmen sie erst nach einem Neustart.")
            .size(12)
            .color(muted_color()),
    ]
    .spacing(16);
    frame(
        "Programme".into(),
        640.0,
        false,
        body.into(),
        row![btn("Aktualisieren", edit(Edit::Refresh), false),]
            .spacing(8)
            .into(),
    )
}

fn presets(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    for p in &app.presets {
        list = list.push(
            row![
                container(text(p.clone()).size(14).color(text_color())).width(Length::Fill),
                btn("Laden", edit(Edit::LoadPreset(p.clone())), true),
                btn("Export", edit(Edit::ExportPreset(p.clone())), false),
                btn("Löschen", edit(Edit::DeletePreset(p.clone())), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let body = column![
        error_line(app),
        h3("Gespeicherte Presets"),
        list,
        btn("Preset importieren …", edit(Edit::ImportPreset), false),
        h3("Aktuelle Einstellung speichern"),
        row![
            text_input("Name", &app.new_preset)
                .on_input(|t| edit(Edit::NewPreset(t)))
                .on_submit(edit(Edit::SavePreset)),
            btn("Speichern", edit(Edit::SavePreset), true),
        ]
        .spacing(8),
        hint("Ein Preset merkt sich Fader, Stumm und Effekte aller Eingänge."),
    ]
    .spacing(10);
    frame("Presets".into(), 520.0, false, body.into(), no_footer())
}

fn soundboard(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    for (i, s) in app.sounds.iter().enumerate() {
        list = list.push(
            row![
                button(
                    text(s.name.clone())
                        .size(14)
                        .color(look_text(Look::Primary))
                )
                .width(Length::Fill)
                .padding([8, 14])
                .on_press(edit(Edit::SoundPlay(i)))
                .style(flat(Look::Primary)),
                btn("Entfernen", edit(Edit::SoundRemove(i)), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let body = column![
        error_line(app),
        row![
            btn("Sounds hinzufügen …", edit(Edit::SoundAdd), false),
            btn("Alle stoppen", edit(Edit::SoundStop), false),
        ]
        .spacing(8),
        if app.sounds.is_empty() {
            hint("Noch keine Sounds. Füge WAV- oder MP3-Dateien hinzu.")
        } else {
            list.into()
        },
        hint(
            "Ein Klick spielt den Sound auf dem Streifen „Soundboard“. Dort stellst du Lautstärke und Ausgänge ein."
        ),
    ]
    .spacing(10);
    frame("Soundboard".into(), 520.0, false, body.into(), no_footer())
}

fn windows(app: &App) -> Element<'_, Message> {
    let mut body = column![error_line(app)].spacing(10);
    match &app.win {
        None => body = body.push(hint("Lade …")),
        Some(w) if !w.supported => {
            body = body.push(hint(
                "Die Windows-Soundeinstellungen gibt es nur unter Windows.",
            ))
        }
        Some(w) => {
            for (title, list) in [("Wiedergabe", &w.playback), ("Aufnahme", &w.recording)] {
                body = body.push(h3(title));
                let choices: Vec<Choice> = list
                    .iter()
                    .map(|e| Choice {
                        id: e.id.clone(),
                        label: device_label(&e.name),
                    })
                    .collect();
                let pick = |label: &'static str, comm: bool| {
                    let current = list
                        .iter()
                        .find(|e| {
                            if comm {
                                e.is_default_communications
                            } else {
                                e.is_default
                            }
                        })
                        .and_then(|e| choices.iter().find(|c| c.id == e.id).cloned());
                    row![
                        container(text(label).size(13).color(text_color())).width(130),
                        pick_list(choices.clone(), current, move |c| edit(Edit::WinDefault(
                            c.id, comm
                        )))
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(Vertical::Center)
                };
                body = body
                    .push(pick("Standardgerät", false))
                    .push(pick("Für Anrufe", true));
                for e in list {
                    let id = e.id.clone();
                    let badge = if e.is_default {
                        "  Standard"
                    } else if e.is_default_communications {
                        "  Anrufe"
                    } else {
                        ""
                    };
                    let mute_id = id.clone();
                    let muted = e.muted;
                    body = body.push(
                        row![
                            container(
                                text(format!("{}{badge}", device_label(&e.name)))
                                    .size(13)
                                    .color(text_color())
                            )
                            .width(230),
                            slider(0.0..=100.0, e.volume * 100.0, move |v| edit(
                                Edit::WinVolume(id.clone(), v)
                            )),
                            container(
                                text(format!("{} %", (e.volume * 100.0).round()))
                                    .font(MONO)
                                    .size(13)
                                    .color(text_color())
                            )
                            .width(54)
                            .align_right(Length::Fill),
                            button(text(if muted { "Stumm" } else { "Laut" }).size(12).color(
                                look_text(if muted { Look::Danger } else { Look::Secondary })
                            ))
                            .padding([4, 8])
                            .width(64)
                            .on_press(edit(Edit::WinMute(mute_id, !muted)))
                            .style(flat(if muted {
                                Look::Danger
                            } else {
                                Look::Secondary
                            })),
                        ]
                        .spacing(10)
                        .align_y(Vertical::Center),
                    );
                }
            }
            body = body.push(hint(
                "Faderust nimmt Geräte direkt ab. Der Windows-Standard bestimmt nur, wohin Programme ohne eigene Zuordnung spielen und welches Mikrofon sie nutzen.",
            ));
        }
    }
    frame(
        "Windows-Sound".into(),
        700.0,
        false,
        body.into(),
        row![btn("Aktualisieren", edit(Edit::Refresh), false),]
            .spacing(8)
            .into(),
    )
}

fn design(app: &App) -> Element<'_, Message> {
    let swatch = |hex: &'static str, on: bool, msg: Message| {
        let c = hex_color(hex);
        button(space())
            .width(28)
            .height(28)
            .style(move |_, _| button::Style {
                background: Some(Background::Color(c)),
                border: Border {
                    color: if on { text_color() } else { border() },
                    width: if on { 3.0 } else { 1.0 },
                    radius: 14.0.into(),
                },
                ..button::Style::default()
            })
            .on_press(msg)
    };
    let hide = HIDE_AFTER
        .iter()
        .map(|h| {
            (
                h.to_string(),
                app.hide_after == *h,
                edit(Edit::HideAfter(h)),
            )
        })
        .collect();
    let accent_now = app.accent();
    let signal_now = app.signal();
    let mode_now = app.mode();
    let accents = ACCENTS.iter().fold(row![].spacing(8), |r, (_, _, hex)| {
        r.push(swatch(
            hex,
            accent_now.eq_ignore_ascii_case(hex),
            edit(Edit::Accent((*hex).into())),
        ))
    });
    let signals = SIGNALS.iter().fold(row![].spacing(8), |r, (_, _, hex)| {
        r.push(swatch(
            hex,
            signal_now.eq_ignore_ascii_case(hex),
            edit(Edit::Signal((*hex).into())),
        ))
    });
    let lang_now = app.lang();
    let view_card = card(
        "Ansicht",
        segmented(vec![
            (
                "Pult".into(),
                matches!(app.view, crate::View::Desk),
                Message::SetView(crate::View::Desk),
            ),
            (
                "Matrix".into(),
                matches!(app.view, crate::View::Matrix),
                Message::SetView(crate::View::Matrix),
            ),
        ]),
    );
    let hide_card = card(
        "Stille Programme ausblenden",
        column![
            segmented(hide),
            hint("„Beendete“ blendet nur Programme aus, die geschlossen sind. Stille, aber geöffnete bleiben sichtbar."),
        ]
        .spacing(8),
    );
    let accent_card = card(
        "Akzentfarbe",
        column![
            accents,
            custom_color(
                app.accent_input
                    .clone()
                    .unwrap_or_else(|| accent_now.clone()),
                Target::Accent,
                app.picker,
                |t| edit(Edit::AccentHex(t)),
            ),
        ]
        .spacing(10),
    );
    let signal_card = card(
        "Signalfarbe",
        column![
            signals,
            custom_color(
                app.signal_input
                    .clone()
                    .unwrap_or_else(|| signal_now.clone()),
                Target::Signal,
                app.picker,
                |t| edit(Edit::SignalHex(t)),
            ),
        ]
        .spacing(10),
    );
    let lcd_now = app.lcd_hex();
    let lcd_custom = !app.prefs.lcd_color.is_empty();
    let lcd_swatches = LCD_COLORS.iter().fold(row![].spacing(8), |r, (_, _, hex)| {
        r.push(swatch(
            hex,
            lcd_custom && lcd_now.eq_ignore_ascii_case(hex),
            edit(Edit::Lcd((*hex).into())),
        ))
    });
    let lcd_card = card(
        "Display-Farbe",
        column![
            lcd_swatches,
            custom_color(
                app.lcd_input.clone().unwrap_or_else(|| lcd_now.clone()),
                Target::Lcd,
                app.picker,
                |t| edit(Edit::LcdHex(t)),
            ),
        ]
        .spacing(10),
    );
    let looks = [
        ("", "Standard"),
        ("flat", "Flach"),
        ("glow", "Leuchtend"),
        ("pill", "Pille"),
        ("neon", "Neon"),
        ("hardware", "Hardware"),
        ("tile", "Kachel"),
    ];
    let button_card = card(
        "Knopf-Look",
        column![
            segmented(
                looks[..4]
                    .iter()
                    .map(|(k, n)| (
                        (*n).into(),
                        app.prefs.button_look == *k,
                        edit(Edit::ButtonLook(k))
                    ))
                    .collect()
            ),
            segmented(
                looks[4..]
                    .iter()
                    .map(|(k, n)| (
                        (*n).into(),
                        app.prefs.button_look == *k,
                        edit(Edit::ButtonLook(k))
                    ))
                    .collect()
            ),
        ]
        .spacing(8),
    );
    let parts_card = card(
        "Farben einzeln",
        crate::style::Part::ALL
            .iter()
            .fold(column![].spacing(12), |col, &x| {
                let set = app.prefs.part_colors.get(x.key()).cloned();
                let shown = match &app.part_input {
                    Some((p, t)) if *p == x => t.clone(),
                    _ => set.clone().unwrap_or_default(),
                };
                let custom = set.is_some();
                col.push(
                    column![
                        row![
                            text(tr(x.label())).size(13).font(BOLD),
                            space().width(Length::Fill),
                            if custom {
                                btn(tr("Standard"), edit(Edit::PartHex(x, String::new())), false)
                            } else {
                                text(tr("Standard")).size(12).color(muted_color()).into()
                            },
                        ]
                        .align_y(Vertical::Center),
                        custom_color(
                            if shown.is_empty() {
                                app.part_default(x)
                            } else {
                                shown
                            },
                            Target::Part(x),
                            app.picker,
                            move |t| edit(Edit::PartHex(x, t)),
                        ),
                    ]
                    .spacing(6),
                )
            }),
    );
    let edge_card = card(
        "Grüner Rand bei Signal",
        opts(vec![
            opt(
                "Ausgänge",
                "Der Streifen leuchtet grün, solange Ton durchläuft.",
                !app.prefs.output_edge_off,
                |v| edit(Edit::OutputEdge(v)),
            ),
            opt(
                "Geräte",
                "Wie bei den Ausgängen, für Mikrofone und andere Geräte.",
                !app.prefs.device_edge_off,
                |v| edit(Edit::DeviceEdge(v)),
            ),
            opt(
                "Programme",
                "Wie bei den Ausgängen, für Programme.",
                !app.prefs.program_edge_off,
                |v| edit(Edit::ProgramEdge(v)),
            ),
        ]),
    );
    let fader_card = card(
        "Fader",
        opts(vec![
            opt(
                "Skala am Fader",
                "Zahlen in dB neben der Bahn.",
                !app.prefs.scale_off,
                |v| edit(Edit::Scale(v)),
            ),
            opt(
                "Wert in der Kappe",
                "Die aktuelle dB-Zahl steht auf der Fader-Kappe.",
                !app.prefs.cap_value_off,
                |v| edit(Edit::CapValue(v)),
            ),
            opt(
                "Nullstrich am Fader",
                "Eine Linie markiert 0 dB.",
                app.prefs.zero_mark,
                |v| edit(Edit::ZeroMark(v)),
            ),
            opt(
                "Leuchten am Fader",
                "Die Bahn leuchtet bis zur Kappe.",
                !app.prefs.glow_off,
                |v| edit(Edit::Glow(v)),
            ),
            opt(
                "Eckige Fader-Kappe",
                "Statt der runden Kappe.",
                app.prefs.cap_square,
                |v| edit(Edit::CapSquare(v)),
            ),
            opt(
                "Kappe in Akzentfarbe",
                "Die Kappe nimmt die Akzentfarbe an.",
                app.prefs.accent_cap,
                |v| edit(Edit::AccentCap(v)),
            ),
            opt(
                "Schmale Bahn",
                "Dünnere Führungsschiene.",
                app.prefs.thin_track,
                |v| edit(Edit::ThinTrack(v)),
            ),
            opt(
                "Schmaler Griff als Kappe",
                "Ein flacher Griff statt der großen Kappe.",
                app.prefs.grip_cap,
                |v| edit(Edit::GripCap(v)),
            ),
            opt(
                "Wert im Display",
                "Die dB-Zahl unter dem Fader steht in einem eigenen Display.",
                app.prefs.readout_display,
                |v| edit(Edit::ReadoutDisplay(v)),
            ),
        ]),
    );
    let form = if app.prefs.meter_line {
        4
    } else if app.prefs.meter_look == 2 {
        3
    } else if app.prefs.meter_look == 1 {
        2
    } else if app.prefs.meter_segments {
        1
    } else {
        0
    };
    let form_btn = |n: u8, label: &str| (label.to_string(), form == n, edit(Edit::MeterForm(n)));
    let meter_card = card(
        "Pegel",
        column![
            hint("Form der Pegelanzeige am Fader"),
            segmented(vec![
                form_btn(0, "Balken"),
                form_btn(1, "Segmente"),
                form_btn(2, "Punkte"),
            ]),
            segmented(vec![form_btn(3, "Blöcke"), form_btn(4, "Linie")]),
            opts(vec![
                opt(
                    "Pegel einfarbig",
                    "Ohne Farbverlauf von Grün nach Rot, auch in der Matrix.",
                    app.prefs.one_color,
                    |v| edit(Edit::OneColor(v)),
                ),
                opt(
                    "Spitzenwert-Marke",
                    "Eine Marke hält kurz den höchsten Pegel, auch in der Matrix.",
                    !app.prefs.peak_off,
                    |v| edit(Edit::PeakMark(v)),
                ),
            ]),
        ]
        .spacing(10),
    );
    let matrix_card = card(
        "Matrix",
        opts(vec![
            opt(
                "Pegelbalken in der Matrix",
                "Ein dünner Pegelbalken unter jeder Zeile.",
                !app.prefs.matrix_meters_off,
                |v| edit(Edit::MatrixMeters(v)),
            ),
            opt(
                "Spitzenwert-Marke",
                "Gilt für Pult und Matrix, die Matrix zeigt sie auf dem Balken.",
                !app.prefs.peak_off,
                |v| edit(Edit::PeakMark(v)),
            ),
            opt(
                "Pegel einfarbig",
                "Gilt für Pult und Matrix.",
                app.prefs.one_color,
                |v| edit(Edit::OneColor(v)),
            ),
        ]),
    );
    let matrix_fx_card = card(
        "Effekte",
        opts(vec![opt(
            "Drehregler mit LED-Kranz",
            "Die Regler der Effekte sind Drehknöpfe statt Schieber.",
            app.prefs.knobs,
            |v| edit(Edit::Knobs(v)),
        )]),
    );
    let space_save_card = card(
        "Platz sparen",
        opts(vec![
            opt(
                "dB-Zahl unter dem Fader ausblenden",
                "Der Wert steht schon in der Kappe, die Zeile wird frei.",
                app.prefs.hide_readout,
                |v| edit(Edit::HideReadout(v)),
            ),
            opt(
                "Mute, Effekte und Echo in einer Reihe",
                "Drei kleine Tasten nebeneinander statt untereinander.",
                app.prefs.button_row,
                |v| edit(Edit::ButtonRow(v)),
            ),
            opt(
                "Schmale Ausgänge und Geräte",
                "So schmal wie die Programme: mehr Programme auf einmal sichtbar.",
                app.prefs.slim_strips,
                |v| edit(Edit::SlimStrips(v)),
            ),
            opt(
                "Dünne Scrollleiste bei den Programmen",
                "Weniger freier Rand unter den Streifen.",
                app.prefs.slim_scroll,
                |v| edit(Edit::SlimScroll(v)),
            ),
        ]),
    );
    let head_card = card(
        "Streifen und Leiste",
        opts(vec![
            opt(
                "Streifenkopf mit Symbol",
                "Symbol und Farbe des Programms oben am Streifen.",
                app.prefs.strip_head,
                |v| edit(Edit::StripHead(v)),
            ),
            opt(
                "Pegelverlauf im Kopf (30 s)",
                "Eine kleine Kurve zeigt die letzten 30 Sekunden.",
                app.prefs.strip_history,
                |v| edit(Edit::StripHistory(v)),
            ),
            opt(
                "Pfeile zum Sortieren auf den Streifen",
                "Streifen lassen sich nach links und rechts schieben.",
                app.prefs.sort_strips,
                |v| edit(Edit::SortStrips(v)),
            ),
            opt(
                "Echo-Taste auf jedem Streifen",
                "Schaltet den Echo-Effekt direkt am Streifen.",
                app.prefs.echo_button,
                |v| edit(Edit::EchoButton(v)),
            ),
            opt(
                "Eigene Farbe pro Streifen",
                "Jeder Streifen bekommt seinen eigenen Farbton.",
                app.prefs.strip_colors,
                |v| edit(Edit::StripColors(v)),
            ),
            opt(
                "Streifen als erhabene Platten",
                "Die Streifen wirken wie Platten mit Schatten.",
                app.prefs.relief,
                |v| edit(Edit::Relief(v)),
            ),
        ]),
    );
    let extras_card = card(
        "Fenster",
        opts(vec![
            opt(
                "Soundboard",
                "Zeigt das Soundboard oben in der Leiste.",
                app.prefs.soundboard,
                |v| edit(Edit::Soundboard(v)),
            ),
            opt(
                "Einblendung bei Stream Deck",
                "Kurze Anzeige am Bildschirmrand, wenn du am Stream Deck drehst.",
                !app.prefs.osd_off,
                |v| edit(Edit::Osd(v)),
            ),
            opt(
                "Mit Windows starten",
                "Faderust startet beim Anmelden im Tray, ohne Fenster.",
                app.autostart,
                |v| edit(Edit::Autostart(v)),
            ),
            opt(
                "Kleiner Mixer neben der Taskbar",
                "Ein Klick auf das Tray-Symbol öffnet einen kleinen Mixer.",
                app.prefs.tray_popup,
                |v| edit(Edit::TrayPopup(v)),
            ),
        ]),
    );
    let look_card = card(
        "Design",
        column![
            segmented(vec![
                (
                    "Wie Windows".into(),
                    mode_now == "system",
                    edit(Edit::Mode("system"))
                ),
                (
                    "Hell".into(),
                    mode_now == "light",
                    edit(Edit::Mode("light"))
                ),
                (
                    "Dunkel".into(),
                    mode_now == "dark",
                    edit(Edit::Mode("dark"))
                ),
            ]),
            segmented(vec![
                (
                    "Carbon".into(),
                    mode_now == "carbon",
                    edit(Edit::Mode("carbon"))
                ),
                (
                    "GitHub-Dunkel".into(),
                    mode_now == "github",
                    edit(Edit::Mode("github"))
                ),
                (
                    "Mitternacht".into(),
                    mode_now == "midnight",
                    edit(Edit::Mode("midnight"))
                ),
            ]),
            segmented(vec![
                (
                    "Sonnenuntergang".into(),
                    mode_now == "sunset",
                    edit(Edit::Mode("sunset"))
                ),
                (
                    "Terminal-Grün".into(),
                    mode_now == "terminal",
                    edit(Edit::Mode("terminal"))
                ),
            ]),
            segmented(vec![
                (
                    "Neon-Pult".into(),
                    mode_now == "neon",
                    edit(Edit::Mode("neon"))
                ),
                (
                    "Papier".into(),
                    mode_now == "paper",
                    edit(Edit::Mode("paper"))
                ),
            ]),
        ]
        .spacing(8),
    );
    let lang_card = card(
        "Sprache",
        segmented(vec![
            ("English".into(), lang_now == "en", edit(Edit::Lang("en"))),
            ("Deutsch".into(), lang_now == "de", edit(Edit::Lang("de"))),
        ]),
    );
    let density = app.prefs.density.unwrap_or(1);
    let glide = app.prefs.glide.unwrap_or(1);
    let space_card = card(
        "Dichte und Bewegung",
        column![
            hint("Abstand der Streifen"),
            segmented(vec![
                ("Kompakt".into(), density == 0, edit(Edit::Density(0))),
                ("Normal".into(), density == 1, edit(Edit::Density(1))),
                ("Luftig".into(), density == 2, edit(Edit::Density(2))),
            ]),
            hint("Fader gleiten"),
            segmented(vec![
                ("Aus".into(), glide == 0, edit(Edit::GlideSpeed(0))),
                ("Normal".into(), glide == 1, edit(Edit::GlideSpeed(1))),
                ("Langsam".into(), glide == 2, edit(Edit::GlideSpeed(2))),
            ]),
        ]
        .spacing(8),
    );
    let profile_rows = app
        .prefs
        .profiles
        .keys()
        .fold(column![].spacing(6), |col, name| {
            col.push(
                row![
                    button(
                        text(name.clone())
                            .size(13)
                            .color(look_text(Look::Secondary))
                    )
                    .width(Length::Fill)
                    .padding([6, 12])
                    .on_press(edit(Edit::ProfileLoad(name.clone())))
                    .style(flat(Look::Secondary)),
                    button(
                        text("✕")
                            .size(12)
                            .color(look_text(Look::Secondary))
                            .center()
                    )
                    .width(28)
                    .padding([6, 0])
                    .on_press(edit(Edit::ProfileDelete(name.clone())))
                    .style(flat(Look::Secondary)),
                ]
                .spacing(6),
            )
        });
    let mut save = text_input("Name", &app.profile_name)
        .on_input(|t| edit(Edit::ProfileName(t)))
        .width(Length::Fill);
    if !app.profile_name.trim().is_empty() {
        save = save.on_submit(edit(Edit::ProfileSave));
    }
    let profile_card = card(
        "Profile",
        column![
            hint("Speichert das ganze Design unter einem Namen. Ein Klick lädt es wieder."),
            profile_rows,
            row![
                save,
                if app.profile_name.trim().is_empty() {
                    Element::from(text(tr("Speichern")).size(13).color(muted_color()))
                } else {
                    btn(tr("Speichern"), edit(Edit::ProfileSave), true)
                },
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        ]
        .spacing(8),
    );
    let preview_card = card(
        "Vorschau",
        column![
            container(app.preview()).center_x(Length::Fill),
            hint("Zeigt jede Änderung sofort: Probier die Schalter einfach aus."),
        ]
        .spacing(8),
    );
    let tab = app.design_tab;
    let tabs = segmented(vec![
        ("Aussehen".into(), tab == 0, edit(Edit::DesignTab(0))),
        ("Pult".into(), tab == 1, edit(Edit::DesignTab(1))),
        ("Matrix".into(), tab == 2, edit(Edit::DesignTab(2))),
        ("Farben".into(), tab == 3, edit(Edit::DesignTab(3))),
        ("Allgemein".into(), tab == 4, edit(Edit::DesignTab(4))),
    ]);
    let tab_hint = match tab {
        1 => Some("Gilt nur für die Ansicht „Pult“."),
        2 => Some("Gilt nur für die Ansicht „Matrix“."),
        _ => None,
    };
    let content: Element<'_, Message> = match tab {
        1 => row![
            cards(1, vec![space_save_card, head_card, edge_card]),
            cards(1, vec![fader_card, meter_card]),
        ]
        .spacing(12)
        .align_y(Vertical::Top)
        .into(),
        2 => row![cards(1, vec![matrix_card]), cards(1, vec![matrix_fx_card]),]
            .spacing(12)
            .align_y(Vertical::Top)
            .into(),
        3 => row![
            cards(1, vec![accent_card, signal_card, lcd_card]),
            cards(1, vec![parts_card]),
        ]
        .spacing(12)
        .align_y(Vertical::Top)
        .into(),
        4 => row![
            cards(1, vec![hide_card, lang_card]),
            cards(1, vec![extras_card]),
        ]
        .spacing(12)
        .align_y(Vertical::Top)
        .into(),
        _ => row![
            cards(1, vec![look_card, button_card]),
            cards(1, vec![view_card, space_card]),
        ]
        .spacing(12)
        .align_y(Vertical::Top)
        .into(),
    };
    let content: Element<'_, Message> = match tab_hint {
        Some(h) => column![hint(h), content].spacing(10).into(),
        None => content,
    };
    let body = row![
        cards(5, vec![preview_card, profile_card]),
        column![tabs, content]
            .spacing(12)
            .width(Length::FillPortion(8)),
    ]
    .spacing(12)
    .align_y(Vertical::Top);
    frame("Design".into(), 1280.0, false, body.into(), no_footer())
}

pub fn tip<'a>(
    content: impl Into<Element<'a, Message>>,
    tip: impl Into<String>,
) -> Element<'a, Message> {
    iced::widget::tooltip(
        content,
        container(text(tip.into()).size(12).color(text_color()))
            .padding(6)
            .style(|_| container::Style {
                background: Some(Background::Color(panel_2())),
                border: Border {
                    color: border(),
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..container::Style::default()
            }),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

/// Any colour: a picker (opened by the swatch) and a field for #rrggbb.
fn custom_color(
    value: String,
    target: Target,
    picker: Option<(Target, (f32, f32, f32))>,
    on_input: impl Fn(String) -> Message + 'static,
) -> Element<'static, Message> {
    let valid = crate::style::parse_hex_input(&value);
    let preview = valid.as_deref().map(crate::style::hex);
    let open = picker.filter(|(t, _)| *t == target);
    let swatch = button(space())
        .width(34)
        .height(28)
        .padding(0)
        .on_press(edit(Edit::Picker(target)))
        .style(move |_, status| button::Style {
            background: Some(Background::Color(preview.unwrap_or(Color::TRANSPARENT))),
            border: Border {
                color: if open.is_some() || matches!(status, button::Status::Hovered) {
                    accent_light()
                } else if preview.is_some() {
                    border()
                } else {
                    danger()
                },
                width: 2.0,
                radius: 6.0.into(),
            },
            ..button::Style::default()
        });
    let head = row![
        text("Eigene Farbe").size(13).color(muted_color()),
        swatch,
        text_input("#rrggbb", &value).on_input(on_input).width(84),
        btn(
            if open.is_some() {
                "Schließen"
            } else {
                "Wähler"
            },
            edit(Edit::Picker(target)),
            false
        ),
    ]
    .spacing(8)
    .align_y(Vertical::Center);
    match open {
        Some((_, hsv)) => column![
            head,
            iced::widget::canvas(crate::picker::Picker { hsv, target })
                .width(crate::picker::W)
                .height(crate::picker::H),
        ]
        .spacing(10)
        .into(),
        None => head.into(),
    }
}

fn hex_color(h: &str) -> Color {
    crate::style::hex(h)
}

fn deck_section(app: &App) -> Element<'_, Message> {
    let Some(deck) = &app.deck else {
        return hint("Lade …");
    };
    let Some(bundled) = &deck.bundled else {
        return hint("Dieses Faderust enthält kein Stream-Deck-Plugin.");
    };
    let label = if deck.refused {
        "Das Plugin ist zu alt und darf sich nicht mehr verbinden. Bitte aktualisieren:".to_string()
    } else if !deck.connected {
        "Stream Deck nicht verbunden. Plugin einrichten oder neu installieren:".to_string()
    } else if deck.update {
        format!(
            "Plugin {} → {bundled} {}",
            deck.installed.as_deref().unwrap_or("(alt)"),
            tr("verfügbar")
        )
    } else {
        format!(
            "Plugin {} {}",
            deck.installed.as_deref().unwrap_or(""),
            tr("ist aktuell")
        )
    };
    let action = if deck.connected && !deck.update {
        None
    } else if deck.connected || deck.refused {
        Some("Plugin aktualisieren")
    } else {
        Some("Plugin installieren")
    };
    let mut r = row![hint(label), space().width(Length::Fill)]
        .spacing(8)
        .align_y(Vertical::Center);
    if let Some(a) = action {
        r = r.push(btn(a, edit(Edit::InstallDeck), false));
    }
    column![r, error_text(&app.deck_error)].spacing(6).into()
}

fn error_text(e: &str) -> Element<'static, Message> {
    if e.is_empty() {
        space().height(0).into()
    } else {
        text(e.to_string()).size(13).color(danger()).into()
    }
}

fn log(app: &App) -> Element<'_, Message> {
    // As high as the window allows, so the buttons never fall off the bottom.
    let log_height = app
        .size_keep
        .map_or(480.0, |s| (s.height - 360.0).clamp(140.0, 520.0));
    let empty = match &app.log_text {
        None => Some("Lade …"),
        Some(t) if t.trim().is_empty() => Some("Noch nichts protokolliert."),
        Some(_) => None,
    };
    let view: Element<'_, Message> = match empty {
        Some(t) => text(t).size(12).color(muted_color()).into(),
        None => iced::widget::text_editor(&app.log_view)
            .on_action(Message::LogAction)
            .size(12)
            .height(log_height)
            .padding(10)
            .style(|_, _| iced::widget::text_editor::Style {
                background: Background::Color(bg()),
                border: Border {
                    color: border(),
                    width: 1.0,
                    radius: 3.0.into(),
                },
                placeholder: muted_color(),
                value: text_color(),
                selection: Color { a: 0.4, ..accent() },
            })
            .into(),
    };
    let body = column![
        hint("Was Faderust zuletzt gemeldet hat. Gezeigt sind die letzten 150 Zeilen; Text lässt sich markieren und mit Strg+C kopieren. „Kopieren“ legt die gezeigten Zeilen in die Zwischenablage, „Alles kopieren“ die ganze Datei. Bei einem Fehler bitte in ein Issue einfügen."),
        row![
            btn("Aktualisieren", edit(Edit::Refresh), false),
            btn("Kopieren", edit(Edit::CopyLog), false),
            btn("Alles kopieren", edit(Edit::CopyLogAll), false),
            btn("Ordner öffnen", edit(Edit::OpenLogFolder), false),
        ]
        .spacing(8),
        view,
    ]
    .spacing(10);
    frame("Protokoll".into(), 900.0, false, body.into(), no_footer())
}

fn selftest(app: &App) -> Element<'_, Message> {
    use faderust_core::selftest::Status;
    let mut body = column![
        error_line(app),
        hint("Prüft Geräte, Puffer, Fernsteuerung und Plugin. Der Bericht lässt sich in ein Issue kopieren."),
    ]
    .spacing(10);
    match &app.checks {
        None => body = body.push(hint("Prüfe …")),
        Some(checks) => {
            let problems = checks
                .iter()
                .filter(|c| matches!(c.status, Status::Warn | Status::Fail))
                .count();
            body = body.push(
                text(if problems == 0 {
                    "Alles in Ordnung".to_string()
                } else {
                    format!("{problems} {}", tr("Punkte brauchen einen Blick"))
                })
                .size(15)
                .color(if problems == 0 { signal() } else { amber() }),
            );
            for c in checks {
                let (mark, color) = match c.status {
                    Status::Ok => ("✓", signal()),
                    Status::Warn => ("!", amber()),
                    Status::Fail => ("✕", danger()),
                    Status::Info => ("i", muted_color()),
                };
                body = body.push(
                    row![
                        container(text(mark).size(14).font(BOLD).color(color)).width(20),
                        column![
                            text(c.title.clone())
                                .size(13)
                                .font(BOLD)
                                .color(text_color()),
                            text(c.detail.clone()).size(12).color(muted_color()),
                        ]
                        .spacing(2),
                    ]
                    .spacing(8),
                );
            }
        }
    }
    frame(
        "Selbsttest".into(),
        700.0,
        false,
        body.into(),
        row![
            btn("Nochmal prüfen", edit(Edit::Refresh), false),
            btn("Bericht kopieren", edit(Edit::CopyChecks), false),
        ]
        .spacing(8)
        .into(),
    )
}

fn settings(app: &App) -> Element<'_, Message> {
    let Some(draft) = &app.draft else {
        return frame("Geräte".into(), 720.0, true, hint("Lade …"), no_footer());
    };
    let device_choices = |list: &[faderust_core::Device], current: &str| -> (Vec<Choice>, Choice) {
        let mut choices: Vec<Choice> = list
            .iter()
            .map(|d| Choice {
                id: d.name.clone(),
                label: device_label(&d.name),
            })
            .collect();
        let lower = current.to_lowercase();
        let selected = list
            .iter()
            .find(|d| d.id == current)
            .or_else(|| list.iter().find(|d| d.name.to_lowercase().contains(&lower)))
            .map(|d| Choice {
                id: d.name.clone(),
                label: device_label(&d.name),
            })
            .unwrap_or(Choice {
                id: current.to_string(),
                label: device_label(current),
            });
        // A configured device that is unplugged right now stays selectable.
        if !choices.contains(&selected) {
            choices.insert(0, selected.clone());
        }
        (choices, selected)
    };
    let arrow = |label: &'static str, msg: Option<Message>| {
        let b = button(text(label).size(13).color(text_color()))
            .padding([4, 8])
            .style(flat(Look::Secondary));
        match msg {
            Some(m) => b.on_press(m),
            None => b,
        }
    };
    let program = |d: &str| d.starts_with("app:");
    let last_dev = draft.inputs.iter().rposition(|i| !program(&i.device));
    let first_dev = draft.inputs.iter().position(|i| !program(&i.device));
    let mut inputs = column![].spacing(8);
    for (i, input) in draft.inputs.iter().enumerate() {
        if program(&input.device) {
            continue;
        }
        let (choices, current) = device_choices(&app.devs.inputs, &input.device);
        let prev = draft.inputs[..i].iter().rposition(|x| !program(&x.device));
        let next = draft.inputs[i + 1..]
            .iter()
            .position(|x| !program(&x.device))
            .map(|p| p + i + 1);
        inputs = inputs.push(
            row![
                text_input("Name", &input.label)
                    .on_input(move |t| edit(Edit::InLabel(i, t)))
                    .width(140),
                pick_list(choices, Some(current), move |c| edit(Edit::InDevice(
                    i, c.id
                )))
                .width(Length::Fill),
                arrow(
                    "↑",
                    (first_dev != Some(i)).then(|| edit(Edit::InMove(
                        i,
                        prev.map_or(0, |p| p as isize - i as isize)
                    )))
                ),
                arrow(
                    "↓",
                    (last_dev != Some(i)).then(|| edit(Edit::InMove(
                        i,
                        next.map_or(0, |n| n as isize - i as isize)
                    )))
                ),
                btn("✕", edit(Edit::InRemove(i)), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let mut outputs = column![].spacing(8);
    let n_out = draft.outputs.len();
    for (o, output) in draft.outputs.iter().enumerate() {
        let (choices, current) = device_choices(&app.devs.outputs, &output.device);
        let release = RELEASE
            .iter()
            .find(|(s, _)| *s == output.release_secs)
            .copied()
            .unwrap_or((output.release_secs, "Andere"));
        outputs = outputs.push(
            row![
                text_input("Name", &output.label)
                    .on_input(move |t| edit(Edit::OutLabel(o, t)))
                    .width(120),
                pick_list(choices, Some(current), move |c| edit(Edit::OutDevice(
                    o, c.id
                )))
                .width(Length::Fill),
                text_input("ms", &output.buffer_ms.to_string())
                    .on_input(move |t| edit(Edit::OutBuffer(o, t)))
                    .width(54),
                pick_list(
                    RELEASE.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
                    Some(release.1),
                    move |n| edit(Edit::OutRelease(
                        o,
                        RELEASE
                            .iter()
                            .find(|(_, x)| *x == n)
                            .map_or(30, |(s, _)| *s)
                    ))
                ),
                arrow("←", (o > 0).then(|| edit(Edit::OutMove(o, -1)))),
                arrow("→", (o + 1 < n_out).then(|| edit(Edit::OutMove(o, 1)))),
                btn("✕", edit(Edit::OutRemove(o)), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let cable_section: Element<'_, Message> = match &app.usb {
        None => hint("Lade …"),
        Some(u) if !u.supported => hint("Das Faderust Kabel gibt es nur unter Windows."),
        Some(u) => {
            let mut col = column![hint(
                "Virtuelle Kabel für Programm-Mikro und Programme, die du nicht über die Boxen hören willst. Faderust stellt sie selbst als USB-Soundgerät bereit, ohne zusätzliche Treiber-Apps. Einmalige Einrichtung, dafür sind Administratorrechte nötig.",
            )]
            .spacing(10);
            if !u.installed {
                col = col.push(btn(
                    if app.usb_busy {
                        "Richte ein …"
                    } else {
                        "Faderust Kabel einrichten"
                    },
                    edit(Edit::UsbSetup),
                    true,
                ));
            } else {
                let count = u.count.clamp(1, 4) as u8;
                col = col
                    .push(
                        check(u.running)
                            .label(tr("Faderust Kabel verwenden"))
                            .on_toggle(|v| edit(Edit::UsbEnable(v))),
                    )
                    .push(
                        row![
                            text("Anzahl Kabel").size(13).color(text_color()),
                            pick_list([1u8, 2, 3, 4], Some(count), |n| edit(Edit::UsbCount(n))),
                            space().width(Length::Fill),
                            btn("Entfernen", edit(Edit::UsbRemove), false),
                        ]
                        .spacing(12)
                        .align_y(Vertical::Center),
                    );
                if u.running {
                    col = col.push(
                        text(
                            u.attached
                                .iter()
                                .enumerate()
                                .map(|(i, on)| {
                                    format!(
                                        "Faderust Kabel{}: {}",
                                        if i == 0 {
                                            String::new()
                                        } else {
                                            format!(" {}", i + 1)
                                        },
                                        if *on {
                                            "verbunden"
                                        } else {
                                            "verbinde mit Windows …"
                                        }
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                        .size(12)
                        .color(muted_color()),
                    );
                }
            }
            col.into()
        }
    };
    let debug_card = {
        let mut c = column![
            check(app.prefs.debug)
                .label(tr("Messungen, Selbsttest und Protokoll einschalten"))
                .on_toggle(|v| edit(Edit::Debug(v))),
            hint("Aus spart Leistung: dann laufen keine Messungen (CPU je Thread, Kabel, Zeiten) und es werden keine Zeilen dafür geschrieben."),
        ]
        .spacing(8);
        if app.prefs.debug {
            c = c.push(
                row![
                    hint("Prüft Geräte, Puffer, Fernsteuerung und Plugin."),
                    space().width(Length::Fill),
                    btn("Starten", Message::Open(Some(Dialog::Selftest)), false),
                    btn("Protokoll", Message::Open(Some(Dialog::Log)), false),
                ]
                .spacing(8)
                .align_y(Vertical::Center),
            );
        }
        card("Debug-Modus", c)
    };
    let test_card = {
        let build = faderust_core::update::own_build();
        let mine = if build.is_empty() {
            format!(
                "{} {}",
                tr("Dieses Faderust ist ein Release:"),
                env!("CARGO_PKG_VERSION")
            )
        } else {
            let name = faderust_core::update::own_name();
            let label = if name.is_empty() {
                build.to_string()
            } else {
                format!("{name} ({build})")
            };
            format!("{} {}", tr("Dieses Faderust ist ein Test-Build:"), label)
        };
        let mut c = column![
            check(app.prefs.test_channel)
                .label(tr("Testversionen: Updates auch von Test-Builds"))
                .on_toggle(|v| edit(Edit::TestChannel(v))),
            hint("Ein Test-Build ist der Stand von main vor dem Release. Er entsteht nach jedem Merge automatisch in GitHub (Actions, „Test build“). Gesucht wird nur mit „Jetzt suchen“."),
            hint(mine),
        ]
        .spacing(8);
        if app.prefs.test_channel {
            let latest = app.test_builds.iter().find(|b| b.tag == "main");
            c = c.push(match latest {
                Some(b) => hint(format!("{} {}", tr("Neueste Testversion:"), b)),
                None => hint("Noch keine Testversion geladen: „Jetzt suchen“ fragt nach."),
            });
            let found = match &app.update {
                Some(u) => format!("{} {}", tr("Gefunden:"), u.label()),
                None => tr("Kein neueres Build gefunden.").to_string(),
            };
            let mut line = row![hint(found), space().width(Length::Fill)]
                .spacing(8)
                .align_y(Vertical::Center);
            if app.update.is_some() && !app.updating {
                line = line.push(btn("Installieren", Message::InstallUpdate, true));
            }
            c = c.push(line.push(btn("Jetzt suchen", Message::CheckTest, false)));
        }
        card("Testversionen", c)
    };
    let export_card = card(
        "Design markieren",
        column![
            hint("Macht ein Bild des Hauptfensters. Darauf ziehst du Rechtecke, schreibst Notizen und kopierst alles mit einem Klick in die Zwischenablage."),
            btn("Design markieren", edit(Edit::MarkupStart), false),
        ]
        .spacing(8),
    );
    let tab = app.devices_tab;
    let tabs = segmented(vec![
        ("Geräte".into(), tab == 0, edit(Edit::DevicesTab(0))),
        ("Verbindungen".into(), tab == 1, edit(Edit::DevicesTab(1))),
        ("Erweitert".into(), tab == 2, edit(Edit::DevicesTab(2))),
    ]);
    let content: Element<'_, Message> = match tab {
        1 => row![
            cards(
                1,
                vec![
                    card("Faderust Kabel", cable_section),
                    card("Stream Deck", deck_section(app)),
                ]
            ),
            cards(1, vec![card("Programm-Mikro", mic_section(app)),]),
        ]
        .spacing(12)
        .align_y(Vertical::Top)
        .into(),
        2 => row![
            cards(1, vec![debug_card, export_card]),
            cards(1, vec![test_card]),
        ]
        .spacing(12)
        .align_y(Vertical::Top)
        .into(),
        _ => row![cards(
            3,
            vec![
                card(
                    "Eingänge",
                    column![inputs, btn("+ Eingang", edit(Edit::InAdd), false)].spacing(10),
                ),
                card(
                    "Ausgänge",
                    column![outputs, btn("+ Ausgang", edit(Edit::OutAdd), false)].spacing(10),
                ),
            ]
        ),]
        .into(),
    };
    let body = column![error_line(app), tabs, content].spacing(12);
    frame(
        "Geräte".into(),
        1180.0,
        true,
        body.into(),
        hint("Änderungen werden automatisch gespeichert."),
    )
}

/// "Programm-Mikro": an output on a virtual cable, so Discord, Teams & co.
/// get Faderust's microphone mix by recording the cable's other end.
fn mic_section(app: &App) -> Element<'_, Message> {
    if let Some(device) = &app.mic_device {
        return column![
            text(tr("Programm-Mikro angelegt."))
                .size(13)
                .font(BOLD)
                .color(signal()),
            hint(format!(
                "{} {}. {}",
                tr("Wähle in Discord, Teams usw. als Mikrofon"),
                device_label(device),
                tr("Die Spalte „Programm-Mikro“ bestimmt, was dort ankommt.")
            )),
            check(app.mic_default)
                .label(tr("Als Windows-Mikrofon für Anrufe setzen"))
                .on_toggle(|v| edit(Edit::MicDefault(v))),
        ]
        .spacing(6)
        .into();
    }
    if app.mic_cable().is_some() {
        return row![
            btn("+ Programm-Mikro", edit(Edit::MicAdd), false),
            hint("Ausgang auf einem virtuellen Kabel, den Programme als Mikrofon nutzen"),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
        .into();
    }
    hint(if app.has_cable() {
        "Für ein Programm-Mikro (Discord, Teams …) braucht es ein freies virtuelles Kabel. Alle Kabel sind schon belegt: erhöhe die Anzahl unter „Faderust Kabel“."
    } else {
        "Für ein Programm-Mikro (Discord, Teams …) braucht es ein virtuelles Kabel."
    })
}
