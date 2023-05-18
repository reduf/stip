use clap::Parser;

mod base32;
mod otpauth;
mod sha1;
mod stb_image;
mod totp;
mod vault;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path on disk if the file to be signed.
    #[clap(value_name = "file", index = 1)]
    input: String,

    /// Optional password if the file is stored in a encrypted zip.
    #[clap(short, long, value_name = "password")]
    password: Option<Option<String>>,

    /// When in interactive mode, stip will list available files from the zip file.
    #[clap(short, long)]
    interactive: bool,

    /// Print the output in a JSON serialized format.
    #[clap(long)]
    json: bool,
}

fn main() {
    let args = Args::parse();

    let print_as_json = args.json;
    match run(args) {
        Err(message) => {
            eprintln!("{}", message);
            std::process::exit(1);
        }
        Ok(token) => {
            if print_as_json {
                println!("{}", serde_json::to_string_pretty(&token).expect("Can't serialize the structure to JSON"));
            } else {
                println!("{:06} - Valid for {:.0?}", token.number, token.remaining_duration());
            }
        }
    };
}

fn run(args: Args) -> Result<otpauth::TotpToken, String> {
    // First check if "-p" or "--password" was specified.
    // When "-p" is specified, and there is still no value, simply prompt for it.
    let password = args.password.map(|password| {
        return password.unwrap_or_else(|| {
            return rpassword::prompt_password("Enter password: ")
                .expect("Failed to read user password");
        });
    });

    let input_bytes = if args.interactive {
        let input_bytes = vault::interactive(&args.input, password)
            .map_err(|_| String::from("Couldn't select and read an image interactively."))?;
        input_bytes
    } else {
        // Convert `Option<String>` to `Option<&str>` to `Option<&[u8]>`.
        let password = password.as_deref().map(|inner| inner.as_bytes());
        let input_bytes = vault::open(&args.input, password)
            .map_err(|_| format!("Can't read input '{}'.", args.input))?;
        input_bytes
    };

    let img = stb_image::load_bytes(input_bytes.as_slice())
        .map_err(|_| format!("Couldn't read the image '{}'.", args.input))?;

    let mut img = rqrr::PreparedImage::prepare_from_greyscale(img.width, img.height, |x, y| {
        return img.data()[(y * img.width) + x];
    });

    if let Some(grid) = img.detect_grids().first() {
        let content = grid
            .decode()
            .map_err(|_| String::from("Failed to decode the QR code."))?
            .1;
        let parsed = otpauth::ParsedUrl::parse(&content)
            .map_err(|_| String::from("Failed to parse URL found in QR code."))?;
        let token = totp::from_now(parsed.secret.as_slice(), 6);
        return Ok(token);
    } else {
        return Err(String::from("Failed to detect the QR code."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(suffix: &str) -> String {
        let mut result = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        result.push("tests");
        result.push("data");
        result.push(suffix);
        return result.into_os_string().into_string().unwrap();
    }

    fn args(input: &str) -> Args {
        return Args::parse_from([String::from("stip.exe"), resource(input)]);
    }

    #[test]
    fn load_different_image_formats() {
        run(args("noreply.example.jpg")).unwrap();
        run(args("noreply.example.png")).unwrap();
    }
}
