//! What surrounds the window: one running instance, autostart, the tray icon
//! and the saved window preferences.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Passed by the Windows autostart entry: start in the tray, no window.
pub const MINIMIZED_ARG: &str = "--minimized";

/// Port the running instance listens on, so a second start can ask it to
/// show its window instead of opening the devices again.
const INSTANCE_PORT: u16 = 47812;

/// Becomes the running instance, or asks the running one to show itself.
/// `Err(())` means another Faderust is running and has been told to show its
/// window: this one should exit. `Ok` carries a receiver that fires when a
/// second start asked for the window.
pub fn single_instance() -> Result<Receiver<()>, ()> {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, INSTANCE_PORT)) {
        Ok(listener) => {
            let (tx, rx) = channel();
            let _ = thread::Builder::new()
                .name("faderust-instance".into())
                .spawn(move || {
                    for mut stream in listener.incoming().flatten() {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                        let mut buf = [0u8; 8];
                        if stream.read(&mut buf).is_ok_and(|n| &buf[..n] == b"show") {
                            let _ = tx.send(());
                        }
                    }
                });
            Ok(rx)
        }
        Err(_) => {
            if let Ok(mut s) = TcpStream::connect((Ipv4Addr::LOCALHOST, INSTANCE_PORT)) {
                let _ = s.write_all(b"show");
                Err(())
            } else {
                // The port is taken by something else: run anyway.
                Ok(channel().1)
            }
        }
    }
}

/// Value name of the autostart entry; the same the old Tauri version used, so an
/// update keeps working autostart.
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "faderust";

