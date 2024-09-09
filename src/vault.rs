#![allow(clippy::needless_return)]

use std::path::{PathBuf, Path};
use std::collections::HashMap;
use keepass::{
    db::NodeRef,
    Database,
    DatabaseKey,
};
use uuid::Uuid;

use crate::{otpauth, stb_image::{self, Channel, Image}};

#[derive(Debug)]
pub struct Error;

pub struct Vault {
    pub path: PathBuf,
    pub database: Database,
    pub custom_icons: Vec<Image>,
    custom_icons_idx: HashMap<Uuid, usize>,
}

pub struct VaultSecret {
    pub name: String,
    parsed_url: otpauth::ParsedUrl,
    pub icon: Option<usize>,
}

impl VaultSecret {
    pub fn from_path(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read(path).map_err(|err| {
            eprintln!("Failed to open {:?}, err: {}", path, err);
            return Error;
        })?;

        let img = stb_image::load_from_memory(content.as_slice(), Channel::Grey).map_err(|err| {
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

            let parsed_url = otpauth::ParsedUrl::parse(content).map_err(|err| {
                eprintln!("Failed to parse URL found in QR code of {:?}, error: {:?}", path, err);
                return Error;
            })?;

            return Ok(VaultSecret {
                name: format!("{}: {}", parsed_url.issuer, parsed_url.account_name),
                parsed_url: parsed_url,
                icon: None,
            });
        } else {
            eprintln!("Failed to detect the QR code of {:?}", path);
            return Err(Error);
        }
    }

    pub fn url(&self) -> &str {
        return self.parsed_url.raw.as_ref();
    }

    pub fn secret(&self) -> &[u8] {
        return self.parsed_url.secret.as_slice();
    }

    pub fn period(&self) -> u64 {
        return self.parsed_url.period;
    }

    pub fn digits(&self) -> usize {
        return self.parsed_url.digits;
    }
}

impl Vault {
    pub fn open(path: PathBuf, password: &str) -> Result<Self, Error> {
        let content = std::fs::read(path.as_path()).map_err(|err| {
            eprintln!("Failed to open {:?}, err: {}", path.as_path(), err);
            return Error;
        })?;

        let key = DatabaseKey::new().with_password(password);
        let database = Database::parse(content.as_slice(), key).map_err(|err| {
            eprintln!("Failed to read database, err: {}", err);
            return Error;
        })?;

        let mut custom_icons: Vec<Image> = Vec::new();
        let mut custom_icons_idx: HashMap<Uuid, usize> = HashMap::new();
        for (idx, icon) in database.meta.custom_icons.icons.iter().enumerate() {
            if let Ok(img) = stb_image::load_from_memory(icon.data.as_slice(), Channel::Rgba) {
                custom_icons.push(img);
                custom_icons_idx.insert(icon.uuid, custom_icons.len() - 1);
            } else {
                eprintln!("Failed to load custom icon {}", idx);
            }
        }

        return Ok(Vault {
            path,
            database,
            custom_icons,
            custom_icons_idx,
        });
    }

    pub fn secrets(&self) -> Vec<VaultSecret> {
        let mut secrets = Vec::new();
        for (idx, node) in self.database.root.iter().enumerate() {
            if let NodeRef::Entry(entry) = node {
                let title = entry.get_title().map(str::to_string).unwrap_or_else(|| format!("entry:{}", idx));

                let custom_icon_idx = entry
                    .custom_icon_uuid
                    .map(|uuid| self.custom_icons_idx.get(&uuid).cloned())
                    .flatten();

                if let Some(url) = entry.get_url() {
                    let parsed_url = match otpauth::ParsedUrl::parse(url.to_string()) {
                        Ok(parsed_url) => parsed_url,
                        Err(err) => {
                            eprintln!(
                                "Failed to parse URL found in QR code of '{}', error: {:?}",
                                title,
                                err,
                            );
                            continue;
                        }
                    };

                    secrets.push(VaultSecret {
                        name: title,
                        parsed_url: parsed_url,
                        icon: custom_icon_idx,
                    });
                } else {
                    eprintln!("Skipping '{}', because the entry doesn't have an url", title);
                }
            }
        }

        return secrets;
    }
}
