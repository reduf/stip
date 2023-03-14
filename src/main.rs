use clap::Parser;
use image::{self, ImageFormat};
use std::io::Cursor;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path on disk if the file to be signed.
    #[clap(value_name = "file", index = 1)]
    input: String,

    /// Optional password if the file is stored in a encrypted zip.
    #[clap(short, long, value_name = "password")]
    password: Option<String>,
}

fn main() {
    let args = Args::parse();

    let format = ImageFormat::from_path(args.input.as_str())
        .expect("Can't infer the image format from the path");

    let password = args.password.as_deref().map(|inner| inner.as_bytes());
    let input_bytes = vault::open(&args.input, password).expect("Can't read input");
    let img = image::load(Cursor::new(input_bytes), format)
        .map_err(|_| {
            println!("Couldn't read the file '{}'", args.input);
            std::process::exit(1);
        })
        .unwrap()
        .to_luma8();
    // Prepare for detection
    let mut img = rqrr::PreparedImage::prepare(img);
    // Search for grids, without decoding
    let grids = img.detect_grids();
    assert_eq!(grids.len(), 1);
    // Decode the grid
    let (_meta, content) = grids[0].decode().unwrap();

    let parsed = otpauth::ParsedUrl::parse(&content).unwrap();
    let number = otpauth::totp::from_now(parsed.secret.as_slice(), 6);
    println!("{:06}", number);
}
