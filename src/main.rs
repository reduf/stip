use clap::Parser;
use image;
use otpauth;
use rqrr;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path on disk if the file to be signed.
    #[clap(value_name = "file", index = 1)]
    input: String,
}

fn main() {
    let args = Args::parse();

    let img = image::open(&args.input).map_err(|_| {
        println!("Couldn't read the file '{}'", args.input);
        std::process::exit(1);
    }).unwrap().to_luma8();
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
