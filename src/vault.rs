#![allow(clippy::needless_return)]

use std::path::{PathBuf, Path};
use keepass::{
    db::NodeRef,
    Database,
    DatabaseKey,
};

use crate::{otpauth, stb_image};

#[derive(Debug)]
pub struct Error;

pub struct Vault {
    content: Vec<u8>,
}

pub struct VaultSecret {
    pub name: String,
    pub secret: Box<[u8]>,
}

impl VaultSecret {
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read(path).map_err(|err| {
            eprintln!("Failed to open {:?}, err: {}", path, err);
            return Error;
        })?;

        let img = stb_image::load_from_memory(content.as_slice(), stb_image::Channel::Grey).map_err(|err| {
            eprintln!("Couldn't read the image {:?}, error: {}", path, err);
            return Error;
        })?;

        let mut img = rqrr::PreparedImage::prepare_from_greyscale(img.width, img.height, |x, y| {
            return img.data()[(y * img.width) + x];
        });

        if let Some(grid) = img.detect_grids().first() {
            let content = grid
                .decode()
                .map_err(|err| {
                    eprintln!("Failed to decode the QR code of {:?}, error: {}", path, err);
                    return Error;
                })?
                .1;

            let parsed = otpauth::ParsedUrl::parse(&content).map_err(|err| {
                eprintln!("Failed to parse URL found in QR code of {:?}, error: {:?}", path, err);
                return Error;
            })?;

            return Ok(VaultSecret {
                name: format!("{}: {}", parsed.issuer, parsed.account_name),
                secret: parsed.secret.into_boxed_slice(),
            });
        } else {
            eprintln!("Failed to detect the QR code of {:?}", path);
            return Err(Error);
        }
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

    pub fn list(&self, password: Option<&str>) -> Result<Vec<VaultSecret>, Error> {
        let key = DatabaseKey::new().with_password(password.unwrap_or(""));
        let database = Database::parse(self.content.as_slice(), key).map_err(|err| {
            eprintln!("Failed to read database, err: {}", err);
            return Error;
        })?;

        let mut results = Vec::new();
        for (idx, node) in database.root.iter().enumerate() {
            if let NodeRef::Entry(entry) = node {
                let title = entry.get_title().map(str::to_string);

                if let Some(url) = entry.get_url() {
                    let parsed = match otpauth::ParsedUrl::parse(&url) {
                        Ok(parsed) => parsed,
                        Err(err) => {
                            eprintln!(
                                "Failed to parse URL found in QR code of '{}', error: {:?}",
                                title.unwrap_or_else(|| format!("id:{}", idx)),
                                err,
                            );
                            continue;
                        }
                    };

                    let name = title.unwrap_or_else(|| format!("{}: {}", parsed.issuer, parsed.account_name));
                    results.push(VaultSecret {
                        name,
                        secret: parsed.secret.into_boxed_slice(),
                    });
                } else {
                    eprintln!(
                        "Skipping '{}', because the entry doesn't have an url",
                        title.unwrap_or_else(|| format!("id:{}", idx)),
                    );
                }
            }
        }

        return Ok(results);
    }
}