/// Whether Faderust starts with Windows.
pub fn autostart_enabled() -> bool {
    if !cfg!(windows) {
        return false;
    }
    faderust_core::proc::hidden("reg")
        .args(["query", RUN_KEY, "/v", RUN_VALUE])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Starts Faderust with Windows (in the tray) or not.
pub fn set_autostart(on: bool) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Autostart gibt es nur unter Windows".into());
    }
    let mut cmd = faderust_core::proc::hidden("reg");
    if on {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        cmd.args([
            "add",
            RUN_KEY,
            "/v",
            RUN_VALUE,
            "/t",
            "REG_SZ",
            "/d",
            &format!("\"{}\" {MINIMIZED_ARG}", exe.display()),
            "/f",
        ]);
    } else {
        cmd.args(["delete", RUN_KEY, "/v", RUN_VALUE, "/f"]);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() || !on {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// The design of a profile.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct DesignSnap {
    pub mode: String,
    pub accent: String,
    pub signal: String,
    pub lcd_color: String,
    pub part_colors: std::collections::BTreeMap<String, String>,
    pub button_look: String,
    pub meter_look: u8,
    pub density: Option<u8>,
    pub glide: Option<u8>,
    pub cap_square: bool,
    pub scale_off: bool,
    pub zero_mark: bool,
    pub glow_off: bool,
    pub accent_cap: bool,
    pub cap_value_off: bool,
    pub thin_track: bool,
    pub grip_cap: bool,
    pub strip_colors: bool,
    pub relief: bool,
    pub readout_display: bool,
    pub meter_segments: bool,
    pub meter_line: bool,
    pub one_color: bool,
    pub peak_off: bool,
    pub matrix_meters_off: bool,
    pub strip_head: bool,
    pub strip_history: bool,
    pub knobs: bool,
    pub echo_button: bool,
    pub hide_readout: bool,
    pub button_row: bool,
    pub slim_strips: bool,
    pub slim_scroll: bool,
}

impl Prefs {
    /// The design as JSON, for a profile.
    pub fn design_json(&self) -> String {
        let d = DesignSnap {
            mode: self.mode.clone(),
            accent: self.accent.clone(),
            signal: self.signal.clone(),
            lcd_color: self.lcd_color.clone(),
            part_colors: self.part_colors.clone(),
            button_look: self.button_look.clone(),
            meter_look: self.meter_look,
            density: self.density,
            glide: self.glide,
            cap_square: self.cap_square,
            scale_off: self.scale_off,
            zero_mark: self.zero_mark,
            glow_off: self.glow_off,
            accent_cap: self.accent_cap,
            cap_value_off: self.cap_value_off,
            thin_track: self.thin_track,
            grip_cap: self.grip_cap,
            strip_colors: self.strip_colors,
            relief: self.relief,
            readout_display: self.readout_display,
            meter_segments: self.meter_segments,
            meter_line: self.meter_line,
            one_color: self.one_color,
            peak_off: self.peak_off,
            matrix_meters_off: self.matrix_meters_off,
            strip_head: self.strip_head,
            strip_history: self.strip_history,
            knobs: self.knobs,
            echo_button: self.echo_button,
            hide_readout: self.hide_readout,
            button_row: self.button_row,
            slim_strips: self.slim_strips,
            slim_scroll: self.slim_scroll,
        };
        serde_json::to_string(&d).unwrap_or_default()
    }

    /// Takes a profile's design; false when it cannot be read.
    pub fn set_design(&mut self, json: &str) -> bool {
        let Ok(d) = serde_json::from_str::<DesignSnap>(json) else {
            return false;
        };
        self.mode = d.mode;
        self.accent = d.accent;
        self.signal = d.signal;
        self.lcd_color = d.lcd_color;
        self.part_colors = d.part_colors;
        self.button_look = d.button_look;
        self.meter_look = d.meter_look;
        self.density = d.density;
        self.glide = d.glide;
        self.cap_square = d.cap_square;
        self.scale_off = d.scale_off;
        self.zero_mark = d.zero_mark;
        self.glow_off = d.glow_off;
        self.accent_cap = d.accent_cap;
        self.cap_value_off = d.cap_value_off;
        self.thin_track = d.thin_track;
        self.grip_cap = d.grip_cap;
        self.strip_colors = d.strip_colors;
        self.relief = d.relief;
        self.readout_display = d.readout_display;
        self.meter_segments = d.meter_segments;
        self.meter_line = d.meter_line;
        self.one_color = d.one_color;
        self.peak_off = d.peak_off;
        self.matrix_meters_off = d.matrix_meters_off;
        self.strip_head = d.strip_head;
        self.strip_history = d.strip_history;
        self.knobs = d.knobs;
        self.echo_button = d.echo_button;
        self.hide_readout = d.hide_readout;
        self.button_row = d.button_row;
        self.slim_strips = d.slim_strips;
        self.slim_scroll = d.slim_scroll;
        true
    }

    /// Seconds the fader glide takes to settle (0 is off).
    pub fn glide_smooth(&self) -> f32 {
        match self.glide.unwrap_or(1) {
            0 => 0.0,
            2 => 0.22,
            _ => 0.08,
        }
    }

    /// Space between the strips.
    pub fn gap(&self) -> f32 {
        match self.density.unwrap_or(1) {
            0 => 4.0,
            2 => 18.0,
            _ => 10.0,
        }
    }

    /// Padding inside a strip card.
    pub fn card_pad(&self) -> [u16; 2] {
        match self.density.unwrap_or(1) {
            0 => [6, 3],
            2 => [14, 10],
            _ => [10, 6],
        }
    }
}

/// Window preferences that outlive a start (the Tauri app kept them in the
/// browser's local storage).
#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default)]
pub struct Prefs {
    /// Where the main window was last: x, y, width, height (logical px).
    pub win: Option<[f32; 4]>,
    /// The same in physical pixels as Windows has it (x, y, width, height).
    /// Logical values break when monitors have different scaling.
    pub win_px: Option<[i32; 4]>,
    /// "desk" or "matrix".
    pub view: String,
    /// Seconds of silence after which a program leaves the desk; `None` is the default.
    pub hide_after: Option<u64>,
    /// New programs are not taken up by themselves (the switch in the programs window).
    pub auto_programs_off: bool,
    /// Programs Faderust took up by itself already (lower-case names).
    pub auto_seen: Vec<String>,
    /// Outputs hidden from the desk (labels).
    pub hidden_outputs: Vec<String>,
    /// "dark", "light" or "system"; empty is dark.
    pub mode: String,
    /// Accent and signal colour as "#rrggbb"; empty is the default.
    pub accent: String,
    pub signal: String,
    /// No green edge around an output that carries signal.
    pub output_edge_off: bool,
    /// Update to the rolling test build of the repository too.
    pub test_channel: bool,
    /// Tag (the branch name) of the test build to follow.
    pub test_branch: String,
    /// Debug mode: measurements, self-test and log are shown and written.
    pub debug: bool,
    /// The same for devices and for programs on the desk.
    pub device_edge_off: bool,
    pub program_edge_off: bool,
    /// Building blocks of the look, each switched on its own.
    /// LED segments instead of the continuous meter bar.
    pub meter_segments: bool,
    /// No peak mark on the meters.
    pub peak_off: bool,
    /// A line at 0 dB across the fader slot.
    pub zero_mark: bool,
    /// The dB value in a display box under the fader.
    pub readout_display: bool,
    /// No level bars in the matrix.
    pub matrix_meters_off: bool,
    /// Square fader caps instead of round ones.
    pub cap_square: bool,
    /// No scale (numbers and marks) beside the faders.
    pub scale_off: bool,
    /// No glow around the accent bar of the fader.
    pub glow_off: bool,
    /// The meters in the signal colour only.
    pub one_color: bool,
    /// Fader caps in the accent colour.
    pub accent_cap: bool,
    /// No dB value in the fader cap.
    pub cap_value_off: bool,
    /// Thin track instead of the recessed slot.
    pub thin_track: bool,
    /// Strips as raised panels.
    pub relief: bool,
    /// Slim grip cap in the accent colour.
    pub grip_cap: bool,
    /// The meter as a thin line.
    pub meter_line: bool,
    /// Every strip in its own colour.
    pub strip_colors: bool,
    /// A symbol and a colour band in the head of each strip.
    pub strip_head: bool,
    /// The last 30 seconds of the level in the strip head.
    pub strip_history: bool,
    /// Rotary knobs with an LED ring instead of sliders in the effects.
    pub knobs: bool,
    /// Order of devices and programs on the desk (input device ids).
    pub strip_order: Vec<String>,
    /// Arrows on the strips to move them.
    pub sort_strips: bool,
    /// Meter shape: 0 bar/segments, 1 dots, 2 blocks.
    pub meter_look: u8,
    /// Spacing of the desk: 0 compact, 1 normal (default), 2 airy.
    pub density: Option<u8>,
    /// Speed of the fader glide: 0 off, 1 normal (default), 2 slow.
    pub glide: Option<u8>,
    /// Saved design profiles: name and the design as JSON.
    pub profiles: std::collections::BTreeMap<String, String>,
    /// Look of the keys: empty (standard), flat, glow, pill, neon, hardware, tile.
    pub button_look: String,
    /// An echo switch on every strip of the desk.
    pub echo_button: bool,
    /// Desk: no dB value under the fader.
    pub hide_readout: bool,
    /// Desk: mute, effects and echo in one row.
    pub button_row: bool,
    /// Desk: outputs and devices as narrow as the programs.
    pub slim_strips: bool,
    /// Desk: a thin scroll bar for the programs.
    pub slim_scroll: bool,
    /// Own colours per element (keys of `style::Part`), "#rrggbb".
    pub part_colors: std::collections::BTreeMap<String, String>,
    /// Colour of the value display as "#rrggbb"; empty is the signal colour.
    pub lcd_color: String,
    /// "de", "en" or empty for the system language.
    pub lang: String,
    /// The window stays above all others.
    pub pinned: bool,
    /// The soundboard is switched on.
    pub soundboard: bool,
    /// No on-screen note when the Stream Deck changes a level.
    pub osd_off: bool,
    /// A click on the tray icon opens the compact mixer next to it.
    pub tray_popup: bool,
}

/// "de" or "en", whichever the system uses.
pub fn system_lang() -> &'static str {
    #[cfg(windows)]
    {
        let out = faderust_core::proc::hidden("reg")
            .args([
                "query",
                r"HKCU\Control Panel\International",
                "/v",
                "LocaleName",
            ])
            .output();
        if let Ok(o) = out {
            let t = String::from_utf8_lossy(&o.stdout).to_lowercase();
            if t.contains("reg_sz") && t.contains("en-") {
                return "en";
            }
            return "de";
        }
    }
    let l = std::env::var("LANG").unwrap_or_default().to_lowercase();
    if l.starts_with("en") {
        "en"
    } else {
        "de"
    }
}

