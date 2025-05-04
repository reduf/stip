use std::os::raw::{c_int, c_void};

extern "C" {
    pub fn stbi_load_from_memory(
        buffer: *const u8,
        len: c_int,
        x: *mut c_int,
        y: *mut c_int,
        channels_in_file: *mut c_int,
        desired_channels: c_int,
    ) -> *mut u8;

    pub fn free(ptr: *mut c_void);
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Channel {
    Default = 0,
    Grey = 1,
    _GreyAlpha = 2,
    _Rgb = 3,
    Rgba = 4,
}


pub struct Image {
    pub width: usize,
    pub height: usize,
    pub bytes_per_pixel: usize,
    data: *mut u8,
}

impl Image {
    pub fn data(&self) -> &[u8] {
        let len = self.width * self.height * self.bytes_per_pixel;
        return unsafe { std::slice::from_raw_parts(self.data, len) };
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if self.data != std::ptr::null_mut() {
            unsafe { free(self.data as *mut c_void) };
            self.data = std::ptr::null_mut();
        }
    }
}

pub fn load_from_memory(bytes: &[u8], channel: Channel) -> Result<Image, &'static str> {
    let mut width = 0;
    let mut height = 0;
    let mut channels_in_file = 0;
    let image = unsafe {
        stbi_load_from_memory(
            bytes.as_ptr(),
            bytes.len() as c_int,
            &mut width,
            &mut height,
            &mut channels_in_file,
            channel as i32,
        )
    };

    let bytes_per_pixel = if channel == Channel::Default {
        channels_in_file as usize
    } else {
        channel as usize
    };

    return Ok(Image {
        width: width as usize,
        height: height as usize,
        bytes_per_pixel,
        data: image,
    });
}
