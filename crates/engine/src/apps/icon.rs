//! The icon of a running program, as a PNG for the Stream Deck plugin.

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

use super::wasapi::exe_path;

/// The large icon (usually 32 x 32) of the executable `pid` runs, as a PNG.
pub fn process_icon_png(pid: u32) -> Option<Vec<u8>> {
    let path = exe_path(pid)?;
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let mut info = SHFILEINFOW::default();
    let found = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if found == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let rgba = unsafe { icon_rgba(&info) };
    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }
    let (width, height, pixels) = rgba?;
    Some(super::png::encode_rgba(width, height, &pixels))
}

/// Width, height and RGBA pixels of the icon in `info`.
///
/// # Safety
/// `info.hIcon` must be a valid icon handle.
unsafe fn icon_rgba(info: &SHFILEINFOW) -> Option<(u32, u32, Vec<u8>)> {
    let mut icon = ICONINFO::default();
    unsafe { GetIconInfo(info.hIcon, &mut icon) }.ok()?;
    let mut size = BITMAP::default();
    let got = unsafe {
        GetObjectW(
            icon.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut size as *mut BITMAP as *mut _),
        )
    };
    let (width, height) = (size.bmWidth, size.bmHeight);
    let mut out = None;
    if got != 0 && width > 0 && height > 0 && width <= 256 && height <= 256 {
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative: rows from the top.
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let hdc = unsafe { GetDC(None) };
        let rows = unsafe {
            GetDIBits(
                hdc,
                icon.hbmColor,
                0,
                height as u32,
                Some(pixels.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            )
        };
        unsafe { ReleaseDC(None, hdc) };
        if rows != 0 {
            // BGRA to RGBA. Icons without an alpha channel come back with
            // alpha 0 everywhere: show them opaque.
            let opaque = pixels.chunks_exact(4).all(|p| p[3] == 0);
            for p in pixels.chunks_exact_mut(4) {
                p.swap(0, 2);
                if opaque {
                    p[3] = 255;
                }
            }
            out = Some((width as u32, height as u32, pixels));
        }
    }
    unsafe {
        let _ = DeleteObject(icon.hbmColor.into());
        let _ = DeleteObject(icon.hbmMask.into());
    }
    out
}
