#![allow(clippy::needless_return)]

use promptly::{prompt, ReadlineError};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use zip::result::{InvalidPassword, ZipResult};
use zip::read::ZipFile;

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

fn result_from_zip_file(
    result: ZipResult<Result<ZipFile<'_>, InvalidPassword>>,
    file_name: &str,
) -> Result<Vec<u8>, Error> {
    match result {
        Err(e) => {
            println!("Error reading file '{}', error: {}", file_name, e);
            return Err(Error);
        }
        Ok(Err(InvalidPassword)) => {
            println!("Invaid password when reading '{}'", file_name);
            return Err(Error);
        }
        Ok(Ok(mut file)) => {
            let mut buffer = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buffer).map_err(|_| Error)?;
            return Ok(buffer);
        }
    };
}

pub fn interactive(path: &str, password: Option<&[u8]>) -> Result<(Vec<u8>, String), Error> {
    let path = Path::new(path);
    let reader = File::open(&path).map_err(|_| Error)?;

    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let mut names = Vec::new();
    for file_number in 0..zip.len() {
        if let Ok(file) = zip.by_index_raw(file_number) {
            names.push((file.name().to_string(), file_number));
        } else {
            println!("Failed to read file #{}", file_number);
        }
    }

    if names.is_empty() {
        println!("No files could be found in '{:?}'", path);
        return Err(Error);
    }

    for (idx, (file_name, _)) in names.iter().enumerate() {
        println!("{}: {}", idx + 1, file_name);
    }

    let idx = if 1 < names.len() {
        prompt_in_range("Select which file you want to use", 1, names.len())? - 1
    } else {
        0
    };

    let file_number = names[idx].1;
    let result = if let Some(password) = password {
        zip.by_index_decrypt(file_number, password)
    } else {
        zip.by_index(file_number).map(Ok)
    };

    let bytes = result_from_zip_file(result, &names[idx].0)?;
    return Ok((bytes, std::mem::take(&mut names[idx]).0));
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
