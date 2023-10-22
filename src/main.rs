use clap::Parser;

mod base32;
mod otpauth;
mod sha1;
mod stb_image;
mod totp;
mod vault;
mod app;
mod password;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path on disk if the file to be signed.
    #[clap(value_name = "file", index = 1)]
    input: Option<String>,

    /// Optional password if the file is stored in a encrypted zip.
    #[clap(short, long, value_name = "password")]
    password: Option<Option<String>>,

    /// When in interactive mode, stip will list available files from the zip file.
    #[clap(short, long)]
    interactive: bool,
}

fn main() {
    let args = Args::parse();

    // First check if "-p" or "--password" was specified.
    // When "-p" is specified, and there is still no value, simply prompt for it.
    let _ = args.password.map(|password| {
        return password.unwrap_or_else(|| {
            return rpassword::prompt_password("Enter password: ")
                .expect("Failed to read user password");
        });
    });

    if app::build(args.input.as_deref()).is_err() {
        eprintln!("Failed to open input '{:?}'", args.input);
    }
}
