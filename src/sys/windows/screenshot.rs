use core::ffi::c_void;

use windows::Win32::{
    UI::WindowsAndMessaging::{
        SM_CXSCREEN,
        SM_CYSCREEN,
        GetSystemMetrics,
    },
    Graphics::Gdi::{
        HDC,
        GetDC,
        ReleaseDC,
        DeleteDC,
        BitBlt,
        SelectObject,
        BITMAPINFO,
        BITMAPINFOHEADER,
        BI_RGB,
        RGBQUAD,
        CreateDIBSection,
        CreateCompatibleDC,
        DIB_RGB_COLORS,
        SRCCOPY,
    },
    Foundation::HWND,
};

fn get_primary_screen_dimensions() -> Result<(i32, i32), windows::core::Error> {
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    if width == 0 {
        return Err(windows::core::Error::from_win32());
    }
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if height == 0 {
        return Err(windows::core::Error::from_win32());
    }
    return Ok((width, height));
}

struct ScreenDeviceContext(Option<HWND>, HDC);

impl std::ops::Drop for ScreenDeviceContext {
    fn drop(&mut self) {
        if !self.1.is_invalid() {
            unsafe { ReleaseDC(self.0, self.1) };
            self.0 = None;
            self.1 = HDC::default();
        }
    }
}

struct DeviceContext(HDC);

impl std::ops::Drop for DeviceContext {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { let _ = DeleteDC(self.0); };
        }
    }
}

fn get_screen_dc() -> Result<ScreenDeviceContext, windows::core::Error> {
    let dc = unsafe { GetDC(None) };
    if dc.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    return Ok(ScreenDeviceContext(None, dc));
}

fn create_compatible_dc(dc: HDC) -> Result<DeviceContext, windows::core::Error> {
    let new_dc = unsafe { CreateCompatibleDC(Some(dc)) };
    if new_dc.is_invalid() {
        return Err(windows::core::Error::from_win32());
    }
    return Ok(DeviceContext(new_dc));
}

pub fn capture_screen() -> Result<(usize, usize, Vec<u8>), ()> {
    let src_dc = get_screen_dc().map_err(|err| {
        eprintln!("get_screen_dc failed, err: {:?}", err);
    })?;
    let dst_dc = create_compatible_dc(src_dc.1).map_err(|err| {
        eprintln!("create_compatible_dc failed, err: {:?}", err);
        return ();
    })?;

    let bytes_per_pixel = 4;
    let (width, height) = get_primary_screen_dimensions().map_err(|err| {
        eprintln!("get_primary_screen_dimensions failed, err: {:?}", err);
        return ();
    })?;

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // If biHeight is negative, the bitmap is a top-down DIB with the origin at the upper left corner.
            biPlanes: 1,
            biBitCount: bytes_per_pixel * 8,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default()],
    };

    let mut pixels_ptr: *mut c_void = std::ptr::null_mut();
    let bitmap = unsafe {
        let ppvbits: *mut *mut c_void = &mut pixels_ptr as *mut *mut c_void;
        CreateDIBSection(Some(dst_dc.0), &bmi, DIB_RGB_COLORS, ppvbits, None, 0)
    }.map_err(|err| {
        eprintln!("CreateDIBSection failed, err: {:?}", err);
        return ();
    })?;

    unsafe { SelectObject(dst_dc.0, bitmap.into()) };
    unsafe { BitBlt(dst_dc.0, 0, 0, width, height, Some(src_dc.1), 0, 0, SRCCOPY) }.map_err(|err| {
        eprintln!("BitBlt failed, err: {:?}", err);
        return ();
    })?;

    let width = width as usize;
    let height = height as usize;
    let bytes_per_pixel = bytes_per_pixel as usize;

    let pixels_ptr = pixels_ptr as *const u8;
    let pixel_bytes = unsafe { std::slice::from_raw_parts(pixels_ptr, bytes_per_pixel * width * height) };
    let mut results = Vec::with_capacity(width * height);
    for bgra in pixel_bytes.chunks_exact(4) {
        let r = bgra[2] as f32;
        let g = bgra[1] as f32;
        let b = bgra[0] as f32;
        let grey = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
        results.push(grey);
    }

    return Ok((width, height, results));
}
