use std::io;

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src/stb_image.c");
    cc::Build::new()
        .file("src/stb_image.c")
        .define("STB_IMAGE_IMPLEMENTATION", None)
        .compile("stb_image");
    return Ok(());
}
