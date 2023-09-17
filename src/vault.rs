#![allow(clippy::needless_return)]

use promptly::{prompt, ReadlineError};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use zip::result::{InvalidPassword, ZipResult};
use zip::read::ZipFile;

use crate::{otpauth, stb_image};

#[derive(Debug)]
pub struct Error;

fn can_continue(err: io::Error) -> Result<(), Error> {
    #[cfg(windows)]
    if err.kind() == io::ErrorKind::NotFound {
        return Ok(());
    }

    #[cfg(unix)]
    if err.raw_os_error() == Some(20) {
        return Ok(());
    }

    return Err(Error);
}

fn prompt_in_range(label: &str, min: usize, max: usize) -> Result<usize, Error> {
    const MAX_NUMBER_OF_TRY: usize = 3;
    for _ in 0..MAX_NUMBER_OF_TRY {
        match prompt::<usize, _>(label) {
            Ok(result) => {
                if min <= result && result <= max {
                    return Ok(result);
                }
            }
            Err(ReadlineError::Interrupted) => return Err(Error),
            _ => println!("Try again"),
        };
    }

    return Err(Error);
}

pub fn result_from_zip_file(
    result: ZipResult<Result<ZipFile<'_>, InvalidPassword>>,
    filename: &str,
) -> Result<Vec<u8>, Error> {
    match result {
        Err(e) => {
            println!("Error reading file '{}', error: {}", filename, e);
            return Err(Error);
        }
        Ok(Err(InvalidPassword)) => {
            println!("Invaid password when reading '{}'", filename);
            return Err(Error);
        }
        Ok(Ok(mut file)) => {
            let mut buffer = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buffer).map_err(|_| Error)?;
            return Ok(buffer);
        }
    };
}

#[derive(Default)]
struct VaultFile {
    filename: String,
    file_number: usize,
    encrypted: bool,
}

pub fn interactive(path: &str, password: Option<&str>) -> Result<Vec<u8>, Error> {
    let path = Path::new(path);
    let reader = File::open(&path).map_err(|_| Error)?;

    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut names = Vec::new();
    for file_number in 0..zip.len() {
        if let Ok(file) = zip.by_index_raw(file_number) {
            let encrypted = file.encrypted();
            names.push(VaultFile {
                filename: file.name().to_string(),
                file_number,
                encrypted: encrypted,
            });
        } else {
            println!("Failed to read file #{}", file_number);
        }
    }

    if names.is_empty() {
        println!("No files could be found in '{:?}'", path);
        return Err(Error);
    }

    for (idx, entry) in names.iter().enumerate() {
        let encrypted_letter = if entry.encrypted { 'E' } else { ' ' };
        println!("{}: [{}] {}", idx + 1, encrypted_letter, entry.filename);
    }

    let idx = if 1 < names.len() {
        prompt_in_range("Select which file you want to use", 1, names.len())? - 1
    } else {
        0
    };

    let file_number = names[idx].file_number;
    let result = if names[idx].encrypted {
        if let Some(password) = password {
            zip.by_index_decrypt(file_number, password.as_bytes())
        } else {
            let password = rpassword::prompt_password("Enter password: ")
                .expect("Failed to read user password");
            zip.by_index_decrypt(file_number, password.as_bytes())
        }
    } else {
        zip.by_index(file_number).map(Ok)
    };

    let bytes = result_from_zip_file(result, &names[idx].filename)?;
    return Ok(bytes);
}

pub fn open(path: &str, password: Option<&[u8]>) -> Result<Vec<u8>, Error> {
    let path = Path::new(path);

    match fs::read(path) {
        Err(err) => can_continue(err)?,
        Ok(bytes) => return Ok(bytes),
    };

    let mut reader = None;

    let mut builder = PathBuf::from(path);
    while builder.pop() {
        match File::open(&builder) {
            Err(err) => can_continue(err)?,
            Ok(file) => {
                reader = Some(file);
                break;
            }
        }
    }

    let reader = reader.ok_or(Error)?;
    if let Ok(suffix) = path.strip_prefix(&builder) {
        let suffix = suffix.to_str().ok_or(Error)?;
        let suffix = suffix.replace('\\', "/");

        let mut zip = zip::ZipArchive::new(reader).unwrap();
        let result = if let Some(password) = password {
            zip.by_name_decrypt(&suffix, password)
        } else {
            zip.by_name(&suffix).map(Ok)
        };

        return result_from_zip_file(result, &suffix);
    } else {
        unreachable!();
    }
}

pub struct VaultSecret {
    pub filename: String,
    pub secret: Option<Box<[u8]>>,
    encrypted: bool,
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
        return Ok(());
    } else {
        eprintln!("Failed to detect the QR code of '{}'", output.filename);
        return Err(Error);
    }
}

pub fn list(path: &str, password: Option<&str>) -> Result<Vec<VaultSecret>, Error> {
    let path = Path::new(path);
    let reader = File::open(&path).map_err(|_| Error)?;

    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut results = Vec::new();
    for file_number in 0..zip.len() {
        let mut new_secret = if let Ok(file) = zip.by_index_raw(file_number) {
            let encrypted = file.encrypted();
            VaultSecret {
                filename: file.name().to_string(),
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
                eprintln!("Invaid password when reading '{}'", new_secret.filename);
                results.push(new_secret);
                continue;
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
