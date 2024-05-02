#![allow(clippy::needless_return)]

use std::io::{Cursor, Read};
use std::path::{PathBuf, Path};
use zip::result::InvalidPassword;

use crate::{otpauth, stb_image};

#[derive(Debug)]
pub struct Error;

pub struct Vault {
    content: Vec<u8>,
}

pub struct VaultSecret {
    pub filename: String,
    pub name: String,
    pub secret: Option<Box<[u8]>>,
    encrypted: bool,
}

impl VaultSecret {
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read(path).map_err(|err| {
            eprintln!("Failed to open {:?}, err: {}", path, err);
            return Error;
        })?;

        let mut new_secret = VaultSecret {
            filename: path.to_str().ok_or_else(|| {
                eprintln!("Failed to convert path {:?} to utf8 string", path);
                return Error;
            })?.to_string(),
            name: String::from(""),
            secret: None,
            encrypted: false,
        };

        read_secret(&mut new_secret, content.as_slice())?;
        return Ok(new_secret);
    }
}

impl Vault {
    pub fn open(path: PathBuf) -> Result<Self, Error> {
        let content = std::fs::read(path.as_path()).map_err(|err| {
            eprintln!("Failed to open {:?}, err: {}", path.as_path(), err);
            return Error;
        })?;
        return Ok(Self { content });
    }

    pub fn requires_password(&self) -> bool {
        let reader = Cursor::new(self.content.as_slice());
        let mut zip = zip::ZipArchive::new(reader).unwrap();
        for file_number in 0..zip.len() {
            if let Ok(file) = zip.by_index_raw(file_number) {
                if file.encrypted() {
                    return true;
                }
            } else {
                eprintln!("Failed to read file #{}", file_number);
            }
        }
        return false;
    }

    pub fn list(&self, password: Option<&str>) -> Result<Vec<VaultSecret>, Error> {
        let reader = Cursor::new(self.content.as_slice());
        let mut zip = zip::ZipArchive::new(reader).unwrap();
        let mut results = Vec::new();
        for file_number in 0..zip.len() {
            let mut new_secret = if let Ok(file) = zip.by_index_raw(file_number) {
                let encrypted = file.encrypted();
                VaultSecret {
                    filename: file.name().to_string(),
                    name: String::from(""),
                    secret: None,
                    encrypted,
                }
            } else {
                eprintln!("Failed to read file #{}", file_number);
                continue;
            };

            let result = if new_secret.encrypted {
                if let Some(password) = password {
                    zip.by_index_decrypt(file_number, password.as_bytes())
                } else {
                    Ok(Err(InvalidPassword))
                }
            } else {
                zip.by_index(file_number).map(Ok)
            };

            match result {
                Err(err) => {
                    eprintln!("Error reading file '{}', error: {}", new_secret.filename, err);
                    continue;
                }
                Ok(Err(InvalidPassword)) => {
                    return Err(Error);
                }
                Ok(Ok(mut file)) => {
                    let mut buffer = Vec::with_capacity(file.size() as usize);
                    file.read_to_end(&mut buffer).map_err(|_| Error)?;
                    if read_secret(&mut new_secret, buffer.as_slice()).is_ok() {
                        results.push(new_secret);
                    }
                }
            }
        }

        return Ok(results);
    }
}

fn read_secret(output: &mut VaultSecret, image_bytes: &[u8]) -> Result<(), Error> {
    let img = stb_image::load_bytes(image_bytes).map_err(|err| {
        eprintln!("Couldn't read the image '{}', error: {}", output.filename, err);
        return Error;
    })?;

    let mut img = rqrr::PreparedImage::prepare_from_greyscale(img.width, img.height, |x, y| {
        return img.data()[(y * img.width) + x];
    });

    if let Some(grid) = img.detect_grids().first() {
        let content = grid
            .decode()
            .map_err(|err| {
                eprintln!("Failed to decode the QR code of '{}', error: {}", output.filename, err);
                return Error;
            })?
            .1;

        let parsed = otpauth::ParsedUrl::parse(&content).map_err(|err| {
            eprintln!("Failed to parse URL found in QR code of '{}', error: {:?}", output.filename, err);
            return Error;
        })?;

        output.secret = Some(parsed.secret.into_boxed_slice());
        output.name = format!("{}: {}", parsed.issuer, parsed.account_name);
        return Ok(());
    } else {
        eprintln!("Failed to detect the QR code of '{}'", output.filename);
        return Err(Error);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    const FILE_CONTENT: &[u8] = b"file content";

    fn resource(suffix: &str) -> String {
        let mut result = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        result.push("tests");
        result.push("vault");
        result.push(suffix);
        return result.into_os_string().into_string().unwrap();
    }

    #[test]
    fn test_file_on_disk() {
        assert_eq!(
            super::open(&resource("file.txt"), None).unwrap().as_slice(),
            FILE_CONTENT
        );
        assert_eq!(
            super::open(&resource("subdir/file.txt"), None)
                .unwrap()
                .as_slice(),
            FILE_CONTENT
        );
    }

    #[test]
    fn test_file_in_decrypted_zip() {
        assert_eq!(
            super::open(&resource("decrypted.zip/file.txt"), None)
                .unwrap()
                .as_slice(),
            FILE_CONTENT
        );
        assert_eq!(
            super::open(&resource("decrypted.zip/subdir/file.txt"), None)
                .unwrap()
                .as_slice(),
            FILE_CONTENT
        );
    }

    #[test]
    fn test_file_in_encrypted_zip() {
        let password = Some(b"password".as_ref());
        assert_eq!(
            super::open(&resource("decrypted.zip/file.txt"), password)
                .unwrap()
                .as_slice(),
            FILE_CONTENT
        );
        assert_eq!(
            super::open(&resource("decrypted.zip/subdir/file.txt"), password)
                .unwrap()
                .as_slice(),
            FILE_CONTENT
        );
    }
}
