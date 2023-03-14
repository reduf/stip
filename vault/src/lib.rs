#![allow(clippy::needless_return)]

use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use zip::result::InvalidPassword;

#[derive(Debug)]
pub struct Error;

pub fn open(path: &str, password: Option<&[u8]>) -> Result<Vec<u8>, Error> {
    let path = Path::new(path);

    match fs::read(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => (),
        Err(_) => return Err(Error),
        Ok(bytes) => return Ok(bytes),
    };

    let mut reader = None;

    let mut builder = PathBuf::from(path);
    while builder.pop() {
        match File::open(&builder) {
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(_) => return Err(Error),
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

        match result {
            Err(e) => {
                println!("Error reading file '{}', error: {}", suffix, e);
                return Err(Error);
            }
            Ok(Err(InvalidPassword)) => {
                println!("Invaid password when reading '{}'", suffix);
                return Err(Error);
            }
            Ok(Ok(mut file)) => {
                let mut buffer = Vec::with_capacity(file.size() as usize);
                file.read_to_end(&mut buffer).map_err(|_| Error)?;
                return Ok(buffer);
            }
        }
    } else {
        unreachable!();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    const FILE_CONTENT: &[u8] = b"file content";

    fn resource(suffix: &str) -> String {
        let mut result = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        result.push("tests");
        result.push("data");
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
