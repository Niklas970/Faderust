//! Clipboard with a picture (Windows). Elsewhere it answers that it is not
//! available.

/// Puts the picture (`rgba`, `w` × `h`) and a text on the clipboard.
#[cfg(windows)]
pub fn set_image_and_text(w: u32, h: u32, rgba: &[u8], text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_UNICODETEXT: u32 = 13;
    const CF_DIB: u32 = 8;

    /// A global memory block holding `data`, ready for the clipboard.
    unsafe fn block(data: &[u8]) -> Result<windows::Win32::Foundation::HGLOBAL, String> {
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE, data.len()).map_err(|e| e.to_string())?;
            let p = GlobalLock(h) as *mut u8;
            if p.is_null() {
                return Err("Zwischenablage: Speicher nicht gesperrt".into());
            }
            std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
            let _ = GlobalUnlock(h);
            Ok(h)
        }
    }

    let dib = dib(w, h, rgba)?;
    let mut utf16: Vec<u8> = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect();
    utf16.shrink_to_fit();
    unsafe {
        OpenClipboard(None).map_err(|e| e.to_string())?;
        let result = (|| {
            EmptyClipboard().map_err(|e| e.to_string())?;
            let img = block(&dib)?;
            SetClipboardData(CF_DIB, Some(HANDLE(img.0))).map_err(|e| e.to_string())?;
            let txt = block(&utf16)?;
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(txt.0))).map_err(|e| e.to_string())?;
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}

#[cfg(not(windows))]
pub fn set_image_and_text(_: u32, _: u32, _: &[u8], _: &str) -> Result<(), String> {
    Err("Zwischenablage mit Bild gibt es nur unter Windows".into())
}

/// A device independent bitmap (what `CF_DIB` holds): 40-byte header, then
/// the rows bottom-up as BGRA.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn dib(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let (wu, hu) = (w as usize, h as usize);
    if wu == 0 || hu == 0 || rgba.len() != wu * hu * 4 {
        return Err("Bild hat die falsche Größe".into());
    }
    let mut out = Vec::with_capacity(40 + rgba.len());
    out.extend_from_slice(&40u32.to_le_bytes()); // header size
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes()); // positive: bottom-up
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(rgba.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // resolution, colours
    for row in rgba.chunks_exact(wu * 4).rev() {
        for px in row.chunks_exact(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dib_is_bottom_up_bgra() {
        // 1 × 2: red above blue.
        let d = dib(1, 2, &[255, 0, 0, 255, 0, 0, 255, 255]).unwrap();
        assert_eq!(d.len(), 40 + 8);
        assert_eq!(&d[0..4], &40u32.to_le_bytes());
        assert_eq!(&d[40..44], &[255, 0, 0, 255]); // blue row first
        assert_eq!(&d[44..48], &[0, 0, 255, 255]); // then red
        assert!(dib(2, 2, &[0; 4]).is_err());
    }
}
