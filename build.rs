use std::io;
use winres::WindowsResource;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src/stb_image.c");
    cc::Build::new()
        .file("src/stb_image.c")
        .define("STB_IMAGE_IMPLEMENTATION", None)
        .compile("stb_image");

    #[cfg(windows)] {
    WindowsResource::new()
        // This path can be absolute, or relative to your crate root.
        .set_icon("assets/shield-96.ico")
        .compile()?;
    }

    return Ok(());
}