pub fn prefs_path(dir: &Path) -> PathBuf {
    dir.join("ui.json")
}

pub fn load_prefs(dir: &Path) -> Prefs {
    std::fs::read_to_string(prefs_path(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_prefs(dir: &Path, prefs: &Prefs) {
    if let Ok(t) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(prefs_path(dir), t);
    }
}

/// Shared with the audio thread so a hidden window gets no snapshots.
pub type Visible = Arc<AtomicBool>;

pub fn is_visible(v: &Visible) -> bool {
    v.load(Ordering::Relaxed)
}

#[cfg(windows)]
pub mod tray {
    //! Tray icon with "Anzeigen" and "Beenden"; a left click shows the window.

    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    pub struct Tray {
        _icon: TrayIcon,
        show: tray_icon::menu::MenuId,
        quit: tray_icon::menu::MenuId,
    }

    pub enum Action {
        Show,
        /// Left click on the icon.
        Click,
        Quit,
    }

    impl Tray {
        /// `rgba` is the icon as RGBA pixels of `size` × `size`.
        pub fn new(rgba: Vec<u8>, size: u32) -> Option<Tray> {
            let menu = Menu::new();
            let show = MenuItem::new("Anzeigen", true, None);
            let quit = MenuItem::new("Beenden", true, None);
            menu.append_items(&[&show, &quit]).ok()?;
            let icon = TrayIconBuilder::new()
                .with_tooltip("Faderust")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_icon(Icon::from_rgba(rgba, size, size).ok()?)
                .build()
                .ok()?;
            Some(Tray {
                _icon: icon,
                show: show.id().clone(),
                quit: quit.id().clone(),
            })
        }

        /// What the user did with the icon since the last call.
        pub fn poll(&self) -> Vec<Action> {
            let mut out = vec![];
            while let Ok(e) = MenuEvent::receiver().try_recv() {
                if e.id == self.show {
                    out.push(Action::Show);
                } else if e.id == self.quit {
                    out.push(Action::Quit);
                }
            }
            while let Ok(e) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = e
                {
                    out.push(Action::Click);
                }
            }
            out
        }
    }
}
