//! A small note at the bottom of the screen when a remote control (Stream
//! Deck) changes a level or mutes something. It does not take focus, lets
//! clicks through and fades after a moment. Windows only; elsewhere it does
//! nothing.

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use std::sync::{Mutex, Once};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    const WIDTH: i32 = 260;
    const HEIGHT: i32 = 88;
    const SHOW_MS: u32 = 1400;
    const TIMER: usize = 1;
    const SHOW: u32 = WM_APP + 1;

    /// Label, text and the level (dB) of the bar, if any.
    type Note = (String, String, Option<f32>);
    static TEXT: Mutex<Note> = Mutex::new((String::new(), String::new(), None));
    static WINDOW: AtomicIsize = AtomicIsize::new(0);
    static PENDING: AtomicBool = AtomicBool::new(false);
    static START: Once = Once::new();

    pub fn show(label: &str, text: &str, level: Option<f32>) {
        *TEXT.lock().unwrap_or_else(|e| e.into_inner()) =
            (label.to_string(), text.to_string(), level);
        PENDING.store(true, Ordering::Release);
        START.call_once(|| {
            let _ = std::thread::Builder::new()
                .name("faderust-osd".into())
                .spawn(run);
        });
        let hwnd = WINDOW.load(Ordering::Acquire);
        if hwnd != 0 {
            // SAFETY: posting to a window of the OSD thread; a stale handle only fails.
            unsafe {
                let _ = PostMessageW(Some(HWND(hwnd as *mut _)), SHOW, WPARAM(0), LPARAM(0));
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn run() {
        // SAFETY: plain Win32 window on its own thread with its own message loop.
        unsafe {
            let Ok(module) = GetModuleHandleW(None) else {
                return;
            };
            let class = w!("FaderustOsd");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: module.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassW(&wc);
            let hwnd = match CreateWindowExW(
                WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_LAYERED
                    | WS_EX_TRANSPARENT,
                class,
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                WIDTH,
                HEIGHT,
                None,
                None,
                Some(module.into()),
                None,
            ) {
                Ok(h) => h,
                Err(_) => return,
            };
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 235, LWA_ALPHA);
            WINDOW.store(hwnd.0 as isize, Ordering::Release);
            if PENDING.load(Ordering::Acquire) {
                let _ = PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0));
            }
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            SHOW => {
                PENDING.store(false, Ordering::Release);
                // SAFETY: on the window's own thread.
                unsafe {
                    // Bottom centre of the screen, above the taskbar.
                    let sw = GetSystemMetrics(SM_CXSCREEN);
                    let sh = GetSystemMetrics(SM_CYSCREEN);
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        (sw - WIDTH) / 2,
                        sh - HEIGHT - 90,
                        WIDTH,
                        HEIGHT,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                    let _ = InvalidateRect(Some(hwnd), None, true);
                    SetTimer(Some(hwnd), TIMER, SHOW_MS, None);
                }
                LRESULT(0)
            }
            WM_TIMER => {
                // SAFETY: on the window's own thread.
                unsafe {
                    let _ = KillTimer(Some(hwnd), TIMER);
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                // SAFETY: standard paint cycle on the window's own thread.
                unsafe {
                    let mut ps = PAINTSTRUCT::default();
                    let dc = BeginPaint(hwnd, &mut ps);
                    paint(dc);
                    let _ = EndPaint(hwnd, &ps);
                }
                LRESULT(0)
            }
            // SAFETY: default handling.
            _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
        }
    }

    unsafe fn paint(dc: HDC) {
        let (label, text, level) = TEXT.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let rect = RECT {
            left: 0,
            top: 0,
            right: WIDTH,
            bottom: HEIGHT,
        };
        // SAFETY: GDI objects are created and released here.
        unsafe {
            let bg = CreateSolidBrush(COLORREF(0x000000));
            FillRect(dc, &rect, bg);
            let _ = DeleteObject(bg.into());
            let edge = CreateSolidBrush(COLORREF(0x76e600)); // #00e676 as BGR
            FrameRect(dc, &rect, edge);
            let _ = DeleteObject(edge.into());
            SetBkMode(dc, TRANSPARENT);
            let font = |h: i32, weight: i32| {
                CreateFontW(
                    h,
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    0,
                    w!("Segoe UI"),
                )
            };
            let small = font(-15, 400);
            let big = font(-30, 600);
            let old = SelectObject(dc, small.into());
            SetTextColor(dc, COLORREF(0xa0a0a0));
            let mut r = RECT {
                left: 12,
                top: 8,
                right: WIDTH - 12,
                bottom: 30,
            };
            let mut l = wide(&label);
            DrawTextW(
                dc,
                &mut l,
                &mut r,
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
            SelectObject(dc, big.into());
            SetTextColor(dc, COLORREF(0x76e600));
            let mut r = RECT {
                left: 12,
                top: 30,
                right: WIDTH - 12,
                bottom: HEIGHT - 20,
            };
            let mut t = wide(&text);
            DrawTextW(dc, &mut t, &mut r, DT_LEFT | DT_SINGLELINE | DT_NOPREFIX);
            SelectObject(dc, old);
            // The level as a bar: -60 dB empty, +24 dB full.
            if let Some(db) = level {
                let (left, right, top, bottom) = (12, WIDTH - 12, HEIGHT - 16, HEIGHT - 10);
                let track = CreateSolidBrush(COLORREF(0x303030));
                FillRect(
                    dc,
                    &RECT {
                        left,
                        top,
                        right,
                        bottom,
                    },
                    track,
                );
                let _ = DeleteObject(track.into());
                let fraction = ((db + 60.0) / 84.0).clamp(0.0, 1.0);
                let fill_to = left + ((right - left) as f32 * fraction) as i32;
                if fill_to > left {
                    let fill = CreateSolidBrush(COLORREF(0x76e600));
                    FillRect(
                        dc,
                        &RECT {
                            left,
                            top,
                            right: fill_to,
                            bottom,
                        },
                        fill,
                    );
                    let _ = DeleteObject(fill.into());
                }
            }
            let _ = DeleteObject(small.into());
            let _ = DeleteObject(big.into());
        }
    }
}

static ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Turns the note on or off (a setting in the Design dialog).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Shows `label` and `text` for a moment, with a bar for `level` (dB).
pub fn show(label: &str, text: &str, level: Option<f32>) {
    if !ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    #[cfg(windows)]
    imp::show(label, text, level);
    #[cfg(not(windows))]
    let _ = (label, text, level);
}
