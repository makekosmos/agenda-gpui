use super::*;

/// Grab the card's on-screen region as pixels (Windows: BitBlt from the
/// screen DC; the overlay is always in the foreground when this runs).
#[cfg(target_os = "windows")]
pub(super) fn capture_card(
    window: &Window,
    bounds: gpui::Bounds<gpui::Pixels>,
) -> Option<image::RgbaImage> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{HWND, POINT};
    use windows::Win32::Graphics::Gdi::*;

    let raw = HasWindowHandle::window_handle(window).ok()?.as_raw();
    let hwnd = match raw {
        RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
        _ => return None,
    };
    let scale = window.scale_factor();

    unsafe {
        let mut pt = POINT::default();
        if !ClientToScreen(hwnd, &mut pt).as_bool() {
            return None;
        }
        let x = pt.x + (f32::from(bounds.origin.x) * scale).round() as i32;
        let y = pt.y + (f32::from(bounds.origin.y) * scale).round() as i32;
        let w = (f32::from(bounds.size.width) * scale).round() as i32;
        let h = (f32::from(bounds.size.height) * scale).round() as i32;
        if w <= 0 || h <= 0 {
            return None;
        }

        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        let _ = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY);

        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h; // top-down rows
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let got = GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, screen);
        if got == 0 {
            return None;
        }

        for p in buf.as_chunks_mut::<4>().0 {
            p.swap(0, 2); // BGRA -> RGBA
            p[3] = 255;
        }
        image::RgbaImage::from_raw(w as u32, h as u32, buf)
    }
}

/// Copy the card image to the Windows clipboard in a single open session:
/// registered "PNG" format + classic bottom-up CF_DIB (for apps that don't
/// understand PNG clipboard data).
#[cfg(target_os = "windows")]
pub(super) fn write_card_to_clipboard(img: &image::RgbaImage) -> bool {
    use windows::core::s;
    use windows::Win32::Foundation::{GlobalFree, HANDLE};
    use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatA, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_DIB: u32 = 8;
    let (w, h) = (img.width() as usize, img.height() as usize);

    let mut png = Vec::new();
    if image::DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .is_err()
    {
        return false;
    }

    // Bottom-up BGRA DIB: BITMAPINFOHEADER + pixel data.
    let header_len = std::mem::size_of::<BITMAPINFOHEADER>();
    let data_len = w * h * 4;
    let mut dib = vec![0u8; header_len + data_len];
    unsafe {
        (dib.as_mut_ptr() as *mut BITMAPINFOHEADER).write(BITMAPINFOHEADER {
            biSize: header_len as u32,
            biWidth: w as i32,
            biHeight: h as i32, // positive = bottom-up rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            biSizeImage: data_len as u32,
            ..Default::default()
        });
    }
    let raw = img.as_raw();
    for row in 0..h {
        let src = &raw[(h - 1 - row) * w * 4..(h - row) * w * 4];
        let dst = &mut dib[header_len + row * w * 4..header_len + (row + 1) * w * 4];
        for (i, px) in src.as_chunks::<4>().0.iter().enumerate() {
            dst[i * 4] = px[2];
            dst[i * 4 + 1] = px[1];
            dst[i * 4 + 2] = px[0];
            dst[i * 4 + 3] = 255;
        }
    }

    /// Move `data` onto the clipboard under `format`; false on failure.
    unsafe fn put(format: u32, data: &[u8]) -> bool {
        let Ok(hglob) = GlobalAlloc(GMEM_MOVEABLE, data.len()) else {
            return false;
        };
        let ptr = GlobalLock(hglob).cast::<u8>();
        if ptr.is_null() {
            let _ = GlobalFree(Some(hglob));
            return false;
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        let _ = GlobalUnlock(hglob);
        if SetClipboardData(format, Some(HANDLE(hglob.0 as *mut _))).is_err() {
            let _ = GlobalFree(Some(hglob));
            return false;
        }
        true
    }

    unsafe {
        // Clipboard contention is transient — retry briefly.
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(None).is_ok() {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !opened {
            eprintln!("[share] OpenClipboard failed");
            return false;
        }
        let _ = EmptyClipboard();
        let png_fmt = RegisterClipboardFormatA(s!("PNG"));
        let ok_dib = put(CF_DIB, &dib);
        let ok_png = put(png_fmt, &png);
        eprintln!("[share] clipboard: dib={ok_dib} png={ok_png}");
        let _ = CloseClipboard();
        ok_dib || ok_png
    }
}

#[cfg(not(target_os = "windows"))]
pub(super) fn capture_card(
    _window: &Window,
    _bounds: gpui::Bounds<gpui::Pixels>,
) -> Option<image::RgbaImage> {
    None
}

#[cfg(not(target_os = "windows"))]
pub(super) fn write_card_to_clipboard(_img: &image::RgbaImage) -> bool {
    false
}
